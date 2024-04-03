use std::collections::BTreeMap;

use futures::TryFutureExt;
use k8s_openapi::api::core::v1::{
    PersistentVolumeClaim, PersistentVolumeClaimSpec, ResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client, Error};

pub async fn delete_storage(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace);
    api.delete(name, &Default::default()).map_ok(|_| ()).await
}

pub fn volume_claim_template(name: &str, gi: usize) -> PersistentVolumeClaim {
    let mut worker_labels: BTreeMap<String, Quantity> = BTreeMap::new();
    worker_labels.insert("storage".to_owned(), Quantity(format!("{gi}Gi")));

    PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
            resources: Some(ResourceRequirements {
                requests: Some(worker_labels),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
