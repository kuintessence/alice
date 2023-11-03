use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::model::AggregateRoot;

use super::{Event, EventHandler};

impl AggregateRoot for EventInfo {}

#[derive(Debug, Clone)]
pub struct EventInfo {
    pub r#type: String,
    pub time: DateTime<Utc>,
    pub data: String,
}

impl TryFrom<Arc<dyn Event>> for EventInfo {
    type Error = anyhow::Error;

    fn try_from(value: Arc<dyn Event>) -> Result<Self, Self::Error> {
        Ok(Self {
            r#type: value.r#type().to_owned(),
            time: Utc::now(),
            data: value.data()?.to_owned(),
        })
    }
}

impl AggregateRoot for EventHandlerInfo {}

pub struct EventHandlerInfo {
    /// Handler post url.
    pub post_url: String,
    /// Event types that the handler can handle.
    pub types: Vec<String>,
    /// Handler metadata.
    pub metadata: Option<String>,
}

impl From<Arc<dyn EventHandler>> for EventHandlerInfo {
    fn from(value: Arc<dyn EventHandler>) -> Self {
        Self {
            post_url: value.post_url().to_owned(),
            types: value.handle_types().to_owned(),
            metadata: value.metadata(),
        }
    }
}
