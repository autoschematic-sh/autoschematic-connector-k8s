use std::path::{Path, PathBuf};

use autoschematic_core::connector::ResourceAddress;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{
        ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, PersistentVolumeClaimCondition, Pod, Secret, Service,
    },
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{api::ListParams, runtime::reflector::Lookup, Api};

use crate::addr::K8sResourceAddress;

use super::K8sConnector;

macro_rules! list {
    ($client:expr, $res:expr, $type:ident, $namespace:expr) => {{
        let resources: Api<$type> = Api::namespaced($client, &$namespace);
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            $res.push(K8sResourceAddress::$type($namespace.to_string(), name.to_string()).to_path_buf());
        }
    }};

    ($client:expr, $res:expr, $type:ident) => {{
        let resources: Api<$type> = Api::all($client);
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            $res.push(K8sResourceAddress::$type(name.to_string()).to_path_buf());
        }
    }};
}

impl K8sConnector {
    pub async fn do_list(&self, subpath: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        let mut res = Vec::new();
        let nss: Api<Namespace> = Api::all(self.client.clone());
        let namespaces = nss.list_metadata(&ListParams::default()).await?;

        list!(self.client.clone(), res, ClusterRole);
        list!(self.client.clone(), res, ClusterRoleBinding);
        list!(self.client.clone(), res, PersistentVolume);

        for namespace in namespaces.items {
            let Some(namespace_name) = namespace.name() else { continue };
            res.push(K8sResourceAddress::Namespace(namespace_name.to_string()).to_path_buf());

            list!(self.client.clone(), res, Pod, namespace_name);
            list!(self.client.clone(), res, Service, namespace_name);
            list!(self.client.clone(), res, Deployment, namespace_name);
            list!(self.client.clone(), res, ConfigMap, namespace_name);
            list!(self.client.clone(), res, Secret, namespace_name);
            list!(self.client.clone(), res, PersistentVolumeClaim, namespace_name);
            list!(self.client.clone(), res, Role, namespace_name);
            list!(self.client.clone(), res, RoleBinding, namespace_name);
        }

        Ok(res)
    }
}
