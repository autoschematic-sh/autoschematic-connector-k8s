use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use autoschematic_core::connector::ResourceAddress;
use k8s_openapi::{
    api::{
        apps::v1::Deployment,
        core::v1::{ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, Pod, Service},
        rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
    },
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::{
    Api,
    api::{DynamicObject, ListParams},
    discovery,
    runtime::reflector::Lookup,
};

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

macro_rules! list_filtered {
    ($cluster:expr, $client:expr, $res:expr, $type:ident, $namespace:expr, $predicate:expr) => {{
        let resources: Api<$type> = Api::namespaced($client.clone(), &$namespace);
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            if !$predicate(&name) {
                continue;
            }
            $res.push(
                K8sClusterAddress {
                    cluster: $cluster.clone(),
                    res_addr: K8sResourceAddress::$type($namespace.to_string(), name.to_string()),
                }
                .to_path_buf(),
            );
        }
    }};

    ($cluster:expr, $client:expr, $res:expr, $type:ident, $predicate:expr) => {{
        let resources: Api<$type> = Api::all($client.clone());
        for resource in resources.list_metadata(&ListParams::default()).await? {
            let Some(name) = resource.name() else { continue };
            if !$predicate(&name) {
                continue;
            }
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

            list_filtered!(cluster, client, res, ClusterRole, |name: &Cow<str>| !name
                .starts_with("system:"));

            list_filtered!(cluster, client, res, ClusterRoleBinding, |name: &Cow<str>| !name
                .starts_with("system:"));

            list!(cluster, client, res, PersistentVolume);

            // List CustomResourceDefinitions
            list!(cluster, client, res, CustomResourceDefinition);

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
                // list!(cluster, client, res, Role, namespace_name);
                // list!(cluster, client, res, RoleBinding, namespace_name);
                list_filtered!(cluster, client, res, Role, namespace_name, |name: &Cow<str>| !name
                    .starts_with("system:"));

                list_filtered!(cluster, client, res, RoleBinding, namespace_name, |name: &Cow<str>| !name
                    .starts_with("system:"));
            }

            // List custom resources using discovery
            let discovery_client = discovery::Discovery::new(client.clone()).run().await?;

            // Iterate through all discovered custom resource types
            for group in discovery_client.groups() {
                for (api_resource, capabilities) in group.resources_by_stability() {
                    // Skip if not a custom resource (only include resources from non-core groups)
                    if api_resource.group.is_empty()
                        || api_resource.group.starts_with("k8s.io")
                        || api_resource.group.ends_with(".k8s.io")
                    {
                        continue;
                    }

                    // Skip subresources (like status, scale, etc.)
                    if api_resource.kind.contains('/') {
                        continue;
                    }

                    let kind = &api_resource.kind;

                    // List resources based on whether they are namespaced or cluster-scoped
                    if capabilities.scope == kube::discovery::Scope::Namespaced {
                        // List in each namespace
                        let nss: Api<Namespace> = Api::all(client.clone());
                        let namespaces = nss.list_metadata(&ListParams::default()).await?;

                        for namespace in &namespaces.items {
                            let Some(namespace_name) = namespace.name() else { continue };

                            let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), &namespace_name, &api_resource);
                            for resource in api.list_metadata(&ListParams::default()).await? {
                                let Some(name) = resource.name() else { continue };
                                res.push(
                                    K8sClusterAddress {
                                        cluster: cluster.clone(),
                                        res_addr: K8sResourceAddress::CustomResource(
                                            namespace_name.to_string(),
                                            kind.clone(),
                                            name.to_string(),
                                        ),
                                    }
                                    .to_path_buf(),
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(res)
    }
}
