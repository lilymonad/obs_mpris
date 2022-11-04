#![feature(once_cell)]
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use handlebars::{no_escape, Handlebars};
use mpris::PlayerFinder;
use obs_sys::{obs_enum_sources, obs_get_source_by_name, obs_source_t};
use obs_wrapper::{
    log::Logger,
    obs_register_module, obs_string,
    prelude::{DataObj, LoadContext, Module, ModuleContext},
    properties::{ListProp, Properties, TextProp, TextType},
    source::{
        CreatableSourceContext, GetNameSource, GetPropertiesSource, GlobalContext, SourceContext,
        SourceType, Sourceable, UpdateSource, VideoTickSource,
    },
    string::ObsString,
};
use serde::Serialize;

/// The module loaded by OBS
struct MprisModule {
    context: ModuleContext,
    /// The thread launched when the module is loaded.
    /// We need it to make MPRIS calls non-blocking
    mpris_thread: Option<JoinHandle<()>>,
}

/// A source you'll need to add to your scene
/// in order to command the thread and write something on it
struct MprisSource {
    text_source: Option<ObsString>,
    next_update: f32,
}

#[derive(Serialize)]
struct TrackMetadata {
    title: Option<String>,
    album: Option<String>,
    artists: Option<Vec<String>>,
}

/// Global static context of the plugin
struct GlobalCtx<'a> {
    /// The player we want to monitor
    mpris_player: Mutex<Option<ObsString>>,
    /// The track id (for now it's the title, but it should be the whole metadata)
    track_metadata: Mutex<TrackMetadata>,
    /// The template engine
    template_engine: Mutex<Handlebars<'a>>,
    running: AtomicBool,
}

/// Wrap the contxt into a LazyLock to create it dynamically (we cannot do differently because the
/// structure uses Mutex
static GLOBAL_CTX: LazyLock<GlobalCtx<'static>> = LazyLock::new(|| {
    let mut template_engine = Handlebars::new();
    template_engine.register_escape_fn(no_escape);
    GlobalCtx {
        template_engine: Mutex::new(template_engine),
        mpris_player: Mutex::new(None),
        track_metadata: Mutex::new(TrackMetadata {
            title: None,
            album: None,
            artists: None,
        }),
        running: AtomicBool::from(true),
    }
});

/// Implement Sourceable which allow us to register MprisSource as an OBS source
impl Sourceable for MprisSource {
    fn get_id() -> ObsString {
        obs_string!("obs_mpris")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }

    fn create(create: &mut CreatableSourceContext<Self>, _context: SourceContext) -> Self {
        *GLOBAL_CTX.mpris_player.lock().unwrap() = create.settings.get("mpris_device");
        let _ = GLOBAL_CTX
            .template_engine
            .lock()
            .unwrap()
            .register_template_string(
                "template",
                create
                    .settings
                    .get::<ObsString>("template")
                    .as_ref()
                    .map(ObsString::as_str)
                    .unwrap_or(""),
            );
        Self {
            text_source: create.settings.get("text_source"),
            next_update: 0.0,
        }
    }
}

impl GetNameSource for MprisSource {
    fn get_name() -> ObsString {
        obs_string!("MPRIS")
    }
}

// if one day i want to add a graphical feature like showing the album i'll need to implement those
// traits
//
//impl GetWidthSource for MprisSource {
//    fn get_width(&mut self) -> u32 {
//        self.width
//    }
//}
//
//impl GetHeightSource for MprisSource {
//    fn get_height(&mut self) -> u32 {
//        self.height
//    }
//}
//
//impl VideoRenderSource for MprisSource {
//    fn video_render(&mut self, context: &mut GlobalContext, render: &mut VideoRenderContext) {
//        log::info!("mpris rendering");
//        let w = 64; //self.context.width();
//        let h = 64; //self.context.height();
//
//        let mut texture =
//            GraphicsTexture::new(w, h, obs_wrapper::graphics::GraphicsColorFormat::RGBA);
//        let pixels = vec![0xff; (w * h * 4) as usize];
//        texture.set_image(&pixels, w * 4, true);
//        texture.draw(0, 0, w, h, false);
//    }
//}

/// We implement VideoTickSource to update shown information periodicaly
impl VideoTickSource for MprisSource {
    fn video_tick(&mut self, seconds: f32) {
        // update the next_update deadline counter
        // and make an update when asked
        self.next_update -= seconds;
        if self.next_update > 0.0 {
            return;
        }

        // update text source text
        if let Some(source_name) = self.text_source.as_ref() {
            // SAFETY: it's ok because we always get the SourceContext pointer from obs dedicated
            // function
            let mut source = unsafe {
                let ptr = obs_get_source_by_name(source_name.as_ptr());
                if ptr.is_null() {
                    return;
                }
                SourceContext::from_raw(ptr)
            };

            // get the player metadata from global context
            let text = GLOBAL_CTX
                .template_engine
                .lock()
                .unwrap()
                .render("template", &*GLOBAL_CTX.track_metadata.lock().unwrap())
                .unwrap_or_else(|e| e.to_string());

            // set the text
            source.update_source_settings(
                &mut DataObj::from_json(serde_json::json!({ "text": text }).to_string()).unwrap(),
            );
        }

        // reset update timer
        self.next_update = 1.0;
    }
}

/// Implementation of update callback (called when user changed source properties)
impl UpdateSource for MprisSource {
    fn update(
        &mut self,
        settings: &mut obs_wrapper::prelude::DataObj,
        _context: &mut GlobalContext,
    ) {
        if let Some(src_name) = settings.get::<ObsString>("text_source") {
            self.text_source = Some(src_name);
        }

        let _ = GLOBAL_CTX
            .template_engine
            .lock()
            .unwrap()
            .register_template_string(
                "template",
                settings
                    .get::<ObsString>("template")
                    .as_ref()
                    .map(ObsString::as_str)
                    .unwrap_or(""),
            );

        *GLOBAL_CTX.mpris_player.lock().unwrap() = settings.get("mpris_device");
    }
}

/// Setup the property list of the source (what the user can edit when changing the source
/// properties)
impl GetPropertiesSource for MprisSource {
    fn get_properties(&mut self) -> obs_wrapper::properties::Properties {
        let mut props = Properties::new();
        let mut list = props.add_list(
            obs_string!("text_source"),
            obs_string!("The text source to write to"),
            false,
        );

        // Helper function to fill a ListProp (represented by the data parameter) from an obs_source_t
        // given by the funtion "obs_enum_sources"
        unsafe extern "C" fn fill_property_list(data: *mut c_void, src: *mut obs_source_t) -> bool {
            let src = SourceContext::from_raw(src);
            let list = (data as *mut ListProp<ObsString>).as_mut().unwrap();
            let name: ObsString = src.name().unwrap().into();
            list.push(name.clone(), name);
            true
        }

        // SAFETY: this is safe because list has the good type (the callback casts it back to a
        // ListProp<ObsString>) and outlives this call
        unsafe {
            obs_enum_sources(
                Some(fill_property_list),
                (&mut list) as *mut ListProp<ObsString> as *mut c_void,
            );
        }

        props.add(
            obs_string!("mpris_device"),
            obs_string!("The MPRIS player to monitor"),
            TextProp::new(TextType::Default),
        );

        props.add(
            obs_string!("template"),
            obs_string!(
                "The text template to show.\n\
                Use {{variable}} to show a variable.\n\
                Available variables are:\n\
                {{title}}, {{album}}, {{artists}}"
            ),
            TextProp::new(TextType::Multiline),
        );

        props
    }
}

/// Implementing Module allow us to register MprisModule as an OBS module
impl Module for MprisModule {
    fn new(context: ModuleContext) -> Self {
        Self {
            context,
            mpris_thread: None,
        }
    }

    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let _ = Logger::new().init();

        let mpris_thread = thread::spawn(|| {
            let mpris_player_finder = PlayerFinder::new().unwrap();
            let mut current_player = None;

            while GLOBAL_CTX.running.load(Ordering::Relaxed) {
                let mpris_player = GLOBAL_CTX.mpris_player.lock().unwrap().take();

                if let Some(player) = mpris_player {
                    let players = match mpris_player_finder.find_all() {
                        Ok(players) => players,
                        Err(e) => {
                            log::error!("Failed to get MPRIS player list {e}");
                            continue;
                        }
                    };
                    for p in players {
                        let name = p.identity();
                        if name == player.as_str() {
                            current_player = Some(p);
                            break;
                        }
                    }
                }

                if let Some(meta) = current_player.as_ref().and_then(|p| p.get_metadata().ok()) {
                    {
                        let mut metadata = GLOBAL_CTX.track_metadata.lock().unwrap();
                        metadata.title = meta.title().map(Into::into);
                        metadata.album = meta.album_name().map(Into::into);
                        metadata.artists = meta.artists().cloned();
                    }

                    thread::sleep(Duration::from_secs(5));
                }
            }
        });

        self.mpris_thread = Some(mpris_thread);

        let source = load_context
            .create_source_builder::<MprisSource>()
            .enable_get_name()
            .enable_update()
            .enable_get_properties()
            .enable_video_tick()
            .build();

        load_context.register_source(source);
        true
    }

    fn description() -> ObsString {
        obs_string!("This module gives a source which can show information about MPRIS enabled media player")
    }

    fn name() -> ObsString {
        obs_string!("obs_mpris")
    }

    fn author() -> ObsString {
        obs_string!("lilymonade")
    }

    fn unload(&mut self) {
        GLOBAL_CTX.running.store(false, Ordering::Relaxed);
        let _ = self.mpris_thread.take().unwrap().join();
    }
}

// register MprisModule (setup the functions like obs_module_load for obs to understand the plugin)
obs_register_module!(MprisModule);
