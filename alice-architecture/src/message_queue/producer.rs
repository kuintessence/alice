use serde::Serialize;

#[async_trait::async_trait]
pub trait MessageQueueProducer: Send + Sync {
    async fn send(&self, content: &str, topic: &str) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait MessageQueueProducerTemplate<T>: Send + Sync
where
    T: Serialize,
{
    async fn send_object(&self, content: &T, topic: &str) -> anyhow::Result<()>;
}
