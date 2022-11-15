use std::ffi::{c_void, CStr, CString};

use obs_sys::{obs_enum_sources, obs_source_t};
use obs_wrapper::{
    obs_string,
    properties::{ListProp, Properties, TextProp, TextType},
    source::SourceContext,
    string::ObsString,
};

use crate::global;

pub fn add_list_of_text_props(props: &mut Properties) {
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
}

pub fn add_common_properties(props: &mut Properties) {
    let mut list: ListProp<ObsString> = props.add_list(
        obs_string!("mpris_device"),
        obs_string!("The MPRIS player to monitor"),
        false,
    );

    {
        let player_names = global::get().players_list.lock().unwrap();
        for name in player_names.iter() {
            let mut name_nt = name.clone();
            name_nt.push('\0');

            // SAFETY: safe because we just pushed a \0 at the end of name_nt
            let name_cstr = unsafe { CStr::from_bytes_with_nul_unchecked(name_nt.as_bytes()) };
            let name_cstring = CString::from(name_cstr);
            let name_obsstring = ObsString::from(name_cstring);

            list.push(name_obsstring.clone(), name_obsstring);
        }
    }

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
}
