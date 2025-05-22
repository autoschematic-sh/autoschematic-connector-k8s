use anyhow::bail;
use autoschematic_core::{
    connector::{Connector, ConnectorOp, ConnectorOutbox, GetResourceOutput, OpExecOutput, OpPlanOutput, ResourceAddress},
    connector_op, op_exec_output,
    util::{diff_ron_values, ron_check_eq, ron_check_syntax, PrettyConfig, RON},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{
    api::{DeleteParams, ListParams, PatchParams, PostParams},
    runtime::reflector::Lookup,
    Api, Client,
};
use serde::Serialize;

use crate::{
    addr::K8sResourceAddress,
    op::K8sConnectorOp,
    util::{from_str_option, strip_boring_fields},
};
use std::path::Path;

use std::collections::HashMap;

use super::K8sConnector;

macro_rules! create_delete_patch {
    ($type:ty, $name:expr, $client:expr, $op:expr) => {{
        let api: Api<$type> = Api::all($client);

        match $op {
            K8sConnectorOp::Create(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.create(&PostParams::default(), &resource).await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Created {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Patch(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.patch($name, &PatchParams::default(), &kube::api::Patch::Apply(resource))
                    .await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Modified {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Delete => {
                api.delete($name, &DeleteParams::default()).await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Deleted {} {}", stringify!($type), $name)),
                }
            }
        }
    }};
    ($type:ty, $namespace:expr, $name:expr, $client:expr, $op:expr) => {{
        let api: Api<$type> = Api::namespaced($client, $namespace);

        match $op {
            K8sConnectorOp::Create(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.create(&PostParams::default(), &resource).await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Created {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Patch(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.patch($name, &PatchParams::default(), &kube::api::Patch::Apply(resource))
                    .await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Modified {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Delete => {
                api.delete($name, &DeleteParams::default()).await?;
                OpExecOutput {
                    outputs: None,
                    friendly_message: Some(format!("Deleted {} {}", stringify!($type), $name)),
                }
            }
        }
    }};
}

impl K8sConnector {
    pub async fn do_op_exec(&self, addr: &Path, op: &str) -> Result<OpExecOutput, anyhow::Error> {
        let addr = K8sResourceAddress::from_path(addr)?;

        let op = K8sConnectorOp::from_str(op)?;

        let output = match &addr {
            K8sResourceAddress::Namespace(name) => {
                create_delete_patch!(Namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::Pod(namespace, name) => {
                create_delete_patch!(Pod, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::Service(namespace, name) => {
                create_delete_patch!(Service, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::Deployment(namespace, name) => {
                create_delete_patch!(Deployment, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::ConfigMap(namespace, name) => {
                create_delete_patch!(ConfigMap, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::Secret(namespace, name) => {
                create_delete_patch!(Secret, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::PersistentVolumeClaim(namespace, name) => {
                create_delete_patch!(PersistentVolumeClaim, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::PersistentVolume(name) => {
                create_delete_patch!(PersistentVolume, name, self.client.clone(), op)
            }
            K8sResourceAddress::Role(namespace, name) => {
                create_delete_patch!(Role, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::RoleBinding(namespace, name) => {
                create_delete_patch!(RoleBinding, namespace, name, self.client.clone(), op)
            }
            K8sResourceAddress::ClusterRole(name) => {
                create_delete_patch!(ClusterRole, name, self.client.clone(), op)
            }
            K8sResourceAddress::ClusterRoleBinding(name) => {
                create_delete_patch!(ClusterRoleBinding, name, self.client.clone(), op)
            }
        };

        Ok(output)
    }
}
