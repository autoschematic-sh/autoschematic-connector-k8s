use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum K8sResource {
    // Namespace(k8s_openapi::api::core::v1::Namespace)
}

// impl autoschematic_core::connector::Resource for K8sResource {
//     fn to_string(&self) -> Result<String, anyhow::Error> {
//         Ok(serde_yaml::to_string(&self.0)?)
//     }

//     fn from_str(_addr: &impl autoschematic_core::connector::ResourceAddress, s: &str) -> Result<Self, anyhow::Error> where Self: Sized {
//         Ok(Self(serde_yaml::from_str(s)?))
//     }
// }