use uuid::Uuid;

use crate::model::AggregateRoot;

#[async_trait::async_trait]
pub trait LeaseRepository<T>: Send + Sync
where
    T: AggregateRoot,
{
    async fn update_with_lease(&self, key: &str, entity: &T, ttl: i64) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn insert_with_lease(&self, key: &str, entity: &T, ttl: i64) -> anyhow::Result<Uuid> {
        unimplemented!()
    }

    async fn keep_alive(&self, key: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
pub trait ReadOnlyRepository<T>: Send + Sync
where
    T: AggregateRoot,
{
    async fn get_by_id(&self, uuid: Uuid) -> anyhow::Result<T> {
        unimplemented!()
    }

    async fn get_all(&self) -> anyhow::Result<Vec<T>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
pub trait MutableRepository<T>: Send + Sync
where
    T: AggregateRoot,
{
    async fn update(&self, entity: &T) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn insert(&self, entity: &T) -> anyhow::Result<Uuid> {
        unimplemented!()
    }

    async fn delete(&self, entity: &T) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn delete_by_id(&self, uuid: Uuid) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn insert_list(&self, entities: &[&T]) -> anyhow::Result<Vec<Uuid>> {
        unimplemented!()
    }

    async fn save_changed(&self) -> anyhow::Result<bool> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
pub trait DBRepository<T>: ReadOnlyRepository<T> + MutableRepository<T>
where
    T: AggregateRoot,
{
}

#[async_trait::async_trait]
pub trait LeaseDBRepository<T>: DBRepository<T> + LeaseRepository<T>
where
    T: AggregateRoot,
{
}
