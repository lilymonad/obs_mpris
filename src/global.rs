use handlebars::{no_escape, Handlebars};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

#[derive(Serialize)]
pub(crate) struct TrackMetadata {
    pub(crate) title: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) artists: Option<Vec<String>>,
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            title: None,
            album: None,
            artists: None,
        }
    }
}

/// Global static context of the plugin
pub struct GlobalCtx<'a> {
    /// The players we want to monitor per source
    pub(crate) players_list: Mutex<Vec<String>>,
    /// The track id for each player (for now it's the title, but it should be the whole metadata)
    pub(crate) track_metadata: Mutex<HashMap<String, TrackMetadata>>,
    /// The template engine
    pub(crate) template_engine: Mutex<Handlebars<'a>>,
}

/// Wrap the contxt into a LazyLock to create it dynamically (we cannot do differently because the
/// structure uses Mutex
static GLOBAL_CTX: LazyLock<GlobalCtx<'static>> = LazyLock::new(self::init);

pub fn init<'a>() -> GlobalCtx<'a> {
    let mut template_engine = Handlebars::new();
    template_engine.register_escape_fn(no_escape);

    GlobalCtx {
        players_list: Mutex::new(Vec::with_capacity(16)),
        template_engine: Mutex::new(template_engine),
        track_metadata: Mutex::new(HashMap::new()),
    }
}

pub fn get() -> &'static GlobalCtx<'static> {
    &*GLOBAL_CTX
}
