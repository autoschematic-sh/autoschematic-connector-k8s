use std::{path::{Path, PathBuf}, sync::Arc};

use async_trait::async_trait;
use autoschematic_core::{
    connector::{Connector, ConnectorOutbox, FilterOutput, GetResourceOutput, OpExecOutput, OpPlanOutput, ResourceAddress},
    diag::DiagnosticOutput,
    error::{AutoschematicError, AutoschematicErrorType},
    get_resource_output,
    tarpc_bridge::TarpcConnector,
    util::{PrettyConfig, RON, ron_check_eq, ron_check_syntax},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
};
use kube::Client;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{addr::K8sResourceAddress, util::strip_boring_fields};

mod get;
mod list;
mod op_exec;
mod plan;

pub enum SerMode {
    RON,
    YAML,
}

pub struct K8sConnector {
    // outbox: ConnectorOutbox,
    // pub name: String,
    client: Mutex<Option<Client>>,
    prefix: PathBuf,
}

#[async_trait]
impl Connector for K8sConnector {
    async fn new(name: &str, prefix: &Path, outbox: ConnectorOutbox) -> Result<Arc<dyn Connector>, anyhow::Error>
    where
        Self: Sized,
    {
        if name != "k8s" {
            return Err(AutoschematicError {
                kind: AutoschematicErrorType::InvalidConnectorString(name.to_string()),
            }
            .into());
        }

        Ok(Arc::new(K8sConnector {
            client: Mutex::new(None),
            prefix: prefix.into(),
        }))
    }

    async fn init(&self) -> anyhow::Result<()> {
        *self.client.lock().await = Some(Client::try_default().await?);
        Ok(())
    }

    async fn filter(&self, addr: &Path) -> Result<FilterOutput, anyhow::Error> {
        if let Ok(_) = K8sResourceAddress::from_path(addr) {
            Ok(FilterOutput::Resource)
        } else {
            Ok(FilterOutput::None)
        }
    }

    async fn list(&self, subpath: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        self.do_list(subpath).await
    }

    async fn get(&self, addr: &Path) -> Result<Option<GetResourceOutput>, anyhow::Error> {
        self.do_get(addr).await
    }

    async fn plan(
        &self,
        addr: &Path,
        current: Option<Vec<u8>>,
        desired: Option<Vec<u8>>,
    ) -> Result<Vec<OpPlanOutput>, anyhow::Error> {
        self.do_plan(addr, current, desired).await
    }

    async fn op_exec(&self, addr: &Path, op: &str) -> Result<OpExecOutput, anyhow::Error> {
        self.do_op_exec(addr, op).await
    }

    async fn eq(&self, addr: &Path, a: &[u8], b: &[u8]) -> Result<bool, anyhow::Error> {
        let addr = K8sResourceAddress::from_path(addr)?;

        match addr {
            K8sResourceAddress::Namespace(_) => ron_check_eq::<Namespace>(a, b),
            K8sResourceAddress::Pod(_, _) => ron_check_eq::<Pod>(a, b),
            K8sResourceAddress::Service(_, _) => ron_check_eq::<Service>(a, b),
            K8sResourceAddress::Deployment(_, _) => ron_check_eq::<Deployment>(a, b),
            K8sResourceAddress::ConfigMap(_, _) => ron_check_eq::<ConfigMap>(a, b),
            K8sResourceAddress::Secret(_, _) => ron_check_eq::<Secret>(a, b),
            K8sResourceAddress::PersistentVolumeClaim(_, _) => ron_check_eq::<PersistentVolumeClaim>(a, b),
            K8sResourceAddress::PersistentVolume(_) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::Role(_, _) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::RoleBinding(_, _) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::ClusterRole(_) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::ClusterRoleBinding(_) => ron_check_eq::<PersistentVolume>(a, b),
        }
    }

    async fn diag(&self, addr: &Path, a: &[u8]) -> Result<DiagnosticOutput, anyhow::Error> {
        let addr = K8sResourceAddress::from_path(addr)?;

        match addr {
            K8sResourceAddress::Namespace(_) => ron_check_syntax::<Namespace>(a),
            K8sResourceAddress::Pod(_, _) => ron_check_syntax::<Pod>(a),
            K8sResourceAddress::Service(_, _) => ron_check_syntax::<Service>(a),
            K8sResourceAddress::Deployment(_, _) => ron_check_syntax::<Deployment>(a),
            K8sResourceAddress::ConfigMap(_, _) => ron_check_syntax::<ConfigMap>(a),
            K8sResourceAddress::Secret(_, _) => ron_check_syntax::<Secret>(a),
            K8sResourceAddress::PersistentVolumeClaim(_, _) => ron_check_syntax::<PersistentVolumeClaim>(a),
            K8sResourceAddress::PersistentVolume(_) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::Role(_, _) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::RoleBinding(_, _) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::ClusterRole(_) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::ClusterRoleBinding(_) => ron_check_syntax::<PersistentVolume>(a),
        }
    }
}

impl K8sConnector {}
