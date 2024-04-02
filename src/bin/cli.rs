use clap::Parser;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::{Api, Client};

use example_citus_operator::crd::{CitusCluster, CitusClusterSpec};

#[derive(Clone, Debug, Parser)]
struct Opts {
    /// Name of the cluster
    name: String,

    #[clap(short, long, default_value = "default")]
    namespace: String,

    /// Number of workers
    #[clap(short, long, default_value = "1")]
    size: usize,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();
    let client = Client::try_default().await.expect("client config");
    let crd_api: Api<CitusCluster> = Api::namespaced(client.clone(), &opts.namespace);

    crd_api
        .create(
            &PostParams::default(),
            &CitusCluster {
                metadata: ObjectMeta {
                    name: Some(opts.name.to_owned()),
                    ..ObjectMeta::default()
                },
                spec: CitusClusterSpec {
                    workers: opts.size as i32,
                },
            },
        )
        .await
        .expect("create");
}
