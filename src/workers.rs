use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{Api, Client, Error};
use kube::api::{DeleteParams, PostParams};

pub async fn deploy(
    client: Client,
    name: &str,
    cnt: i32,
    namespace: &str,
) -> Result<StatefulSet, Error> {
    let mut worker_labels: BTreeMap<String, String> = BTreeMap::new();
    worker_labels.insert("app".to_owned(), name.to_owned());

    let ss: StatefulSet = StatefulSet {
        metadata: ObjectMeta {
            name: Some(qname(name)),
            namespace: Some(namespace.to_owned()),
            labels: Some(worker_labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(StatefulSetSpec {
            service_name: qname(name),
            replicas: Some(cnt),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(worker_labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "worker".to_owned(),
                        image: Some("citusdata/citus:12.1".to_owned()),
                        image_pull_policy: Some("IfNotPresent".to_owned()),
                        ports: Some(vec![ContainerPort {
                            container_port: 5432,
                            ..ContainerPort::default()
                        }]),
                        env: Some(vec![EnvVar {
                            name: "POSTGRES_PASSWORD".to_owned(),
                            value: Some("yourpassword".to_owned()),
                            ..EnvVar::default()
                        }]),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                metadata: Some(ObjectMeta {
                    labels: Some(worker_labels.clone()),
                    ..ObjectMeta::default()
                }),
            },
            ..StatefulSetSpec::default()
        }),
        ..StatefulSet::default()
    };

    let ss_api: Api<StatefulSet> = Api::namespaced(client.clone(), namespace);
    ss_api.create(&PostParams::default(), &ss).await
}

pub async fn expose(client: Client, name: &str, namespace: &str) -> Result<Service, Error> {
    let mut master_labels: BTreeMap<String, String> = BTreeMap::new();
    master_labels.insert("app".to_owned(), name.to_owned());
    master_labels.insert("node".to_owned(), "master".to_owned());

    let mut worker_labels: BTreeMap<String, String> = BTreeMap::new();
    worker_labels.insert("app".to_owned(), name.to_owned());

    let mut master_selector_labels: BTreeMap<String, String> = BTreeMap::new();
    master_selector_labels.insert("node".to_owned(), "master".to_owned());

    let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let headless_svc = Service {
        metadata: ObjectMeta {
            name: Some(qname(name)),
            namespace: Some(namespace.to_owned()),
            labels: Some(worker_labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(ServiceSpec {
            ports: Some(vec![ServicePort {
                name: Some("pg".to_owned()),
                port: 5432,
                target_port: Some(IntOrString::Int(5432)),
                ..ServicePort::default()
            }]),
            selector: Some(worker_labels),
            cluster_ip: None,
            ..ServiceSpec::default()
        }),
        ..Service::default()
    };

    service_api
        .create(&PostParams::default(), &headless_svc)
        .await
}

pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let ss_api: Api<StatefulSet> = Api::namespaced(client.clone(), namespace);
    let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let qname = qname(name);
    let dparams = DeleteParams::default();

    ss_api.delete(&qname, &dparams).await?;
    service_api.delete(&qname, &dparams).await?;

    Ok(())
}

pub(crate) fn qname(name: &str) -> String {
    format!("{name}-workers")
}
