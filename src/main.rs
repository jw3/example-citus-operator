use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use kube::{
    api::{Api, ResourceExt},
    Client,
};
use kube::Resource;
use kube::runtime::Controller;
use kube::runtime::controller::Action;
use kube::runtime::watcher::Config;

use crate::cluster::{add_finalizer, delete_finalizer};
use crate::crd::CitusCluster;

mod cluster;
mod crd;
mod master;
mod workers;
mod jobs;

// use tracing::*;

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();
    let client = Client::try_default().await.expect("client config");
    let crd_api: Api<CitusCluster> = Api::all(client.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(client.clone()));

    Controller::new(crd_api.clone(), Config::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            match reconciliation_result {
                Ok(ccr) => {
                    println!("Reconciliation successful. Resource: {:?}", ccr);
                }
                Err(reconciliation_err) => {
                    eprintln!("Reconciliation error: {:?}", reconciliation_err)
                }
            }
        })
        .await;
}

struct ContextData {
    client: Client,
}

impl ContextData {
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

enum ClusterAction {
    Create,
    Delete,
    NoOp,
}

async fn reconcile(cc: Arc<CitusCluster>, context: Arc<ContextData>) -> Result<Action, Error> {
    let client: Client = context.client.clone(); // The `Client` is shared -> a clone from the reference is obtained
    let namespace: String = match cc.namespace() {
        None => {
            return Err(Error::UserInputError(
                "Expected namespaced resource.".to_owned(),
            ));
        }
        Some(namespace) => namespace,
    };
    let name = cc.name_any();
    match determine_action(&cc) {
        ClusterAction::Create => {
            add_finalizer(client.clone(), &name, &namespace).await?;
            cluster::deploy(client, &name, cc.spec.workers, &namespace).await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        ClusterAction::Delete => {
            cluster::delete(client.clone(), &name, &namespace).await?;
            delete_finalizer(client, &name, &namespace).await?;
            Ok(Action::await_change())
        }
        ClusterAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    }
}

fn determine_action(cc: &CitusCluster) -> ClusterAction {
    if cc.meta().deletion_timestamp.is_some() {
        ClusterAction::Delete
    } else if cc
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        ClusterAction::Create
    } else {
        ClusterAction::NoOp
    }
}

fn on_error(cc: Arc<CitusCluster>, error: &Error, _context: Arc<ContextData>) -> Action {
    eprintln!("Reconciliation error:\n{:?}.\n{:?}", error, cc);
    Action::requeue(Duration::from_secs(5))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("k8s error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("crd error: {0}")]
    UserInputError(String),
}
