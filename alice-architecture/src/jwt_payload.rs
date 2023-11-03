use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Default, Clone)]
pub struct Payload {
    pub iss: String,
    /// 用户 uuid
    pub sub: Uuid,
    /// 用户名
    pub preferred_username: String,
    /// Key: Client id
    /// Value: Resources to access.
    pub resource_access: HashMap<String, Value>,
}
