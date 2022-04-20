// Minimal custom client example.
use k8s_openapi::api::authorization::v1::{
    ResourceAttributes, SelfSubjectAccessReview, SelfSubjectAccessReviewSpec,
};

use kube::{api::ObjectMeta, client::ConfigExt, Api, Client, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;
    let https = config.openssl_https_connector()?;
    let service = tower::ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(config.auth_layer()?)
        .service(hyper::Client::builder().build(https));
    let client = Client::new(service, config.default_namespace);

    let sar: Api<SelfSubjectAccessReview> = Api::all(client);
    let sar_data = SelfSubjectAccessReview {
        metadata: ObjectMeta::default(),
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: None,
                name: None,
                namespace: Some("".to_string()),
                resource: Some("pods".to_string()),
                subresource: None,
                verb: Some("get".to_string()),
                version: None,
            }),
            non_resource_attributes: None,
        },
        status: None,
    };

    let sar_resp = sar.create(&Default::default(), &sar_data).await?;

    let resp_status = sar_resp.status.unwrap();
    if resp_status.allowed {
        println!("{}", resp_status.reason.unwrap())
    } else {
        println!("denied")
    }

    Ok(())
}
