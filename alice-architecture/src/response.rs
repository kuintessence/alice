use std::collections::HashMap;
use std::fmt::{Debug, Display};

use serde::Serialize;

#[cfg(feature = "derive")]
pub mod derive {
    pub use alice_architecture_derive::I18NEnum;
}

pub trait Locale {
    fn text_with_args(&self, id: &str, args: HashMap<String, String>) -> anyhow::Result<String>;

    fn text(&self, id: &str) -> anyhow::Result<String>;
}

#[derive(Serialize)]
pub struct LocalizedMsg {
    pub status: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

pub trait I18NEnum: Display + Debug {
    fn localize(&self, locale: &dyn Locale) -> anyhow::Result<LocalizedMsg>;
    fn status(&self) -> u16;
}
