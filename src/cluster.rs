use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use kube::{Api, Client, Error};
use kube::api::{Patch, PatchParams};
use serde_json::{json, Value};

use crate::{jobs, master, workers};
use crate::crd::CitusCluster;

pub type CitusDeployment = (Deployment, StatefulSet);

pub async fn deploy(
    client: Client,
    name: &str,
    num_workers: i32,
    namespace: &str,
) -> Result<CitusDeployment, Error> {
    let master = master::deploy(client.clone(), name, namespace).await?;
    let workers = workers::deploy(client.clone(), name, num_workers, namespace).await?;
    jobs::register_workers(client.clone(), name, num_workers, namespace).await?;

    master::expose(client.clone(), name, namespace).await?;
    workers::expose(client.clone(), name, namespace).await?;

    Ok((master, workers))
}

pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    master::delete(client.clone(), name, namespace).await?;
    workers::delete(client.clone(), name, namespace).await?;

    Ok(())
}

pub async fn add_finalizer(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<CitusCluster, Error> {
    let api: Api<CitusCluster> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["citusclusters.jw3.xyz/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

pub async fn delete_finalizer(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<CitusCluster, Error> {
    let api: Api<CitusCluster> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });
    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}
