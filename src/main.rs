use std::path::{Path, PathBuf};

use async_trait::async_trait;
use autoschematic_core::{
    connector::{Connector, ConnectorOutbox, GetResourceOutput, OpExecOutput, OpPlanOutput},
    tarpc_bridge::{init_server, tarpc_connector_main},
};
use connector::K8sConnector;

pub mod addr;
mod connector;
mod resource;
mod op;
mod op_impl;
mod util;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    tarpc_connector_main::<K8sConnector>().await?;
    Ok(())
}
