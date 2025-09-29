use std::path::{Path, PathBuf};

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;
use autoschematic_core::{
    connector::ResourceAddress,
    error_util::{invalid_addr, invalid_addr_path},
};

type Namespace = String;
type Name = String;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
pub enum K8sResourceAddress {
    Namespace(Namespace),
    Pod(Namespace, Name),
    Service(Namespace, Name),
    Deployment(Namespace, Name),
    ConfigMap(Namespace, Name),
    // Secret(Namespace, Name),
    PersistentVolumeClaim(Namespace, Name),
    PersistentVolume(Namespace),
    Role(Namespace, Name),
    RoleBinding(Namespace, Name),
    ClusterRole(Name),
    ClusterRoleBinding(Name),

    // Binding(Namespace, Name),
    // Endpoints(Namespace, Name),
    // LimitRange(Namespace, Name),
    // Node(Namespace, Name),

    // PodTemplate(Namespace, Name),
    // ReplicationController(Namespace, Name),
    // ResourceQuota(Namespace, Name),
    // ServiceAccount(Namespace, Name),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
pub struct K8sClusterAddress {
    pub cluster: String,
    pub res_addr: K8sResourceAddress,
}

impl ResourceAddress for K8sClusterAddress {
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let path_components: Vec<&str> = path
            .components()
            .into_iter()
            .map(|s| s.as_os_str().to_str().unwrap_or_default())
            .collect();

        fn val(s: &str) -> bool {
            s.ends_with(".yaml") || s.ends_with(".yml")
        }
        fn strip(s: &str) -> &str {
            s.strip_suffix(".yaml").unwrap_or(s)
        }

        match &path_components[..] {
            ["k8s", cluster, rest @ ..] => {
                let res_addr = match &rest[..] {
                    ["ns", namespace, "ns.yaml"] => K8sResourceAddress::Namespace(namespace.to_string()),
                    ["ns", namespace, "ns.yml"] => K8sResourceAddress::Namespace(namespace.to_string()),
                    ["ns", namespace, "pod", pod_name] if val(pod_name) => {
                        K8sResourceAddress::Pod(namespace.to_string(), strip(pod_name).to_string())
                    }
                    ["ns", namespace, "service", service_name] if val(service_name) => {
                        K8sResourceAddress::Service(namespace.to_string(), strip(service_name).to_string())
                    }
                    ["ns", namespace, "deployment", deployment_name] if val(deployment_name) => {
                        K8sResourceAddress::Deployment(namespace.to_string(), strip(deployment_name).to_string())
                    }
                    ["ns", namespace, "configmap", configmap_name] if val(configmap_name) => {
                        K8sResourceAddress::ConfigMap(namespace.to_string(), strip(configmap_name).to_string())
                    }
                    // ["ns", namespace, "secret", secret_name] if val(secret_name) => {
                    //     K8sResourceAddress::Secret(namespace.to_string(), strip(secret_name).to_string())
                    // }
                    ["ns", namespace, "persistentvolumeclaim", pvc_name] if val(pvc_name) => {
                        K8sResourceAddress::PersistentVolumeClaim(namespace.to_string(), strip(pvc_name).to_string())
                    }
                    ["persistentvolume", pv_name] if val(pv_name) => {
                        K8sResourceAddress::PersistentVolume(strip(pv_name).to_string())
                    }
                    ["ns", namespace, "role", role_name] if val(role_name) => {
                        K8sResourceAddress::Role(namespace.to_string(), strip(role_name).to_string())
                    }
                    ["ns", namespace, "rolebinding", role_name] if val(role_name) => {
                        K8sResourceAddress::RoleBinding(namespace.to_string(), strip(role_name).to_string())
                    }
                    ["clusterrole", role_name] if val(role_name) => {
                        K8sResourceAddress::ClusterRole(strip(role_name).to_string())
                    }
                    ["clusterrolebinding", role_name] if val(role_name) => {
                        K8sResourceAddress::ClusterRoleBinding(strip(role_name).to_string())
                    }
                    _ => return Err(invalid_addr_path(path)),
                };

                Ok(K8sClusterAddress {
                    cluster: cluster.to_string(),
                    res_addr,
                })
            }

            _ => Err(invalid_addr_path(path)),
        }
    }

    fn to_path_buf(&self) -> PathBuf {
        let cluster = &self.cluster;
        match &self.res_addr {
            K8sResourceAddress::Namespace(namespace) => PathBuf::from(format!("k8s/{cluster}/ns/{}/ns.yaml", namespace)),
            K8sResourceAddress::Pod(namespace, pod) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/pod/{}.yaml", namespace, pod))
            }
            K8sResourceAddress::Service(namespace, service) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/service/{}.yaml", namespace, service))
            }
            K8sResourceAddress::Deployment(namespace, deployment) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/deployment/{}.yaml", namespace, deployment))
            }
            K8sResourceAddress::ConfigMap(namespace, configmap) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/configmap/{}.yaml", namespace, configmap))
            }
            // K8sResourceAddress::Secret(namespace, secret) => {
            //     PathBuf::from(format!("k8s/{cluster}/ns/{}/secret/{}.yaml", namespace, secret))
            // }
            K8sResourceAddress::PersistentVolumeClaim(namespace, pvc) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/persistentvolumeclaim/{}.yaml", namespace, pvc))
            }
            K8sResourceAddress::PersistentVolume(pv) => PathBuf::from(format!("k8s/{cluster}persistentvolume/{}.yaml", pv)),
            K8sResourceAddress::Role(namespace, role) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/role/{}.yaml", namespace, role))
            }
            K8sResourceAddress::ClusterRole(name) => PathBuf::from(format!("k8s/{cluster}/clusterrole/{}.yaml", name)),
            K8sResourceAddress::RoleBinding(namespace, name) => {
                PathBuf::from(format!("k8s/{cluster}/ns/{}/rolebinding/{}.yaml", namespace, name))
            }
            K8sResourceAddress::ClusterRoleBinding(name) => {
                PathBuf::from(format!("k8s/{cluster}/clusterrolebinding/{}.yaml", name))
            }
            // K8sResourceAddress::Binding(namespace, name) => {
            //     PathBuf::from(format!("k8s/{cluster}/ns/{}/binding/{}.yaml", namespace, name))
            // }
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
