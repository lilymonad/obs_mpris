use std::ffi::c_void;

use obs_sys::{obs_enum_sources, obs_source_t};
use obs_wrapper::{
    obs_string,
    properties::{ListProp, Properties, TextProp, TextType},
    source::SourceContext,
    string::ObsString,
};

pub fn mpris_info_source_properties() -> Properties {
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
