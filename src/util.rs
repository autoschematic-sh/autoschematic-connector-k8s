use autoschematic_core::{
    connector::GetResourceOutput,
    util::{PrettyConfig, RON},
};
use kube::api::ObjectMeta;
use ron::de;
use serde::{Deserialize, Serialize};

pub fn strip_boring_fields(meta: &mut ObjectMeta) {
    meta.creation_timestamp = None;
    meta.managed_fields = None;
    meta.resource_version = None;
    meta.uid = None;
}

pub fn from_str_option<'a, T>(s: &'a Option<Vec<u8>>) -> anyhow::Result<Option<T>>
where
    T: serde::Deserialize<'a>,
{
    match &s {
        Some(s) => {
            let s = str::from_utf8(s)?;
            Ok(Some(RON.from_str(s)?))
        }
        None => Ok(None),
    }
}

pub fn get_ser_resource_output<T: Serialize>(t: &T) -> anyhow::Result<Option<GetResourceOutput>> {
    Ok(Some(GetResourceOutput {
        resource_definition: RON.to_string_pretty(t, PrettyConfig::default())?.into(),
        outputs: None,
    }))
}
