use clap::Parser;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::PostParams;

use example_citus_operator::crd::{CitusCluster, CitusClusterSpec};

#[derive(Clone, Debug, Parser)]
struct Opts {
    #[clap(subcommand)]
    command: Subcommand,

    #[clap(short, long, default_value = "default")]
    namespace: String,
}

#[derive(Clone, Debug, Parser)]
enum Subcommand {
    Create(CreateOpts),
    Delete(DeleteOpts),
}

#[derive(Clone, Debug, Parser)]
struct CreateOpts {
    /// Name of the cluster
    name: String,

    /// Number of workers
    #[clap(short, long, default_value = "1")]
    workers: usize,
}

#[derive(Clone, Debug, Parser)]
struct DeleteOpts {
    /// Name of the cluster
    name: String,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let client = Client::try_default().await.expect("client config");
    let crd_api: Api<CitusCluster> = Api::namespaced(client.clone(), &opts.namespace);

    match opts.command {
        Subcommand::Create(c) => {
            crd_api
                .create(
                    &PostParams::default(),
                    &CitusCluster {
                        metadata: ObjectMeta {
                            name: Some(c.name.to_owned()),
                            ..ObjectMeta::default()
                        },
                        spec: CitusClusterSpec {
                            workers: c.workers as i32,
                        },
                    },
                )
                .await
                .expect("create");
        }
        Subcommand::Delete(c) => {
            crd_api
                .delete(&c.name, &Default::default())
                .await
                .expect("delete");
        }
    }
}
