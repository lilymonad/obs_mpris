use handlebars::{no_escape, Handlebars};
use obs_wrapper::prelude::ObsString;
use serde::Serialize;
use std::sync::{atomic::AtomicBool, LazyLock, Mutex};

#[derive(Serialize)]
pub(crate) struct TrackMetadata {
    pub(crate) title: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) artists: Option<Vec<String>>,
}

/// Global static context of the plugin
pub struct GlobalCtx<'a> {
    /// The player we want to monitor
    pub(crate) mpris_player: Mutex<Option<ObsString>>,
    /// The track id (for now it's the title, but it should be the whole metadata)
    pub(crate) track_metadata: Mutex<TrackMetadata>,
    /// The template engine
    pub(crate) template_engine: Mutex<Handlebars<'a>>,
    pub(crate) running: AtomicBool,
}

/// Wrap the contxt into a LazyLock to create it dynamically (we cannot do differently because the
/// structure uses Mutex
pub static GLOBAL_CTX: LazyLock<GlobalCtx<'static>> = LazyLock::new(|| {
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
