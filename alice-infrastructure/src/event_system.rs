use alice_architecture::{
    event_system::{
        model::EventInfo, repository::EventHandlerRepo, Event, EventHandler, EventRouter,
    },
    repository::MutableRepository,
};
use reqwest::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct AliceEventRouter {
    http_client: Arc<Client>,
    event_repo: Arc<dyn MutableRepository<EventInfo>>,
    event_handler_repo: Arc<dyn EventHandlerRepo>,
}

impl AliceEventRouter {
    pub fn new(
        http_client: Arc<Client>,
        event_repo: Arc<dyn MutableRepository<EventInfo>>,
        event_handler_repo: Arc<dyn EventHandlerRepo>,
    ) -> Self {
        Self {
            http_client,
            event_repo,
            event_handler_repo,
        }
    }
}

#[async_trait::async_trait]
impl EventRouter for AliceEventRouter {
    async fn register(&self, info: Arc<dyn EventHandler>) -> anyhow::Result<()> {
        self.event_handler_repo.insert(&info.into()).await?;
        self.event_handler_repo.save_changed().await?;
        Ok(())
    }

    async fn dispatch(&self, event: Arc<dyn Event>) -> anyhow::Result<()> {
        let event: EventInfo = event.try_into()?;
        // save event
        self.event_repo.insert(&event).await?;
        self.event_repo.save_changed().await?;

        // get event handler
        let event_handler_post_urls = self
            .event_handler_repo
            .get_all_by_event_type(&event.r#type)
            .await?
            .iter()
            .map(|el| el.post_url.to_owned())
            .collect::<Vec<_>>();
        for url in event_handler_post_urls {
            let resp = self.http_client.post(url).json(&event.data).send().await?;
            if let Err(e) = resp.error_for_status() {
                log::error!("Error when dispatch event: {}, time: {}", e, event.time);
            }
        }
        Ok(())
    }
}
