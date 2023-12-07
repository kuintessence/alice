use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use alice_architecture::background_service::BackgroundService;
use alice_architecture::message_queue::producer::{
    MessageQueueProducer, MessageQueueProducerTemplate,
};
use tokio::runtime::Handle;
use tracing::Instrument;

pub type ConsumerReturn<'async_fn> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'async_fn>>;
pub type ConsumerFn<SP> = for<'async_fn> fn(content: &'async_fn str, sp: Arc<SP>) -> ConsumerReturn;

#[derive(Debug, Clone)]
pub struct InternalMessage {
    pub target: String,
    pub body: String,
}

pub struct InternalMessageQueueProducer {
    receiver: flume::Receiver<InternalMessage>,
    sender: flume::Sender<InternalMessage>,
}

#[async_trait::async_trait]
impl MessageQueueProducer for InternalMessageQueueProducer {
    async fn send(&self, content: &str, topic:&str) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send_async(InternalMessage {
                target: topic.to_string(),
                body: content.to_string(),
            })
            .await?)
    }
}

#[async_trait::async_trait]
impl<T> MessageQueueProducerTemplate<T> for InternalMessageQueueProducer
where
    T: serde::Serialize + Send + Sync,
{
    async fn send_object(&self, content: &T, topic: &str) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send_async(InternalMessage {
                target: topic.to_string(),
                body: serde_json::to_string(content)?,
            })
            .await?)
    }
}

impl Default for InternalMessageQueueProducer {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalMessageQueueProducer {
    pub fn new() -> Self {
        let (sender, receiver): (
            flume::Sender<InternalMessage>,
            flume::Receiver<InternalMessage>,
        ) = flume::unbounded();
        Self { sender, receiver }
    }

    pub fn get_receiver(&self) -> flume::Receiver<InternalMessage> {
        self.receiver.clone()
    }
}

pub struct InternalMessageQueueConsumer<SP>
where
    SP: Send + Sync + 'static,
{
    receiver: flume::Receiver<InternalMessage>,
    service_provider: Arc<SP>,
    fn_mapper: HashMap<String, ConsumerFn<SP>>,
}

#[async_trait::async_trait]
impl<SP> BackgroundService for InternalMessageQueueConsumer<SP>
where
    SP: Send + Sync + 'static,
{
    async fn run(&self) {
        loop {
            match self.receiver.recv_async().await {
                Ok(message) => {
                    tracing::debug!("message received: {message:#?}");
                    match self.fn_mapper.get(&message.target) {
                        Some(x) => {
                            let sp = self.service_provider.clone();
                            let x = *x;
                            tokio::task::block_in_place(move || {
                                Handle::current().block_on(
                                    async move {
                                        if let Err(e) = x(message.body.as_str(), sp.clone()).await {
                                            tracing::error!("{e}")
                                        }
                                    }
                                    .instrument(tracing::trace_span!("internal_message_queue")),
                                )
                            });
                        }
                        None => tracing::error!("No such service: {}", message.target),
                    }
                }
                Err(e) => tracing::error!("{e}"),
            }
        }
    }
}

impl<SP> InternalMessageQueueConsumer<SP>
where
    SP: Send + Sync + 'static,
{
    pub fn new(
        receiver: flume::Receiver<InternalMessage>,
        service_provider: Arc<SP>,
        fn_mapper: HashMap<String, ConsumerFn<SP>>,
    ) -> Self {
        Self {
            receiver,
            service_provider,
            fn_mapper,
        }
    }
}
