use anyhow::bail;
use autoschematic_core::{
    connector::{
        Connector, ConnectorOp, ConnectorOutbox, GetResourceResponse, OpExecResponse, PlanResponseElement, ResourceAddress,
    },
    connector_op, op_exec_output,
    util::{PrettyConfig, RON, diff_ron_values, ron_check_eq, ron_check_syntax},
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, NamespaceSpec, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service},
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
};
use kube::{
    Api, Client,
    api::{DeleteParams, ListParams, PatchParams, PostParams},
    client,
    runtime::reflector::Lookup,
};
use serde::Serialize;

use crate::{
    addr::{K8sClusterAddress, K8sResourceAddress},
    op::K8sConnectorOp,
    util::{from_str_option, strip_boring_fields},
};
use std::path::Path;

use std::collections::HashMap;

use super::K8sConnector;

macro_rules! create_delete_patch {
    ($type:ty, $name:expr, $client:expr, $op:expr) => {{
        let api: Api<$type> = Api::all($client.clone());

        match $op {
            K8sConnectorOp::Create(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.create(&PostParams::default(), &resource).await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Created {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Patch(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.patch($name, &PatchParams::default(), &kube::api::Patch::Apply(resource))
                    .await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Modified {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Delete => {
                api.delete($name, &DeleteParams::default()).await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Deleted {} {}", stringify!($type), $name)),
                }
            }
        }
    }};
    ($type:ty, $namespace:expr, $name:expr, $client:expr, $op:expr) => {{
        let api: Api<$type> = Api::namespaced($client.clone(), $namespace);

        match $op {
            K8sConnectorOp::Create(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.create(&PostParams::default(), &resource).await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Created {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Patch(resource) => {
                let resource: $type = RON.from_str(&resource)?;
                api.patch($name, &PatchParams::default(), &kube::api::Patch::Apply(resource))
                    .await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Modified {} {}", stringify!($type), $name)),
                }
            }
            K8sConnectorOp::Delete => {
                api.delete($name, &DeleteParams::default()).await?;
                OpExecResponse {
                    outputs: None,
                    friendly_message: Some(format!("Deleted {} {}", stringify!($type), $name)),
                }
            }
        }
    }};
}

impl K8sConnector {
    pub async fn do_op_exec(&self, addr: &Path, op: &str) -> Result<OpExecResponse, anyhow::Error> {
        let addr = K8sClusterAddress::from_path(addr)?;

        let op = K8sConnectorOp::from_str(op)?;

        let client = (*self.get_or_init_client(&addr.cluster).await?).clone();

        let output = match &addr.res_addr {
            K8sResourceAddress::Namespace(name) => {
                create_delete_patch!(Namespace, name, client, op)
            }
            K8sResourceAddress::Pod(namespace, name) => {
                create_delete_patch!(Pod, namespace, name, client, op)
            }
            K8sResourceAddress::Service(namespace, name) => {
                create_delete_patch!(Service, namespace, name, client, op)
            }
            K8sResourceAddress::Deployment(namespace, name) => {
                create_delete_patch!(Deployment, namespace, name, client, op)
            }
            K8sResourceAddress::ConfigMap(namespace, name) => {
                create_delete_patch!(ConfigMap, namespace, name, client, op)
            }
            K8sResourceAddress::PersistentVolumeClaim(namespace, name) => {
                create_delete_patch!(PersistentVolumeClaim, namespace, name, client, op)
            }
            K8sResourceAddress::PersistentVolume(name) => {
                create_delete_patch!(PersistentVolume, name, client, op)
            }
            K8sResourceAddress::Role(namespace, name) => {
                create_delete_patch!(Role, namespace, name, client, op)
            }
            K8sResourceAddress::RoleBinding(namespace, name) => {
                create_delete_patch!(RoleBinding, namespace, name, client, op)
            }
            K8sResourceAddress::ClusterRole(name) => {
                create_delete_patch!(ClusterRole, name, client, op)
            }
            K8sResourceAddress::ClusterRoleBinding(name) => {
                create_delete_patch!(ClusterRoleBinding, name, client, op)
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

        Ok(output)
    }
}
