pub mod model;
pub mod repository;

use std::sync::Arc;

pub trait Event: Send + Sync {
    fn r#type(&self) -> &str;
    fn data(&self) -> anyhow::Result<&str>;
}

#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, data: &str) -> anyhow::Result<()>;
    fn handle_types(&self) -> &[String];
    fn post_url(&self) -> &str;
    fn metadata(&self) -> Option<String>;
}

/// Event router, register and dispatch all kinds of events.
#[async_trait::async_trait]
pub trait EventRouter: Send + Sync {
    /// Register a event handler
    async fn register(&self, handler: Arc<dyn EventHandler>) -> anyhow::Result<()>;
    /// Dispatch a event
    async fn dispatch(&self, event: Arc<dyn Event>) -> anyhow::Result<()>;
}
