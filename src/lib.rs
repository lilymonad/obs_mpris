#![feature(once_cell)]
use std::{
    sync::atomic::Ordering,
    thread::{self, JoinHandle},
    time::Duration,
};

use filter::MprisTextFilter;
use global::GLOBAL_CTX;
use mpris::PlayerFinder;
use obs_wrapper::{
    log::Logger,
    obs_register_module, obs_string,
    prelude::{LoadContext, Module, ModuleContext},
    string::ObsString,
};

mod filter;
mod global;
mod properties;
mod source;

/// The module loaded by OBS
struct MprisModule {
    context: ModuleContext,
    /// The thread launched when the module is loaded.
    /// We need it to make MPRIS calls non-blocking
    mpris_thread: Option<JoinHandle<()>>,
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
            .create_source_builder::<source::MprisSource>()
            .enable_get_name()
            .enable_update()
            .enable_get_properties()
            .enable_video_tick()
            .build();

        let filter = load_context
            .create_source_builder::<MprisTextFilter>()
            .enable_get_name()
            .enable_update()
            .enable_get_properties()
            .enable_video_tick()
            .enable_video_render()
            .build();

        load_context.register_source(source);
        load_context.register_source(filter);

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
