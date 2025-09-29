use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::bail;
use async_trait::async_trait;
use autoschematic_core::{
    connector::{
        Connector, ConnectorOutbox, FilterResponse, GetResourceResponse, OpExecResponse, PlanResponseElement, ResourceAddress,
    },
    diag::DiagnosticResponse,
    error::{AutoschematicError, AutoschematicErrorType},
    get_resource_response,
    tarpc_bridge::TarpcConnector,
    util::{PrettyConfig, RON, ron_check_eq, ron_check_syntax},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
};
use kube::{
    Client, Config,
    config::{KubeConfigOptions, Kubeconfig},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{
    addr::{K8sClusterAddress, K8sResourceAddress},
    util::strip_boring_fields,
};

mod get;
mod list;
mod op_exec;
mod plan;

pub enum SerdeBackend {
    RON,
    YAML,
}

impl SerdeBackend {
    pub fn to_string<S: Serialize>(&self, s: &S) -> anyhow::Result<String> {
        let res = match self {
            SerdeBackend::RON => RON.to_string_pretty(s, PrettyConfig::default())?,
            SerdeBackend::YAML => serde_yaml::to_string(s)?,
        };
        Ok(res)
    }

    pub fn from_str<'a, D: Deserialize<'a>>(&self, d: &'a str) -> anyhow::Result<D> {
        let res = match self {
            SerdeBackend::RON => RON.from_str(d)?,
            SerdeBackend::YAML => serde_yaml::from_str(d)?,
        };
        Ok(res)
    }
}

pub struct K8sConnector {
    // outbox: ConnectorOutbox,
    // pub name: String,
    prefix: PathBuf,
    client_cache: RwLock<HashMap<String, Arc<Client>>>,
}

impl K8sConnector {
    pub fn clusters(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec![String::from("default")])
    }

    pub fn kubecfg(&self, cluster: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    pub async fn get_or_init_client(&self, cluster: &str) -> anyhow::Result<Arc<Client>> {
        let mut cache = self.client_cache.write().await;

        if !cache.contains_key(cluster) {
            let client = match self.kubecfg(cluster)? {
                Some(kubecfg_path) => {
                    let kubecfg = Kubeconfig::read_from(kubecfg_path)?;
                    Client::try_from(
                        Config::from_custom_kubeconfig(
                            kubecfg,
                            &KubeConfigOptions {
                                context: None,
                                cluster: Some(cluster.into()),
                                user: None,
                            },
                        )
                        .await?,
                    )?
                }
                None => Client::try_default().await?,
            };

            cache.insert(cluster.to_string(), Arc::new(client));
        };

        let Some(client) = cache.get(cluster) else {
            bail!("Failed to get client for cluster {}", cluster);
        };

        Ok(client.clone())
    }
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
            prefix: prefix.into(),
            client_cache: RwLock::new(HashMap::new())
        }))
    }

    async fn init(&self) -> anyhow::Result<()> {
        // *self.client.lock().await = Some(Client::try_default().await?);
        Ok(())
    }

    async fn filter(&self, addr: &Path) -> Result<FilterResponse, anyhow::Error> {
        if let Ok(_) = K8sClusterAddress::from_path(addr) {
            Ok(FilterResponse::Resource)
        } else {
            Ok(FilterResponse::None)
        }
    }

    async fn list(&self, subpath: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        self.do_list(subpath).await
    }

    async fn get(&self, addr: &Path) -> Result<Option<GetResourceResponse>, anyhow::Error> {
        self.do_get(addr).await
    }

    async fn plan(
        &self,
        addr: &Path,
        current: Option<Vec<u8>>,
        desired: Option<Vec<u8>>,
    ) -> Result<Vec<PlanResponseElement>, anyhow::Error> {
        self.do_plan(addr, current, desired).await
    }

    async fn op_exec(&self, addr: &Path, op: &str) -> Result<OpExecResponse, anyhow::Error> {
        self.do_op_exec(addr, op).await
    }

    async fn eq(&self, addr: &Path, a: &[u8], b: &[u8]) -> Result<bool, anyhow::Error> {
        let addr = K8sClusterAddress::from_path(addr)?;

        match &addr.res_addr {
            K8sResourceAddress::Namespace(_) => ron_check_eq::<Namespace>(a, b),
            K8sResourceAddress::Pod(_, _) => ron_check_eq::<Pod>(a, b),
            K8sResourceAddress::Service(_, _) => ron_check_eq::<Service>(a, b),
            K8sResourceAddress::Deployment(_, _) => ron_check_eq::<Deployment>(a, b),
            K8sResourceAddress::ConfigMap(_, _) => ron_check_eq::<ConfigMap>(a, b),
            // K8sResourceAddress::Secret(_, _) => ron_check_eq::<Secret>(a, b),
            K8sResourceAddress::PersistentVolumeClaim(_, _) => ron_check_eq::<PersistentVolumeClaim>(a, b),
            K8sResourceAddress::PersistentVolume(_) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::Role(_, _) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::RoleBinding(_, _) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::ClusterRole(_) => ron_check_eq::<PersistentVolume>(a, b),
            K8sResourceAddress::ClusterRoleBinding(_) => ron_check_eq::<PersistentVolume>(a, b),
            // K8sResourceAddress::Binding(_, _) => todo!(),
            // K8sResourceAddress::Endpoints(_, _) => todo!(),
            // K8sResourceAddress::LimitRange(_, _) => todo!(),
            // K8sResourceAddress::Node(_, _) => todo!(),
            // K8sResourceAddress::PodTemplate(_, _) => todo!(),
            // K8sResourceAddress::ReplicationController(_, _) => todo!(),
            // K8sResourceAddress::ResourceQuota(_, _) => todo!(),
            // K8sResourceAddress::ServiceAccount(_, _) => todo!(),
        }
    }

    async fn diag(&self, addr: &Path, a: &[u8]) -> Result<Option<DiagnosticResponse>, anyhow::Error> {
        let addr = K8sClusterAddress::from_path(addr)?;

        match &addr.res_addr {
            K8sResourceAddress::Namespace(_) => ron_check_syntax::<Namespace>(a),
            K8sResourceAddress::Pod(_, _) => ron_check_syntax::<Pod>(a),
            K8sResourceAddress::Service(_, _) => ron_check_syntax::<Service>(a),
            K8sResourceAddress::Deployment(_, _) => ron_check_syntax::<Deployment>(a),
            K8sResourceAddress::ConfigMap(_, _) => ron_check_syntax::<ConfigMap>(a),
            // K8sResourceAddress::Secret(_, _) => ron_check_syntax::<Secret>(a),
            K8sResourceAddress::PersistentVolumeClaim(_, _) => ron_check_syntax::<PersistentVolumeClaim>(a),
            K8sResourceAddress::PersistentVolume(_) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::Role(_, _) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::RoleBinding(_, _) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::ClusterRole(_) => ron_check_syntax::<PersistentVolume>(a),
            K8sResourceAddress::ClusterRoleBinding(_) => ron_check_syntax::<PersistentVolume>(a),
            // K8sResourceAddress::Binding(_, _) => todo!(),
            // K8sResourceAddress::Endpoints(_, _) => todo!(),
            // K8sResourceAddress::LimitRange(_, _) => todo!(),
            // K8sResourceAddress::Node(_, _) => todo!(),
            // K8sResourceAddress::PodTemplate(_, _) => todo!(),
            // K8sResourceAddress::ReplicationController(_, _) => todo!(),
            // K8sResourceAddress::ResourceQuota(_, _) => todo!(),
            // K8sResourceAddress::ServiceAccount(_, _) => todo!(),
        }
    }
}

impl K8sConnector {}
