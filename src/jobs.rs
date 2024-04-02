use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client, Error};
use kube::api::PostParams;

use crate::workers;

pub async fn register_workers(
    client: Client,
    name: &str,
    cnt: i32,
    namespace: &str,
) -> Result<Job, Error> {
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
                            format!("psql -c \"{}\"", (0..cnt)
                                .map(|i| {
                                    let wqname = workers::qname(name);
                                    format!(
                                        r#"SELECT * from master_add_node('{wqname}-{i}.{wqname}', 5432)"#
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
    jobs_api.create(&PostParams::default(), &init_job).await
}
