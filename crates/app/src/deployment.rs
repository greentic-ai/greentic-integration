use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    Application,
    Deployment,
    Mixed,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IaCCapabilities {
    /// Component may write IaC templates/manifests/plans to a preopened area.
    pub write_templates: bool,
    /// Component may request execution of generated plans via the host.
    #[serde(default)]
    pub execute_plans: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub pack_id: String,
    pub pack_version: Version,

    pub tenant: String,
    pub environment: String,

    pub runners: Vec<RunnerPlan>,
    pub messaging: Option<MessagingPlan>,
    pub channels: Vec<ChannelPlan>,
    pub secrets: Vec<SecretPlan>,
    pub oauth: Vec<OAuthPlan>,
    pub telemetry: Option<TelemetryPlan>,

    #[serde(default)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerPlan {
    pub name: String,
    pub replicas: u32,
    #[serde(default)]
    pub capabilities: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingPlan {
    pub logical_cluster: String,
    pub subjects: Vec<MessagingSubjectPlan>,
    #[serde(default)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingSubjectPlan {
    pub name: String,
    pub purpose: String,
    pub durable: bool,
    #[serde(default)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPlan {
    pub name: String,
    pub flow_id: String,
    pub kind: String,
    #[serde(default)]
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPlan {
    pub key: String,
    pub required: bool,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPlan {
    pub provider_id: String,
    pub logical_client_id: String,
    pub redirect_path: String,
    #[serde(default)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPlan {
    pub required: bool,
    pub suggested_endpoint: Option<String>,
    #[serde(default)]
    pub extra: Value,
}
