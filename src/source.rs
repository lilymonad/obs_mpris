use crate::{
    global::{self, TrackMetadata},
    properties::{add_common_properties, add_list_of_text_props},
};
use obs_sys::obs_get_source_by_name;
use obs_wrapper::{
    obs_string,
    prelude::DataObj,
    properties::Properties,
    source::{
        CreatableSourceContext, GetNameSource, GetPropertiesSource, GlobalContext, SourceContext,
        SourceType, Sourceable, UpdateSource, VideoTickSource,
    },
    string::ObsString,
};

/// A source you'll need to add to your scene
/// in order to command the thread and write something on it
pub struct MprisSource {
    context: SourceContext,
    text_source: Option<SourceContext>,
    traced_player_id: String,
    next_update: f32,
}

impl MprisSource {
    fn update_from_data(&mut self, data: &DataObj) {
        // SAFETY: it's ok because we always get the SourceContext pointer from obs dedicated
        // function
        self.text_source = unsafe {
            data.get("text_source").and_then(|source_name: ObsString| {
                let ptr = obs_get_source_by_name(source_name.as_ptr());
                if ptr.is_null() {
                    None
                } else {
                    Some(SourceContext::from_raw(ptr))
                }
            })
        };

        let _ = global::get()
            .template_engine
            .lock()
            .unwrap()
            .register_template_string(
                self.context.name().unwrap(),
                data.get::<ObsString>("template")
                    .as_ref()
                    .map(ObsString::as_str)
                    .unwrap_or(""),
            );

        self.traced_player_id = data
            .get::<ObsString>("mpris_device")
            .map(|s| s.as_str().to_string())
            .unwrap_or_default();
    }
}

/// Implement Sourceable which allow us to register MprisSource as an OBS source
impl Sourceable for MprisSource {
    fn get_id() -> ObsString {
        obs_string!("obs_mpris_source")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }

    fn create(create: &mut CreatableSourceContext<Self>, context: SourceContext) -> Self {
        let mut ret = MprisSource {
            traced_player_id: "".to_string(),
            context,
            text_source: None,
            next_update: 0.0,
        };
        ret.update_from_data(&create.settings);
        ret
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

        let default_env = TrackMetadata::default();
        // update text source text
        if let Some(source) = self.text_source.as_mut() {
            // get the player metadata from global context
            let text = global::get()
                .template_engine
                .lock()
                .unwrap()
                .render(
                    self.context.name().unwrap(),
                    global::get()
                        .track_metadata
                        .lock()
                        .ok()
                        .as_ref()
                        .and_then(|lock| lock.get(&self.traced_player_id))
                        .unwrap_or(&default_env),
                )
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
        self.update_from_data(settings)
    }
}

/// Setup the property list of the source (what the user can edit when changing the source
/// properties)
impl GetPropertiesSource for MprisSource {
    fn get_properties(&mut self) -> obs_wrapper::properties::Properties {
        let mut props = Properties::new();
        add_list_of_text_props(&mut props);
        add_common_properties(&mut props);
        props
    }
}
