use autoschematic_core::{
    connector::{Connector, ConnectorOp, ConnectorOutbox, GetResourceOutput, OpExecOutput, OpPlanOutput, ResourceAddress},
    connector_op,
    util::{diff_ron_values, ron_check_eq, ron_check_syntax, PrettyConfig, RON},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{
    api::{ListParams, PatchParams, PostParams},
    runtime::reflector::Lookup,
    Api, Client,
};
use serde::Serialize;

use crate::{
    addr::K8sResourceAddress,
    op::K8sConnectorOp,
    util::{from_str_option, strip_boring_fields},
};
use std::{ffi::OsString, path::Path};

use super::K8sConnector;

macro_rules! create_delete_patch {
    ($type:ty, $name:expr, $current:expr, $desired:expr) => {{
        let current: Option<$type> = from_str_option(&$current)?;
        let desired: Option<$type> = from_str_option(&$desired)?;
        match (current, desired) {
            (None, Some(desired)) => {
                Some(connector_op!(
                    K8sConnectorOp::Create(RON.to_string(&desired)?),
                    format!("Create {} {}", stringify!($type), $name)
                ))
            }

            (Some(_), None) => {
                Some(connector_op!(
                    K8sConnectorOp::Delete,
                    format!("Delete {} {}", stringify!($type), $name)
                ))
            }

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
            (None, Some(desired)) => {
                Some(connector_op!(
                    K8sConnectorOp::Create(RON.to_string(&desired)?),
                    format!("Create {} {}/{}", stringify!($type), $namespace, $name)
                ))
            }

            (Some(_), None) => {
                Some(connector_op!(
                    K8sConnectorOp::Delete,
                    format!("Delete {} {}/{}", stringify!($type), $namespace, $name)
                ))
            }

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
        current: Option<OsString>,
        desired: Option<OsString>,
    ) -> Result<Vec<OpPlanOutput>, anyhow::Error> {
        let mut res = Vec::new();
        let addr = K8sResourceAddress::from_path(addr)?;

        let op = match addr {
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
            K8sResourceAddress::Secret(namespace, name) => {
                create_delete_patch!(Secret, namespace, name, current, desired)
            }
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
        };

        if let Some(op) = op {
            res.push(op);
        }

        Ok(res)
    }
}
