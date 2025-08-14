use std::path::{Path, PathBuf};

use anyhow::bail;
use autoschematic_core::connector::ResourceAddress;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, PersistentVolumeClaimCondition, Pod, Secret, Service,
    },
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{Api, api::ListParams, runtime::reflector::Lookup};

use crate::addr::K8sClusterAddress;
use crate::addr::K8sResourceAddress;

use super::K8sConnector;

macro_rules! list {
    ($cluster:expr, $client:expr, $res:expr, $type:ident, $namespace:expr) => {{
        let resources: Api<$type> = Api::namespaced($client.clone(), &$namespace);
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            $res.push(
                K8sClusterAddress {
                    cluster: $cluster.clone(),
                    res_addr: K8sResourceAddress::$type($namespace.to_string(), name.to_string()),
                }
                .to_path_buf(),
            );
        }
    }};

    ($cluster:expr, $client:expr, $res:expr, $type:ident) => {{
        let resources: Api<$type> = Api::all($client.clone());
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            $res.push(
                K8sClusterAddress {
                    cluster: $cluster.clone(),
                    res_addr: K8sResourceAddress::$type(name.to_string()),
                }
                .to_path_buf(),
            );
        }
    }};
}

impl K8sConnector {
    pub async fn do_list(&self, subpath: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        let mut res = Vec::new();

        for cluster in self.clusters()? {
            let client = (*self.get_or_init_client(&cluster).await?).clone();

            list!(cluster, client, res, ClusterRole);
            list!(cluster, client, res, ClusterRoleBinding);
            list!(cluster, client, res, PersistentVolume);

            let nss: Api<Namespace> = Api::all(client.clone());
            let namespaces = nss.list_metadata(&ListParams::default()).await?;

            for namespace in &namespaces.items {
                let Some(namespace_name) = namespace.name() else { continue };
                res.push(
                    K8sClusterAddress {
                        cluster: cluster.clone(),
                        res_addr: K8sResourceAddress::Namespace(namespace_name.to_string()),
                    }
                    .to_path_buf(),
                );

                list!(cluster, client, res, Pod, namespace_name);
                list!(cluster, client, res, Service, namespace_name);
                list!(cluster, client, res, Deployment, namespace_name);
                list!(cluster, client, res, ConfigMap, namespace_name);
                // list!(cluster, client, res, Secret, namespace_name);
                list!(cluster, client, res, PersistentVolumeClaim, namespace_name);
                list!(cluster, client, res, Role, namespace_name);
                list!(cluster, client, res, RoleBinding, namespace_name);
            }
        }

        Ok(res)
    }
}
