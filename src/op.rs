use autoschematic_core::{connector::ConnectorOp, util::{PrettyConfig, RON}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum K8sConnectorOp {
    Create(String),
    Patch(String),
    Delete,
}

impl ConnectorOp for K8sConnectorOp {
    fn to_string(&self) -> Result<String, anyhow::Error> {
        Ok(RON.to_string_pretty(&self, PrettyConfig::default())?)
    }

    fn from_str(s: &str) -> Result<Self, anyhow::Error>
    where
        Self: Sized,
    {
        Ok(RON.from_str(s)?)
    }
}
