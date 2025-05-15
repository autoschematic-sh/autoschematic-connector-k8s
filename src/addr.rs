use std::path::{Path, PathBuf};

use arbitrary::Arbitrary;
use autoschematic_core::connector::ResourceAddress;

type Namespace = String;
type Name = String;

#[derive(Debug, Clone, PartialEq, Eq, Arbitrary)]
pub enum K8sResourceAddress {
    Namespace(Namespace),
    Pod(Namespace, Name),
    Service(Namespace, Name),
    Deployment(Namespace, Name),
    ConfigMap(Namespace, Name),
    Secret(Namespace, Name),
    PersistentVolumeClaim(Namespace, Name),
    PersistentVolume(Namespace),
    Role(Namespace, Name),
    RoleBinding(Namespace, Name),
    ClusterRole(Name),
    ClusterRoleBinding(Name),
}

impl ResourceAddress for K8sResourceAddress {
    fn from_path(path: &Path) -> anyhow::Result<Option<Self>> {
        let path_components: Vec<&str> = path
            .components()
            .into_iter()
            .map(|s| s.as_os_str().to_str().unwrap_or_default())
            .collect();

        fn val(s: &str) -> bool {
            s.ends_with(".ron")
        }
        fn strip(s: &str) -> &str {
            s.strip_suffix(".ron").unwrap_or(s)
        }

        let res = match &path_components[..] {
            ["k8s", "ns", namespace, "ns.ron"] => Some(K8sResourceAddress::Namespace(namespace.to_string())),
            ["k8s", "ns", namespace, "pod", pod_name] if val(pod_name) => {
                Some(K8sResourceAddress::Pod(namespace.to_string(), strip(pod_name).to_string()))
            }
            ["k8s", "ns", namespace, "service", service_name] if val(service_name) => {
                Some(K8sResourceAddress::Service(
                    namespace.to_string(),
                    strip(service_name).to_string(),
                ))
            }
            ["k8s", "ns", namespace, "deployment", deployment_name] if val(deployment_name) => {
                Some(K8sResourceAddress::Deployment(
                    namespace.to_string(),
                    strip(deployment_name).to_string(),
                ))
            }
            ["k8s", "ns", namespace, "configmap", configmap_name] if val(configmap_name) => {
                Some(K8sResourceAddress::ConfigMap(
                    namespace.to_string(),
                    strip(configmap_name).to_string(),
                ))
            }
            ["k8s", "ns", namespace, "secret", secret_name] if val(secret_name) => {
                Some(K8sResourceAddress::Secret(
                    namespace.to_string(),
                    strip(secret_name).to_string(),
                ))
            }
            ["k8s", "ns", namespace, "persistentvolumeclaim", pvc_name] if val(pvc_name) => {
                Some(K8sResourceAddress::PersistentVolumeClaim(
                    namespace.to_string(),
                    strip(pvc_name).to_string(),
                ))
            }
            ["k8s", "persistentvolume", pv_name] if val(pv_name) => {
                Some(K8sResourceAddress::PersistentVolume(strip(pv_name).to_string()))
            }
            ["k8s", "ns", namespace, "role", role_name] if val(role_name) => {
                Some(K8sResourceAddress::Role(namespace.to_string(), strip(role_name).to_string()))
            }
            ["k8s", "ns", namespace, "rolebinding", role_name] if val(role_name) => {
                Some(K8sResourceAddress::RoleBinding(
                    namespace.to_string(),
                    strip(role_name).to_string(),
                ))
            }
            ["k8s", "clusterrole", role_name] if val(role_name) => {
                Some(K8sResourceAddress::ClusterRole(strip(role_name).to_string()))
            }
            ["k8s", "clusterrolebinding", role_name] if val(role_name) => {
                Some(K8sResourceAddress::ClusterRoleBinding(strip(role_name).to_string()))
            }
            _ => None,
        };
        Ok(res)
    }

    fn to_path_buf(&self) -> PathBuf {
        match self {
            K8sResourceAddress::Namespace(namespace) => PathBuf::from(format!("k8s/ns/{}/ns.ron", namespace)),
            K8sResourceAddress::Pod(namespace, pod) => PathBuf::from(format!("k8s/ns/{}/pod/{}.ron", namespace, pod)),
            K8sResourceAddress::Service(namespace, service) => {
                PathBuf::from(format!("k8s/ns/{}/service/{}.ron", namespace, service))
            }
            K8sResourceAddress::Deployment(namespace, deployment) => {
                PathBuf::from(format!("k8s/ns/{}/deployment/{}.ron", namespace, deployment))
            }
            K8sResourceAddress::ConfigMap(namespace, configmap) => {
                PathBuf::from(format!("k8s/ns/{}/configmap/{}.ron", namespace, configmap))
            }
            K8sResourceAddress::Secret(namespace, secret) => {
                PathBuf::from(format!("k8s/ns/{}/secret/{}.ron", namespace, secret))
            }
            K8sResourceAddress::PersistentVolumeClaim(namespace, pvc) => {
                PathBuf::from(format!("k8s/ns/{}/persistentvolumeclaim/{}.ron", namespace, pvc))
            }
            K8sResourceAddress::PersistentVolume(pv) => PathBuf::from(format!("k8s/persistentvolume/{}.ron", pv)),
            K8sResourceAddress::Role(namespace, role) => PathBuf::from(format!("k8s/ns/{}/role/{}.ron", namespace, role)),
            K8sResourceAddress::ClusterRole(name) => PathBuf::from(format!("k8s/clusterrole/{}.ron", name)),
            K8sResourceAddress::RoleBinding(namespace, name) => {
                PathBuf::from(format!("k8s/ns/{}/rolebinding/{}.ron", namespace, name))
            }
            K8sResourceAddress::ClusterRoleBinding(name) => PathBuf::from(format!("k8s/clusterrolebinding/{}.ron", name)),
        }
    }
}
