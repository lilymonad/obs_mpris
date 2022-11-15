use obs_sys::obs_filter_get_parent;
use obs_wrapper::{
    graphics::{GraphicsAllowDirectRendering, GraphicsColorFormat, GraphicsEffect},
    obs_string,
    prelude::DataObj,
    properties::Properties,
    source::{
        CreatableSourceContext, GetNameSource, GetPropertiesSource, SourceContext, SourceType,
        Sourceable, UpdateSource, VideoRenderSource, VideoTickSource,
    },
    string::ObsString,
};
use serde_json::json;

use crate::{
    global::{self, TrackMetadata},
    properties::add_common_properties,
};

pub struct MprisTextFilter {
    context: SourceContext,
    traced_player_id: String,
    parent: Option<SourceContext>,
    update_timer: f32,
}

impl MprisTextFilter {
    fn update_from_data(&mut self, data: &DataObj) {
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
            .as_ref()
            .map(ObsString::as_str)
            .unwrap_or("")
            .to_string();
    }
}

impl Sourceable for MprisTextFilter {
    fn get_id() -> ObsString {
        obs_string!("obs_mpris_text_filter")
    }

    fn get_type() -> SourceType {
        SourceType::FILTER
    }

    fn create(create: &mut CreatableSourceContext<Self>, context: SourceContext) -> Self {
        let mut ret = Self {
            traced_player_id: "".to_string(),
            context,
            parent: None,
            update_timer: 0.0,
        };
        ret.update_from_data(&create.settings);
        ret
    }
}

impl VideoTickSource for MprisTextFilter {
    fn video_tick(&mut self, seconds: f32) {
        self.update_timer -= seconds;
    }
}

impl GetNameSource for MprisTextFilter {
    fn get_name() -> ObsString {
        obs_string!("Mpris Text Info")
    }
}

impl VideoRenderSource for MprisTextFilter {
    fn video_render(
        &mut self,
        _context: &mut obs_wrapper::source::GlobalContext,
        render: &mut obs_wrapper::source::VideoRenderContext,
    ) {
        if self.update_timer < 0.0 {
            let parent = self.parent.get_or_insert_with(|| unsafe {
                let ptr = std::mem::transmute(self.context.clone());
                let parent_ptr = obs_filter_get_parent(ptr);
                SourceContext::from_raw(parent_ptr)
            });

            let default_env: TrackMetadata = Default::default();
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

            parent.update_source_settings(
                &mut DataObj::from_json(json!({ "text": text }).to_string()).unwrap(),
            );

            self.update_timer = 1.0;
        }

        let Some(mut effect) = GraphicsEffect::from_effect_string(
            obs_string!(include_str!("nothing.effect")),
            obs_string!("nothing.effect"),
        ) else {
            log::error!("effect could not be compiled");
            return
        };

        self.context.process_filter(
            render,
            &mut effect,
            (0, 0),
            GraphicsColorFormat::RGBA,
            GraphicsAllowDirectRendering::NoDirectRendering,
            |_context, _effect| {},
        )
    }
}

impl GetPropertiesSource for MprisTextFilter {
    fn get_properties(&mut self) -> obs_wrapper::properties::Properties {
        let mut props = Properties::new();
        add_common_properties(&mut props);
        props
    }
}

impl UpdateSource for MprisTextFilter {
    fn update(
        &mut self,
        settings: &mut DataObj,
        _context: &mut obs_wrapper::source::GlobalContext,
    ) {
        self.update_from_data(settings);
    }
}
