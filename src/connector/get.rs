use std::path::Path;

use anyhow::bail;
use autoschematic_core::connector::{GetResourceResponse, ResourceAddress};
use k8s_openapi::{
    api::{
        apps::v1::Deployment,
        core::v1::{ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
        rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
    },
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::{Api, api::DynamicObject, client::ClientBuilder, discovery, ResourceExt};

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
        let resource: Result<$type, kube::Error> = resources.get(&$name).await;
        let Ok(resource) = resource else { return Ok(None) };
        // strip_boring_fields(&mut resource.metadata);
        return get_ser_resource_output(&resource);
    }};
    ($client:expr, $type:ident, $namespace:expr, $name:expr) => {{
        let resources: Api<$type> = Api::namespaced($client, &$namespace);
        let resource: Result<$type, kube::Error> = resources.get(&$name).await;
        let Ok(resource) = resource else { return Ok(None) };
        // strip_boring_fields(&mut resource.metadata);
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
            K8sResourceAddress::CustomResourceDefinition(name) => get!(client, CustomResourceDefinition, name),
            K8sResourceAddress::CustomResource(namespace, kind, name) => {
                // For custom resources, we need to use discovery to find the API group/version
                // and then use dynamic API to fetch the resource
                let discovery_client = discovery::Discovery::new(client.clone()).run().await?;

                // Find the resource by kind
                let api_resource = discovery_client
                    .groups()
                    .flat_map(|group| group.resources_by_stability())
                    .find(|(ar, _)| ar.kind == kind)
                    .map(|(ar, _)| ar);

                let Some(api_resource) = api_resource else {
                    bail!("Custom resource kind '{}' not found in cluster", kind);
                };

                let api: Api<DynamicObject> = Api::namespaced_with(client, &namespace, &api_resource);
                let resource: Result<DynamicObject, kube::Error> = api.get(&name).await;
                let Ok(resource) = resource else { return Ok(None) };

                return get_ser_resource_output(&resource);
            }
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
