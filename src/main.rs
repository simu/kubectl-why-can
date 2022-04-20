use anyhow::anyhow;
use clap::Parser;

use k8s_openapi::api::authorization::v1::{
    ResourceAttributes, SelfSubjectAccessReview, SelfSubjectAccessReviewSpec,
};

use kube::{api::ObjectMeta, client::ConfigExt, Api, Client, Config};

async fn create_client() -> anyhow::Result<Client> {
    let config = Config::infer().await?;
    let https = config.openssl_https_connector()?;
    let service = tower::ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(config.auth_layer()?)
        .service(hyper::Client::builder().build(https));
    Ok(Client::new(service, config.default_namespace))
}

fn create_self_subject_access_review(
    group: Option<String>,
    name: Option<String>,
    namespace: Option<String>,
    resource: Option<String>,
    subresource: Option<String>,
    verb: Option<String>,
    version: Option<String>,
) -> SelfSubjectAccessReview {
    SelfSubjectAccessReview {
        metadata: ObjectMeta::default(),
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group,
                name,
                namespace,
                resource,
                subresource,
                verb,
                version,
            }),
            non_resource_attributes: None,
        },
        status: None,
    }
}

#[derive(Parser, Debug)]
struct Cli {
    /// Verb for SelfSubjectAccessReview
    verb: String,
    /// Resource for SelfSubjectAccessReview
    resource: String,
    /// Resource name for SelfSubjectAccessReview
    name: Option<String>,
    /// Namespace for SelfSubjectAccessReview
    #[clap(short = 'n', long = "namespace")]
    namespace: Option<String>,
}

impl Cli {
    fn parse_resource(&self) -> anyhow::Result<(String, String)> {
        let resparts = self
            .resource
            .split(".")
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        let (resource, group) = resparts
            .split_first()
            .ok_or(anyhow!("Can't split resource and group: {}", self.resource))?;
        let group = if group.len() > 0 {
            group.join(".")
        } else {
            "".to_string()
        };
        return Ok((resource.clone(), group));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let client = create_client().await?;

    let (resource, group) = args.parse_resource()?;

    let sar_data = create_self_subject_access_review(
        Some(group),
        args.name,
        args.namespace,
        Some(resource),
        None,
        Some(args.verb.clone()),
        None,
    );
    let sar: Api<SelfSubjectAccessReview> = Api::all(client);
    let sar_resp = sar.create(&Default::default(), &sar_data).await?;

    let resp_status = sar_resp.status.unwrap();
    if resp_status.allowed {
        let reason = resp_status.reason.unwrap_or("No reason given".to_string());
        println!("Access allowed: {}", reason)
    } else {
        println!("{} {} not allowed", args.verb, args.resource)
    }

    Ok(())
}
