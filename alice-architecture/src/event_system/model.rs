use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{
    model::AggregateRoot,
    repository::{DbEntity, DbField},
};

use super::{Event, EventHandler};

impl AggregateRoot for EventInfo {
    type UpdateEntity = DbEventInfo;
}

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

impl AggregateRoot for EventHandlerInfo {
    type UpdateEntity = DbEventHandlerInfo;
}

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

impl DbEntity for DbEventInfo {}

pub struct DbEventInfo {
    pub r#type: DbField<String>,
    pub time: DbField<DateTime<Utc>>,
    pub data: DbField<String>,
}

impl DbEntity for DbEventHandlerInfo {}

pub struct DbEventHandlerInfo {
    /// Handler post url.
    pub post_url: DbField<String>,
    /// Event types that the handler can handle.
    pub types: DbField<Vec<String>>,
    /// Handler metadata.
    pub metadata: DbField<Option<String>>,
}

impl From<EventHandlerInfo> for DbEventHandlerInfo {
    fn from(value: EventHandlerInfo) -> Self {
        Self {
            post_url: DbField::Set(value.post_url),
            types: DbField::Set(value.types),
            metadata: DbField::Set(value.metadata),
        }
    }
}

impl From<EventInfo> for DbEventInfo {
    fn from(value: EventInfo) -> Self {
        Self {
            r#type: DbField::Set(value.r#type),
            time: DbField::Set(value.time),
            data: DbField::Set(value.data),
        }
    }
}
