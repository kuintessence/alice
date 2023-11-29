use sea_orm::{ActiveValue, Value};
use uuid::Uuid;

use crate::model::AggregateRoot;

#[derive(Default)]
/// Item that used in entity update.
/// But because of Generic bound to T, so we need to use a item which implAggregateRoot as T.
pub enum DbField<T = ()> {
    Set(T),
    #[default]
    NotSet,
}

impl<T> From<DbField<T>> for ActiveValue<T>
where
    T: Into<Value>,
{
    fn from(value: DbField<T>) -> Self {
        match value {
            DbField::Set(v) => Self::Set(v),
            DbField::NotSet => Self::NotSet,
        }
    }
}

impl<T> DbField<T> {
    pub fn value(&self) -> anyhow::Result<&T> {
        match self {
            DbField::Set(v) => Ok(v),
            DbField::NotSet => Err(anyhow::anyhow!("DbField No value!")),
        }
    }
}

pub trait DbEntity {}

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
    T: AggregateRoot + Send + 'static,
{
    async fn update(&self, entity: &T::UpdateEntity) -> anyhow::Result<()> {
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

    async fn insert_list(&self, entities: &[T]) -> anyhow::Result<Vec<Uuid>> {
        unimplemented!()
    }

    async fn save_changed(&self) -> anyhow::Result<bool> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
pub trait DBRepository<T>: ReadOnlyRepository<T> + MutableRepository<T>
where
    T: AggregateRoot + Send + 'static,
{
}

#[async_trait::async_trait]
pub trait LeaseDBRepository<T>: DBRepository<T> + LeaseRepository<T>
where
    T: AggregateRoot + Send + 'static,
{
}
