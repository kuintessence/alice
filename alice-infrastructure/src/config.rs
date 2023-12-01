use serde::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default, Deserialize, Clone, Debug)]

pub struct CommonConfig {
    #[cfg(feature = "telemetry")]
    #[serde(default)]
    pub telemetry: crate::telemetry::TelemetryConfig,
    #[serde(default)]
    pub db: DatabaseConfig,
    #[serde(default)]
    pub host: HostConfig,
    #[serde(default)]
    pub mq: MessageQueueConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub jwt: JwtValidationConfig,
    #[serde(default)]
    pub http_client: HttpClientConfig,
}

#[derive(Deserialize, Clone, Debug)]

pub struct JwtValidationConfig {
    #[serde(default = "JwtValidationConfig::default_required_spec_claims")]
    pub required_spec_claims: HashSet<String>,
    #[serde(default = "JwtValidationConfig::default_leeway")]
    pub leeway: u64,
    #[serde(default = "JwtValidationConfig::default_validate_exp")]
    pub validate_exp: bool,
    #[serde(default = "JwtValidationConfig::default_validate_nbf")]
    pub validate_nbf: bool,
    #[serde(default = "JwtValidationConfig::default_aud")]
    pub aud: Option<HashSet<String>>,
    #[serde(default = "JwtValidationConfig::default_iss")]
    pub iss: Option<HashSet<String>>,
}

impl JwtValidationConfig {
    fn default_required_spec_claims() -> HashSet<String> {
        HashSet::from_iter(vec!["exp".to_string()])
    }
    fn default_leeway() -> u64 {
        60
    }
    fn default_validate_exp() -> bool {
        true
    }
    fn default_validate_nbf() -> bool {
        false
    }
    fn default_aud() -> Option<HashSet<String>> {
        None
    }
    fn default_iss() -> Option<HashSet<String>> {
        None
    }
}

impl Default for JwtValidationConfig {
    fn default() -> Self {
        Self {
            required_spec_claims: HashSet::from_iter(vec!["exp".to_string()]),
            leeway: 60,
            validate_exp: true,
            validate_nbf: false,
            aud: None,
            iss: None,
        }
    }
}

/// Client id
type ClientId = String;
/// Role name
type Role = String;
/// Controller path name
type ControllerPathPrefix = String;
/// Allowed client and resource config
#[derive(Deserialize, Clone, Debug)]
pub struct ResourceControlConfig(
    #[serde(default = "ResourceControlConfig::default_0")]
    pub  HashMap<ClientId, HashMap<Role, Vec<ControllerPathPrefix>>>,
);

impl ResourceControlConfig {
    fn default_0() -> HashMap<ClientId, HashMap<Role, Vec<ControllerPathPrefix>>> {
        HashMap::from([
            (
                "fe".to_string(),
                HashMap::from([(
                    "user".to_string(),
                    [
                        "/workflow-engine".to_string(),
                        "/file-storage".to_string(),
                        "/workflow-editor".to_string(),
                        "/text-storage".to_string(),
                        "/usecase-editor".to_string(),
                    ]
                    .to_vec(),
                )]),
            ),
            (
                "device".to_string(),
                HashMap::from([(
                    "hpc".to_string(),
                    [
                        "/agent".to_string(),
                        "/workflow-engine/ReceiveTaskStatus".to_string(),
                        "/file-storage/PreparePartialUploadFromNodeInstance".to_string(),
                        "/file-storage/PreparePartialUploadFromSnapshot".to_string(),
                        "/file-storage/PartialUpload".to_string(),
                        "/file-storage/FileDownloadUrl".to_string(),
                        "/file-storage/FileDownloadUrls".to_string(),
                        "/file-storage/RangelyDownloadFile".to_string(),
                        "/file-storage/UploadRealTimeFile".to_string(),
                    ]
                    .to_vec(),
                )]),
            ),
        ])
    }

    /// Examine the payload resource_access, and return its allowed controller paths and whether it has a hpc client permission.
    /// If None, means this request is forbidden.
    pub fn examine_permission(
        &self,
        resource_access: &HashMap<String, Value>,
    ) -> Option<(Vec<ControllerPathPrefix>, bool)> {
        let control = &self.0;
        let mut paths = vec![];
        let mut is_hpc = false;
        // Traverse configed access control.
        for (client_id, role_to_allowed_controller_paths) in control.iter() {
            // Look for client access permissions from request
            let client_resource_access = match resource_access.get(client_id) {
                Some(a) => a,
                None => continue,
            };
            let client_owned_roles = match match client_resource_access.get("roles") {
                Some(c) => c,
                None => continue,
            }
            .as_array()
            {
                Some(c) => c,
                None => continue,
            };
            let client_owned_roles =
                client_owned_roles.iter().map(|v| v.as_str()).collect::<Option<Vec<_>>>()?;
            // let client_owned_roles = client_owned_roles?;
            for (role, allowed_paths) in role_to_allowed_controller_paths {
                if client_owned_roles.contains(&role.as_str()) {
                    paths.extend(allowed_paths.to_owned())
                };
            }
            if client_id.eq("device") && client_owned_roles.contains(&"hpc") {
                is_hpc = true;
            }
        }
        Some((paths, is_hpc))
    }
}

impl Default for ResourceControlConfig {
    fn default() -> Self {
        Self(Self::default_0())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct HostConfig {
    #[serde(default = "HostConfig::default_address")]
    pub bind_address: String,
    #[serde(default = "HostConfig::default_port")]
    pub bind_port: u16,
    #[serde(default = "HostConfig::default_upload_path")]
    pub upload_file_path: String,
    /// Config which controller path the client is able to access and what roles of this client can access.
    #[serde(default)]
    pub resources_config: ResourceControlConfig,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            bind_address: Self::default_address(),
            bind_port: Self::default_port(),
            upload_file_path: Self::default_upload_path(),
            resources_config: ResourceControlConfig::default(),
        }
    }
}
impl HostConfig {
    fn default_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_port() -> u16 {
        80
    }

    fn default_upload_path() -> String {
        "tempdir".to_string()
    }
}

#[derive(Default, Deserialize, Clone, Debug)]

pub struct MessageQueueConfig {
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub producer: HashMap<String, String>,
    #[serde(default)]
    pub consumer: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Debug)]

pub struct RedisConfig {
    #[serde(default = "RedisConfig::default_urls")]
    pub urls: Vec<String>,
    #[serde(default = "RedisConfig::default_exp_msecs")]
    pub exp_msecs: i64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            urls: Self::default_urls(),
            exp_msecs: Self::default_exp_msecs(),
        }
    }
}
impl RedisConfig {
    fn default_urls() -> Vec<String> {
        vec!["localhost:6379".to_string()]
    }
    fn default_exp_msecs() -> i64 {
        24 * 60 * 60 * 1000
    }
}

#[derive(Deserialize, Clone, Debug)]

pub struct DatabaseConfig {
    #[serde(default = "DatabaseConfig::default_url")]
    pub url: String,
}

impl DatabaseConfig {
    fn default_url() -> String {
        "postgres://postgres:postgrespassword@localhost:5432/system".to_string()
    }
}
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: Self::default_url(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct HttpClientConfig {
    #[serde(default = "Default::default")]
    pub http_header: HashMap<String, String>,
    #[serde(default = "HttpClientConfig::default_user_agent")]
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            http_header: Default::default(),
            user_agent: Self::default_user_agent(),
        }
    }
}

impl HttpClientConfig {
    pub fn default_user_agent() -> String {
        "COS/1.0".to_string()
    }
}

pub fn build_config() -> anyhow::Result<config::Config> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = config::Config::builder().add_source(
        config::File::with_name("config")
            .required(false)
            .format(config::FileFormat::Yaml),
    );
    for arg in args {
        if arg.ends_with("yaml") || arg.ends_with("yml") {
            config = config.add_source(
                config::File::from(std::path::Path::new(arg.as_str()))
                    .format(config::FileFormat::Yaml)
                    .required(false),
            );
        }
    }
    config = config.add_source(
        config::Environment::with_prefix("ALICE")
            .separator("__")
            .try_parsing(true)
            .list_separator(";")
            .with_list_parse_key("common.redis.urls"),
    );
    Ok(config.build()?)
}
