use std::{
    collections::HashMap,
    future::{ready, Ready},
    pin::Pin,
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
};

use actix_http::body::{EitherBody, MessageBody};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest, HttpMessage,
};
use alice_architecture::jwt_payload::Payload;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use futures_util::{future::LocalBoxFuture, Future, TryFutureExt};
use jsonwebtoken::{
    decode as jwt_decode,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, TokenData, Validation,
};
use reqwest::Client as ReqwestClient;
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    config::ResourceControlConfig,
    data::Database,
    error::{AliceCommonError, AliceError},
};

#[derive(Default, Debug, Clone)]
pub struct AliceScopedConfig {
    pub user_info: Option<UserInfo>,
    pub device_info: Option<DeviceInfo>,
    pub task_info: Option<TaskInfo>,
}

impl FromRequest for AliceScopedConfig {
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_http::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let user_info = req.extensions().get::<UserInfo>().cloned();
            let device_info = req.extensions().get::<DeviceInfo>().cloned();
            let task_info = req.extensions().get::<TaskInfo>().cloned();

            // tracing::info!("user_info: {user_info:?}, device_info: {device_info:?}");
            Ok(AliceScopedConfig {
                user_info,
                device_info,
                task_info,
            })
        })
    }
}

// The authorization middleware insert the UserInfo or DeviceInfo to header extension, other
// crates take it on demand.

#[derive(Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub id: Uuid,
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: Uuid,
}

impl TaskInfo {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}

impl UserInfo {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub preferred_username: String,
}

impl From<Payload> for UserInfo {
    fn from(payload: Payload) -> Self {
        Self { id: payload.sub }
    }
}

impl From<Payload> for DeviceInfo {
    fn from(payload: Payload) -> Self {
        Self {
            id: payload.sub,
            preferred_username: payload.preferred_username,
        }
    }
}

#[async_trait]
pub trait UserIdRepository: Send + Sync {
    async fn get_by_task_id(&self, task_id: &str) -> anyhow::Result<Uuid>;
}

pub struct JwtValidationMiddleware {
    key_storage: Arc<dyn KeyStorage>,
    config: crate::config::JwtValidationConfig,
    all_controllers: bool,
    resources_config: ResourceControlConfig,
    database: Arc<Database>,
}

impl JwtValidationMiddleware {
    pub fn new(
        key_storage: Arc<dyn KeyStorage>,
        config: crate::config::JwtValidationConfig,
        resources_config: ResourceControlConfig,
        database: Arc<Database>,
    ) -> Self {
        Self {
            key_storage,
            config,
            all_controllers: false,
            resources_config,
            database,
        }
    }

    pub fn all_controllers(mut self) -> Self {
        self.all_controllers = true;
        self
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtValidationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtValidationMiddlewareExcutor<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtValidationMiddlewareExcutor {
            service: Rc::new(service),
            key_storage: self.key_storage.clone(),
            config: self.config.clone(),
            all_controllers: self.all_controllers,
            resources_config: self.resources_config.clone(),
            database: self.database.clone(),
        }))
    }
}

pub struct JwtValidationMiddlewareExcutor<S> {
    service: Rc<S>,
    key_storage: Arc<dyn KeyStorage>,
    config: crate::config::JwtValidationConfig,
    all_controllers: bool,
    resources_config: ResourceControlConfig,
    database: Arc<Database>,
}

impl<S> Clone for JwtValidationMiddlewareExcutor<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            key_storage: self.key_storage.clone(),
            config: self.config.clone(),
            all_controllers: self.all_controllers,
            resources_config: self.resources_config.clone(),
            database: self.database.clone(),
        }
    }
}

impl<S, B> Service<ServiceRequest> for JwtValidationMiddlewareExcutor<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let key_storage = self.key_storage.clone();
        let config = self.config.clone();
        let all_controllers = self.all_controllers;
        let resources_config = self.resources_config.clone();
        let database = self.database.clone();

        let protected_prefixs = [""];
        Box::pin(async move {
            let req_path = req.path();
            let flag =
                protected_prefixs.iter().any(|el| req_path.starts_with(el)) || all_controllers;
            if !flag {
                return Ok(service.call(req).await?.map_into_left_body());
            }

            let header = match req.headers().get("Authorization").ok_or::<Error>(
                AliceError::new(AliceCommonError::InvalidToken {
                    error_description: "No Authorization header.".to_string(),
                })
                .into(),
            ) {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ServiceResponse::from_err(e, req.request().to_owned())
                        .map_into_right_body());
                }
            };

            let header = match header.to_str().map_err(|e| {
                AliceError::new(AliceCommonError::InvalidToken {
                    error_description: e.to_string(),
                })
            }) {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ServiceResponse::from_err(e, req.request().to_owned())
                        .map_into_right_body());
                }
            };

            let payload = match parse_jwt_token_payload(header, key_storage, config).await {
                Ok(p) => p,
                Err(e) => {
                    return Ok(ServiceResponse::from_err(e, req.request().to_owned())
                        .map_into_right_body());
                }
            };

            let resource_access = &payload.resource_access;
            let option = resources_config.examine_permission(resource_access);
            let e_403 = AliceError::new(AliceCommonError::InsufficientScope {
                error_description: format!("Token doesn't have permission to access {req_path}"),
            });

            if option.is_none() {
                return Ok(ServiceResponse::from_err(e_403, req.request().to_owned())
                    .map_into_right_body());
            }
            let (allowed_paths, is_hpc) = option.unwrap();
            let has_permission = allowed_paths.iter().any(|prefix| req_path.starts_with(prefix));
            if !has_permission {
                return Ok(ServiceResponse::from_err(e_403, req.request().to_owned())
                    .map_into_right_body());
            }

            let mut info_user_id = None;
            let mut info_device_name = None;
            let mut info_task_id = None;

            if is_hpc {
                if let Some(task_id) = req.headers().get("TaskId") {
                    let (user_id, task_id) = match get_user_and_task_id(database, task_id)
                        .await
                        .map_err(|e| AliceError::new(AliceCommonError::InternalError { source: e }))
                    {
                        Ok(id) => id,
                        Err(e) => {
                            return Ok(ServiceResponse::from_err(e, req.request().to_owned())
                                .map_into_right_body());
                        }
                    };
                    req.extensions_mut().insert(UserInfo::new(user_id));
                    req.extensions_mut().insert(TaskInfo::new(task_id));
                    info_user_id = Some(user_id);
                    info_task_id = Some(task_id);
                }
                info_device_name = Some(payload.preferred_username.to_owned());
                req.extensions_mut().insert(DeviceInfo::from(payload));
            } else {
                info_user_id = Some(payload.sub);
                req.extensions_mut().insert(UserInfo::from(payload));
            }
            let mut info_msg = String::new();
            info_msg.push_str(&format!("Handling {req_path}"));
            if let Some(user_id) = info_user_id {
                info_msg.push_str(&format!(", user_id: {user_id}"));
            }
            if let Some(device_name) = info_device_name {
                info_msg.push_str(&format!(", device_name: {device_name}"));
            }
            if let Some(task_id) = info_task_id {
                info_msg.push_str(&format!(", task_id: {task_id}"));
            }

            tracing::info!("{info_msg}");
            service.call(req).map_ok(|res| res.map_into_left_body()).await
        })
    }
}

async fn get_user_and_task_id(
    database: Arc<Database>,
    task_id: &actix_http::header::HeaderValue,
) -> anyhow::Result<(Uuid, Uuid)> {
    let task_id = task_id.to_str()?.parse::<Uuid>()?;
    let con = database.get_connection();
    let node_id = database_model::prelude::Task::find_by_id(task_id)
        .one(con)
        .await?
        .with_context(|| format!("No such task: {task_id} in jwt validation.",))?
        .node_instance_id;
    let flow_id = database_model::prelude::NodeInstance::find_by_id(node_id)
        .one(con)
        .await?
        .with_context(|| format!("No such node: {node_id} in jwt validation."))?
        .flow_instance_id;
    Ok((
        database_model::prelude::FlowInstance::find_by_id(flow_id)
            .one(con)
            .await?
            .with_context(|| format!("No such flow-instance: {flow_id} in jwt validation"))?
            .user_id,
        task_id,
    ))
}

async fn parse_jwt_token_payload(
    authorization_str: &str,
    key_storage: Arc<dyn KeyStorage>,
    config: crate::config::JwtValidationConfig,
) -> Result<alice_architecture::jwt_payload::Payload, AliceError> {
    let parts = authorization_str.split_whitespace().collect::<Vec<&str>>();
    if parts.is_empty() {
        return Err(AliceError::new(AliceCommonError::InvalidToken {
            error_description: "Authorization header can't be splited by space.".to_string(),
        }));
    }
    if parts.len() < 2 && !parts[0].eq("Bearer") {
        return Err(AliceError::new(AliceCommonError::InvalidToken {
            error_description: "Authorization header doesn't have 'Baerer' str.".to_string(),
        }));
    }
    let token = match parts.get(1) {
        Some(t) => *t,
        None => {
            return Err(AliceError::new(AliceCommonError::InvalidToken {
                error_description: "Authorization header doesn't have token str.".to_string(),
            }));
        }
    };
    let mut validation = Validation::default();

    validation.aud = config.aud.clone();
    validation.leeway = config.leeway;
    validation.validate_exp = config.validate_exp;
    validation.validate_nbf = config.validate_nbf;
    validation.required_spec_claims = config.required_spec_claims.clone();
    validation.iss = config.iss.clone();
    let mut insecure_validation = validation.clone();
    insecure_validation.insecure_disable_signature_validation();

    let payload: TokenData<Payload> = jwt_decode(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &insecure_validation,
    )?;

    let header = payload.header;
    let payload = payload.claims;

    let kid = header.kid.ok_or(AliceError::new(AliceCommonError::InvalidToken {
        error_description: "No kid in token.".to_string(),
    }))?;

    let jwk_set = match key_storage.get(&payload.iss).await {
        Ok(x) => x,
        Err(e) => {
            tracing::warn!("Get jwk first try failed, take second try. - {e}");
            key_storage.reload_keys(&payload.iss).await?
        }
    };
    let jwk_set = MemoryKeyStorage::remove_unsupported_key(&jwk_set)?;
    let jwk_set: JwkSet = serde_json::from_str(&jwk_set)?;
    let key = match jwk_set.keys.iter().find(|x| match &x.common.key_id {
        Some(id) => id.eq(&kid),
        None => false,
    }) {
        Some(k) => k.to_owned(),
        None => {
            let jwk_set = key_storage.reload_keys(&payload.iss).await?;
            let jwk_set = MemoryKeyStorage::remove_unsupported_key(&jwk_set)?;
            let jwk_set: JwkSet = serde_json::from_str(&jwk_set)?;
            jwk_set
                .keys
                .iter()
                .find(|jwk| match &jwk.common.key_id {
                    Some(id) => id.eq(&kid),
                    None => false,
                })
                .ok_or(anyhow::anyhow!("Public key isn't matched."))?
                .to_owned()
        }
    };

    let key = match key.algorithm {
        AlgorithmParameters::RSA(ref params) => {
            DecodingKey::from_rsa_components(&params.n, &params.e)?
        }
        AlgorithmParameters::EllipticCurve(ref params) => {
            let x_cmp = general_purpose::STANDARD.decode(&params.x)?;
            let y_cmp = general_purpose::STANDARD.decode(&params.y)?;

            let mut public_key = Vec::with_capacity(1 + params.x.len() + params.y.len());
            public_key.push(0x04);
            public_key.extend_from_slice(&x_cmp);
            public_key.extend_from_slice(&y_cmp);
            DecodingKey::from_ec_der(&public_key)
        }
        AlgorithmParameters::OctetKeyPair(ref params) => {
            let x_decoded = general_purpose::STANDARD.decode(&params.x)?;
            DecodingKey::from_ed_der(&x_decoded)
        }
        AlgorithmParameters::OctetKey(ref params) => {
            DecodingKey::from_base64_secret(&params.value)?
        }
    };

    validation.algorithms = vec![Algorithm::RS256];

    let _ = jwt_decode::<Payload>(token, &key, &validation)?;

    Ok(payload)
}

#[derive(serde::Deserialize)]
pub struct WellKnownResponse {
    pub jwks_uri: String,
}

#[async_trait::async_trait]
pub trait KeyStorage: Send + Sync {
    async fn get(&self, iss: &str) -> anyhow::Result<String>;
    async fn reload_keys(&self, iss: &str) -> anyhow::Result<String>;
}

pub struct MemoryKeyStorage {
    storage: Arc<Mutex<HashMap<String, String>>>,
    http_client: Arc<ReqwestClient>,
}

impl MemoryKeyStorage {
    pub fn new(
        storage: Arc<Mutex<HashMap<String, String>>>,
        http_client: Arc<ReqwestClient>,
    ) -> Self {
        Self {
            storage,
            http_client,
        }
    }

    fn remove_unsupported_key(jwk_set: &str) -> anyhow::Result<String> {
        let mut jwk_set: Value = serde_json::from_str(jwk_set)?;
        let jwk_set_obj = jwk_set.as_object_mut().ok_or(anyhow::anyhow!("JwkSet isn't object."))?;
        let keys_value = jwk_set_obj
            .get_mut("keys")
            .ok_or(anyhow::anyhow!("No keys in JwkSet."))?
            .as_array_mut()
            .ok_or(anyhow::anyhow!("Keys isn't array."))?;
        let mut new_keys_value = vec![];
        for key in keys_value.iter() {
            let alg = key
                .as_object()
                .ok_or(anyhow!("Key isn't object."))?
                .get("alg")
                .ok_or(anyhow!("No alg in key."))?
                .as_str()
                .ok_or(anyhow!("Alg isn't string."))?;
            // `HS256`, `HS384`, `HS512`, `ES256`, `ES384`, `RS256`, `RS384`, `RS512`, `PS256`, `PS384`, `PS512`, `EdDSA`
            match alg {
                "HS256" | "HS384" | "HS512" | "ES256" | "ES384" | "RS256" | "RS384" | "RS512"
                | "PS256" | "PS384" | "PS512" | "EdDSA" => {
                    new_keys_value.push(key.clone());
                }
                _ => {}
            }
        }
        *keys_value = new_keys_value;

        Ok(jwk_set.to_string())
    }

    async fn insert(&self, iss: &str, jwk_set: &str) -> anyhow::Result<()> {
        let mut storage =
            self.storage.lock().map_err(|_| anyhow::anyhow!("Unable to lock storage."))?;
        storage.insert(iss.to_string(), jwk_set.to_string());
        Ok(())
    }
}

#[async_trait::async_trait]
impl KeyStorage for MemoryKeyStorage {
    async fn get(&self, iss: &str) -> anyhow::Result<String> {
        let storage =
            self.storage.lock().map_err(|_| anyhow::anyhow!("Unable to lock storage."))?;
        match storage.get(iss) {
            Some(x) => Ok(x.clone()),
            None => anyhow::bail!("No such key."),
        }
    }

    async fn reload_keys(&self, iss: &str) -> anyhow::Result<String> {
        let iss_well_known_url =
            url::Url::from_str(&format!("{iss}/"))?.join(".well-known/openid-configuration")?;
        let http_client = &self.http_client;
        let well_known: WellKnownResponse =
            http_client.get(iss_well_known_url).send().await?.json().await?;
        let jwk_set = http_client.get(well_known.jwks_uri).send().await?.text().await?;
        self.insert(iss, jwk_set.as_str()).await?;
        Ok(jwk_set)
    }
}
