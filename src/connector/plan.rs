use autoschematic_core::{
    connector::{
        Connector, ConnectorOp, ConnectorOutbox, GetResourceResponse, OpExecResponse, PlanResponseElement, ResourceAddress,
    },
    connector_op,
    util::{PrettyConfig, RON, diff_ron_values, ron_check_eq, ron_check_syntax},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{
    Api, Client,
    api::{ListParams, PatchParams, PostParams},
    runtime::reflector::Lookup,
};
use serde::Serialize;

use crate::{
    addr::{K8sClusterAddress, K8sResourceAddress},
    op::K8sConnectorOp,
    util::{from_str_option, strip_boring_fields},
};
use std::path::Path;

use super::K8sConnector;

macro_rules! create_delete_patch {
    ($type:ty, $name:expr, $current:expr, $desired:expr) => {{
        let current: Option<$type> = from_str_option(&$current)?;
        let desired: Option<$type> = from_str_option(&$desired)?;
        match (current, desired) {
            (None, Some(desired)) => Some(connector_op!(
                K8sConnectorOp::Create(RON.to_string(&desired)?),
                format!("Create {} {}", stringify!($type), $name)
            )),

            (Some(_), None) => Some(connector_op!(
                K8sConnectorOp::Delete,
                format!("Delete {} {}", stringify!($type), $name)
            )),

            (Some(current), Some(desired)) => {
                let diff = diff_ron_values(&current, &desired)?;
                Some(connector_op!(
                    K8sConnectorOp::Patch(RON.to_string_pretty(&desired, PrettyConfig::default())?),
                    format!("Modify {} {}:\n{}", stringify!($type), $name, diff)
                ))
            }
            _ => None,
        }
    }};
    ($type:ty, $namespace:expr, $name:expr, $current:expr, $desired:expr) => {{
        let current: Option<$type> = from_str_option(&$current)?;
        let desired: Option<$type> = from_str_option(&$desired)?;
        match (current, desired) {
            (None, Some(desired)) => Some(connector_op!(
                K8sConnectorOp::Create(RON.to_string(&desired)?),
                format!("Create {} {}/{}", stringify!($type), $namespace, $name)
            )),

            (Some(_), None) => Some(connector_op!(
                K8sConnectorOp::Delete,
                format!("Delete {} {}/{}", stringify!($type), $namespace, $name)
            )),

            (Some(current), Some(desired)) => {
                let diff = diff_ron_values(&current, &desired)?;
                Some(connector_op!(
                    K8sConnectorOp::Patch(RON.to_string_pretty(&desired, PrettyConfig::default())?),
                    format!("Modify {} {}/{}:\n{}", stringify!($type), $namespace, $name, diff)
                ))
            }
            _ => None,
        }
    }};
}

impl K8sConnector {
    pub async fn do_plan(
        &self,
        addr: &Path,
        current: Option<Vec<u8>>,
        desired: Option<Vec<u8>>,
    ) -> Result<Vec<PlanResponseElement>, anyhow::Error> {
        let mut res = Vec::new();
        let addr = K8sClusterAddress::from_path(addr)?;

        let op = match addr.res_addr {
            K8sResourceAddress::Namespace(name) => {
                create_delete_patch!(Namespace, name, current, desired)
            }
            K8sResourceAddress::Pod(namespace, name) => {
                create_delete_patch!(Pod, namespace, name, current, desired)
            }
            K8sResourceAddress::Service(namespace, name) => {
                create_delete_patch!(Service, namespace, name, current, desired)
            }
            K8sResourceAddress::Deployment(namespace, name) => {
                create_delete_patch!(Deployment, namespace, name, current, desired)
            }
            K8sResourceAddress::ConfigMap(namespace, name) => {
                create_delete_patch!(ConfigMap, namespace, name, current, desired)
            }
            // K8sResourceAddress::Secret(namespace, name) => {
            //     create_delete_patch!(Secret, namespace, name, current, desired)
            // }
            K8sResourceAddress::PersistentVolumeClaim(namespace, name) => {
                create_delete_patch!(PersistentVolumeClaim, namespace, name, current, desired)
            }
            K8sResourceAddress::PersistentVolume(name) => {
                create_delete_patch!(PersistentVolume, name, current, desired)
            }
            K8sResourceAddress::Role(namespace, name) => {
                create_delete_patch!(Role, namespace, name, current, desired)
            }
            K8sResourceAddress::RoleBinding(namespace, name) => {
                create_delete_patch!(RoleBinding, namespace, name, current, desired)
            }
            K8sResourceAddress::ClusterRole(name) => {
                create_delete_patch!(ClusterRole, name, current, desired)
            }
            K8sResourceAddress::ClusterRoleBinding(name) => {
                create_delete_patch!(ClusterRoleBinding, name, current, desired)
            }
            K8sResourceAddress::Binding(_, _) => todo!(),
            K8sResourceAddress::Endpoints(_, _) => todo!(),
            K8sResourceAddress::LimitRange(_, _) => todo!(),
            K8sResourceAddress::Node(_, _) => todo!(),
            K8sResourceAddress::PodTemplate(_, _) => todo!(),
            K8sResourceAddress::ReplicationController(_, _) => todo!(),
            K8sResourceAddress::ResourceQuota(_, _) => todo!(),
            K8sResourceAddress::ServiceAccount(_, _) => todo!(),
        };

        if let Some(op) = op {
            res.push(op);
        }

        Ok(res)
    }
}
