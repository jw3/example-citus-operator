use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
group = "jw3.xyz",
version = "v1alpha1",
kind = "CitusCluster",
plural = "citusclusters",
derive = "PartialEq",
namespaced
)]
pub struct CitusClusterSpec {
    pub workers: i32,
}
