use std::path::Path;

use anyhow::bail;
use autoschematic_core::connector::{GetResourceResponse, ResourceAddress};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{client::ClientBuilder, Api};

use crate::{
    addr::{K8sClusterAddress, K8sResourceAddress},
    util::{get_ser_resource_output, strip_boring_fields},
};

use super::K8sConnector;

macro_rules! match_res {
    ($type:ident) => {
        K8sResourceAddress::$type(name)
    };
}
macro_rules! match_res_namespaced {
    ($type:ident) => {
        K8sResourceAddress::$type(namespace, name)
    };
}

macro_rules! get {
    ($client:expr, $type:ident, $name:ident) => {{
        let resources: Api<$type> = Api::all($client);
        let mut resource: $type = resources.get(&$name).await?;
        strip_boring_fields(&mut resource.metadata);
        return get_ser_resource_output(&resource);
    }};
    ($client:expr, $type:ident, $namespace:expr, $name:expr) => {{
        let resources: Api<$type> = Api::namespaced($client, &$namespace);
        let mut resource: $type = resources.get(&$name).await?;
        strip_boring_fields(&mut resource.metadata);
        return get_ser_resource_output(&resource);
    }};
}

impl K8sConnector {
    pub async fn do_get(&self, addr: &Path) -> Result<Option<GetResourceResponse>, anyhow::Error> {
        let addr = K8sClusterAddress::from_path(addr)?;
        
        let client = (*self.get_or_init_client(&addr.cluster).await?).clone();
        

        match addr.res_addr {
            K8sResourceAddress::Namespace(name) => get!(client, Namespace, name),
            K8sResourceAddress::Pod(namespace, name) => get!(client, Pod, namespace, name),
            K8sResourceAddress::Service(namespace, name) => get!(client, Service, namespace, name),
            K8sResourceAddress::Deployment(namespace, name) => get!(client, Deployment, namespace, name),
            K8sResourceAddress::ConfigMap(namespace, name) => get!(client, ConfigMap, namespace, name),
            // K8sResourceAddress::Secret(namespace, name) => get!(client, Secret, namespace, name),
            K8sResourceAddress::PersistentVolumeClaim(namespace, name) => get!(client, PersistentVolumeClaim, namespace, name),
            K8sResourceAddress::Role(namespace, name) => get!(client, Role, namespace, name),
            K8sResourceAddress::RoleBinding(namespace, name) => get!(client, RoleBinding, namespace, name),
            K8sResourceAddress::PersistentVolume(name) => get!(client, PersistentVolume, name),
            K8sResourceAddress::ClusterRole(name) => get!(client, ClusterRole, name),
            K8sResourceAddress::ClusterRoleBinding(name) => get!(client, ClusterRoleBinding, name),
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
