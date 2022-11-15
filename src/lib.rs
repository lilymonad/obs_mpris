#![feature(once_cell, thread_spawn_unchecked, exclusive_wrapper)]
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
    time::Duration,
};

use filter::MprisTextFilter;
use global::TrackMetadata;
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
    thread_running: AtomicBool,
}

/// Implementing Module allow us to register MprisModule as an OBS module
impl Module for MprisModule {
    fn new(context: ModuleContext) -> Self {
        Self {
            context,
            mpris_thread: None,
            thread_running: AtomicBool::new(true),
        }
    }

    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let _ = Logger::new().init();

        // SAFETY: We can relax the thread lifetime because we join() it before destroying the
        // module (which contains the thread handle)
        let mpris_thread = unsafe {
            thread::Builder::new().spawn_unchecked(|| {
                let mpris_player_finder = PlayerFinder::new().unwrap();
                let mut player_set = HashMap::new();

                while self.thread_running.load(Ordering::Relaxed) {
                    // get player list from dbus
                    let Ok(players) = mpris_player_finder.find_all() else {
                        log::error!("Failed to get MPRIS player list");
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    };

                    // update local map, it should be without clear() because we drain it in the
                    // end
                    player_set.extend(players.into_iter().map(|p| (p.bus_name().to_string(), p)));

                    // update global player name set
                    {
                        let mut lock = global::get().players_list.lock().unwrap();
                        lock.clear();
                        lock.extend(player_set.keys().cloned());
                    }

                    // for each player, store informations in global memory
                    for (name, mut player) in player_set.drain() {
                        player.set_dbus_timeout_ms(10 * 1000);
                        let Ok(meta) = player.get_metadata() else { log::warn!("player {name} timed out"); continue };
                        {
                            let mut lock = global::get().track_metadata.lock().unwrap();

                            let entry = lock.entry(name).or_insert(TrackMetadata::default());
                            entry.title = meta.title().map(ToString::to_string);
                            entry.album = meta.album_name().map(ToString::to_string);
                            entry.artists = meta.artists().cloned();
                        }
                    }
                }
            })
        }
        .unwrap();

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
        self.thread_running.store(false, Ordering::Relaxed);
        let _ = self.mpris_thread.take().unwrap().join();
    }
}

// register MprisModule (setup the functions like obs_module_load for obs to understand the plugin)
obs_register_module!(MprisModule);
