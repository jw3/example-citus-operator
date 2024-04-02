use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, StatefulSet, StatefulSetSpec};
use k8s_openapi::api::batch::v1::{Job, JobSpec, JobTemplateSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{Api, Client, Error};
use kube::api::{DeleteParams, ObjectMeta, Patch, PatchParams, PostParams};
use serde_json::{json, Value};

use crate::crd::CitusCluster;

pub type CitusDeployment = (Deployment, StatefulSet);

pub async fn deploy(
    client: Client,
    name: &str,
    workers: i32,
    namespace: &str,
) -> Result<CitusDeployment, Error> {
    let mut worker_labels: BTreeMap<String, String> = BTreeMap::new();
    worker_labels.insert("app".to_owned(), name.to_owned());

    let ss: StatefulSet = StatefulSet {
        metadata: ObjectMeta {
            name: Some(format!("{name}-workers")),
            namespace: Some(namespace.to_owned()),
            labels: Some(worker_labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(StatefulSetSpec {
            service_name: format!("{name}-worker"),
            replicas: Some(workers),
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

    let mut master_labels: BTreeMap<String, String> = BTreeMap::new();
    master_labels.insert("app".to_owned(), name.to_owned());
    master_labels.insert("node".to_owned(), "master".to_owned());

    let ss_api: Api<StatefulSet> = Api::namespaced(client.clone(), namespace);
    let ss = ss_api.create(&PostParams::default(), &ss).await?;

    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(format!("{name}-master")),
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
                        name: format!("{name}-master"),
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

    // Create the deployment defined above
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let m = deployment_api
        .create(&PostParams::default(), &deployment)
        .await?;

    let init_job = Job {
        metadata: ObjectMeta {
            generate_name: Some("working-init-".to_owned()),
            ..ObjectMeta::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    restart_policy: Some("OnFailure".to_owned()),
                    containers: vec![Container {
                        name: format!("{name}-init-worker"),
                        image: Some("citusdata/citus:12.1".to_owned()),
                        command: Some(vec![
                            "bash".to_owned(),
                            "-c".to_owned(),
                            format!("psql -c \"{}\"", (0..workers)
                                .map(|i| {
                                    format!(
                                        r#"SELECT * from master_add_node('my-citus-cluster-workers-{i}.my-citus-cluster-worker', 5432)"#
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(";")),
                        ]),
                        image_pull_policy: Some("IfNotPresent".to_owned()),
                        env: Some(vec![
                            EnvVar {
                                name: "PGHOST".to_owned(),
                                value: Some(format!("{name}")),
                                ..EnvVar::default()
                            },
                            EnvVar {
                                name: "PGUSER".to_owned(),
                                value: Some("postgres".to_owned()),
                                ..EnvVar::default()
                            },
                            EnvVar {
                                name: "PGPASSWORD".to_owned(),
                                value: Some("yourpassword".to_owned()),
                                ..EnvVar::default()
                            },
                        ]),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                ..PodTemplateSpec::default()
            },
            ..JobSpec::default()
        }),
        ..Job::default()
    };

    let jobs_api: Api<Job> = Api::namespaced(client.clone(), namespace);
    let job = jobs_api.create(&PostParams::default(), &init_job).await?;

    let mut master_selector_labels: BTreeMap<String, String> = BTreeMap::new();
    master_selector_labels.insert("node".to_owned(), "master".to_owned());

    let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

    let headless_svc = Service {
        metadata: ObjectMeta {
            name: Some(format!("{name}-worker")),
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
        .await?;

    let svc = Service {
        metadata: ObjectMeta {
            name: Some(format!("{name}")),
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
    service_api.create(&PostParams::default(), &svc).await?;

    Ok((m, ss))
}

pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    deployment_api
        .delete(&format!("{name}-master"), &DeleteParams::default())
        .await?;

    let api: Api<StatefulSet> = Api::namespaced(client, namespace);
    api.delete(&format!("{name}-workers"), &DeleteParams::default())
        .await?;

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
