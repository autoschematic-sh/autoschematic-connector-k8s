use autoschematic_core::{
    connector::GetResourceResponse,
    util::{PrettyConfig, RON},
};
use kube::api::ObjectMeta;
use ron::de;
use serde::{Deserialize, Serialize};

use crate::{connector::SerdeBackend, neat::neatify_resource};

pub fn strip_boring_fields(meta: &mut ObjectMeta) {
    meta.creation_timestamp = None;
    meta.managed_fields = None;
    meta.resource_version = None;
    meta.uid = None;
}

pub const SERDE: SerdeBackend = SerdeBackend::YAML;

pub fn from_str_option<'a, T>(s: &'a Option<Vec<u8>>) -> anyhow::Result<Option<T>>
where
    T: serde::Deserialize<'a>,
{
    match &s {
        Some(s) => {
            let s = str::from_utf8(s)?;
            Ok(Some(SERDE.from_str(s)?))
        }
        None => Ok(None),
    }
}

pub fn get_ser_resource_output<T: Serialize>(t: &T) -> anyhow::Result<Option<GetResourceResponse>> {

    let mut v = serde_yaml::to_value(t)?;
    neatify_resource(&mut v);

    Ok(Some(GetResourceResponse {
        resource_definition: SERDE.to_string(&v)?.into_bytes(),
        outputs: None,
    }))
}
