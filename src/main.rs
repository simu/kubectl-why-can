use anyhow::anyhow;
use clap::Parser;

use k8s_openapi::api::authorization::v1::{
    FieldSelectorAttributes, LabelSelectorAttributes, ResourceAttributes, SelfSubjectAccessReview,
    SelfSubjectAccessReviewSpec,
};

use kube::{api::ObjectMeta, Api, Client, Config};

use tracing_subscriber::{prelude::*, EnvFilter};

fn create_self_subject_access_review(
    group: Option<String>,
    name: Option<String>,
    namespace: Option<String>,
    resource: Option<String>,
    subresource: Option<String>,
    verb: Option<String>,
    version: Option<String>,
    field_selector: Option<FieldSelectorAttributes>,
    label_selector: Option<LabelSelectorAttributes>,
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
                field_selector,
                label_selector,
            }),
            non_resource_attributes: None,
        },
        status: None,
    }
}

#[derive(Parser, Debug)]
#[clap(name = "kubectl-why-can")]
#[clap(bin_name = "kubectl-why-can")]
#[clap(version)]
struct Cli {
    /// Principal for SubjectAccessReview. Currently only `i` is supported
    principal: String,
    /// Verb for SelfSubjectAccessReview
    verb: String,
    /// Resource (or Resource/Name) for SelfSubjectAccessReview
    resource: String,
    /// Namespace for SelfSubjectAccessReview
    #[clap(short = 'n', long = "namespace")]
    namespace: Option<String>,
    /// All namespaces
    #[clap(short = 'A', long = "all-namespaces")]
    all_namespaces: bool,
    /// User to impersonate for the SelfSubjectAccessReview
    #[clap(long = "as")]
    impersonate: Option<String>,
    /// List of groups to impersonate for the SelfSubjectAccessReview
    #[clap(long = "as-group")]
    impersonate_groups: Option<Vec<String>>,
}

impl Cli {
    fn parse_resource(&self) -> anyhow::Result<(String, String, Option<String>)> {
        let resname = self
            .resource
            .split('/')
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        if resname.len() > 2 {
            return Err(anyhow!(
                "Expected only resource type or RESOURCE/NAME, got {}",
                self.resource
            ));
        }
        let name = if resname.len() > 1 {
            resname[1].clone()
        } else {
            "".to_string()
        };
        let resparts = resname[0]
            .split(".")
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        let (resource, group) = resparts
            .split_first()
            .ok_or(anyhow!("Can't split resource and group: {}", self.resource))?;
        let group = group.join(".");
        return Ok((
            resource.clone(),
            name,
            if group != "" { Some(group) } else { None },
        ));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_env("KUBECTL_WHY_CAN_LOG"))
        .init();

    // set process-wide default crypto provider to the rustls aws-lc implementation.
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();

    let args = Cli::parse();

    if args.principal != "i" {
        return Err(anyhow!("Currently, only `i` is supported as principal."));
    }

    let mut config = Config::infer().await?;
    if args.impersonate_groups.is_some() && args.impersonate.is_none() {
        return Err(anyhow!("--as-group is set, but --as is not set"));
    }
    if args.impersonate.is_some() {
        config.auth_info.impersonate = args.impersonate.clone();
        config.auth_info.impersonate_groups = args.impersonate_groups.clone();
    }
    let client = Client::try_from(config.clone())?;

    let (resource, name, group) = args.parse_resource()?;

    // Configure namespace to perform the SelfSubjectAccessReview for
    let ns = if args.all_namespaces {
        Some("*".to_string())
    } else if let Some(n) = args.namespace {
        Some(n.clone())
    } else {
        // use current context namespace
        Some(config.default_namespace.clone())
    };

    let sar_data = create_self_subject_access_review(
        group,
        Some(name),
        ns,
        Some(resource),
        None,
        Some(args.verb.clone()),
        None,
        None,
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

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
