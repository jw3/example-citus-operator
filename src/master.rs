use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{Api, Client, Error};
use kube::api::{DeleteParams, PostParams};

pub async fn deploy(client: Client, name: &str, namespace: &str) -> Result<Deployment, Error> {
    let mut master_labels: BTreeMap<String, String> = BTreeMap::new();
    master_labels.insert("app".to_owned(), name.to_owned());
    master_labels.insert("node".to_owned(), "master".to_owned());

    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_owned()),
            labels: Some(master_labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(master_labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: name.to_owned(),
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
                    labels: Some(master_labels.clone()),
                    ..ObjectMeta::default()
                }),
            },
            ..DeploymentSpec::default()
        }),
        ..Deployment::default()
    };

    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    deployment_api
        .create(&PostParams::default(), &deployment)
        .await
}

pub async fn expose(client: Client, name: &str, namespace: &str) -> Result<Service, Error> {
    let mut master_labels: BTreeMap<String, String> = BTreeMap::new();
    master_labels.insert("app".to_owned(), name.to_owned());
    master_labels.insert("node".to_owned(), "master".to_owned());

    let mut master_selector_labels: BTreeMap<String, String> = BTreeMap::new();
    master_selector_labels.insert("node".to_owned(), "master".to_owned());

    let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let svc = Service {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_owned()),
            labels: Some(master_labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(ServiceSpec {
            ports: Some(vec![ServicePort {
                name: Some("pg".to_owned()),
                port: 5432,
                target_port: Some(IntOrString::Int(5432)),
                ..ServicePort::default()
            }]),
            selector: Some(master_selector_labels),
            ..ServiceSpec::default()
        }),
        ..Service::default()
    };
    service_api.create(&PostParams::default(), &svc).await
}

pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let dparams = DeleteParams::default();
    deployment_api.delete(name, &dparams).await?;
    service_api.delete(name, &dparams).await?;

    Ok(())
}
