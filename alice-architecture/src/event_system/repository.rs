use crate::repository::DBRepository;

use super::model::EventHandlerInfo;

#[async_trait::async_trait]
pub trait EventHandlerRepo: DBRepository<EventHandlerInfo> {
    async fn get_all_by_event_type(
        &self,
        event_type: &str,
    ) -> anyhow::Result<Vec<EventHandlerInfo>>;
}
