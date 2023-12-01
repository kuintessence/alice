use num_traits::ToPrimitive;
use sea_orm::ActiveValue;
use serde::Serialize;
use uuid::Uuid;

use crate::model::AggregateRoot;

#[derive(Default)]
/// Item that used in entity update.
/// But because of Generic bound to T, so we need to use a item which implAggregateRoot as T.
pub enum DbField<T = ()> {
    Set(T),
    #[default]
    NotSet,
    Unchanged(T),
}

impl<T> From<DbField<T>> for ActiveValue<i32>
where
    T: ToPrimitive,
{
    fn from(value: DbField<T>) -> Self {
        match value {
            DbField::Set(v) => Self::Set(v.to_i32().unwrap()),
            DbField::NotSet => Self::NotSet,
            DbField::Unchanged(v) => Self::Unchanged(v.to_i32().unwrap()),
        }
    }
}

impl<T> From<DbField<Option<T>>> for ActiveValue<Option<i32>>
where
    T: ToPrimitive,
{
    fn from(value: DbField<Option<T>>) -> Self {
        match value {
            DbField::Set(o) => Self::Set(o.map(|v| v.to_i32().unwrap())),
            DbField::NotSet => Self::NotSet,
            DbField::Unchanged(o) => Self::Unchanged(o.map(|v| v.to_i32().unwrap())),
        }
    }
}

impl<T> TryFrom<DbField<Option<T>>> for ActiveValue<Option<serde_json::Value>>
where
    T: Serialize,
{
    type Error = serde_json::Error;
    fn try_from(value: DbField<Option<T>>) -> Result<Self, Self::Error> {
        Ok(match value {
            DbField::Set(o) => Self::Set(o.map(serde_json::to_value).transpose()?),
            DbField::NotSet => Self::NotSet,
            DbField::Unchanged(o) => Self::Unchanged(o.map(serde_json::to_value).transpose()?),
        })
    }
}

impl<T> TryFrom<DbField<T>> for ActiveValue<serde_json::Value>
where
    T: Serialize,
{
    type Error = serde_json::Error;
    fn try_from(value: DbField<T>) -> Result<Self, Self::Error> {
        Ok(match value {
            DbField::Set(v) => Self::Set(serde_json::to_value(v)?),
            DbField::NotSet => Self::NotSet,
            DbField::Unchanged(v) => Self::Unchanged(serde_json::to_value(v)?),
        })
    }
}

impl<T> DbField<T> {
    pub fn value(&self) -> anyhow::Result<&T> {
        match self {
            DbField::Set(v) => Ok(v),
            DbField::NotSet => Err(anyhow::anyhow!("DbField No value!")),
            DbField::Unchanged(v) => Ok(v),
        }
    }

    pub fn into_active_value(self) -> ActiveValue<T>
    where
        T: Into<sea_orm::Value>,
    {
        match self {
            DbField::Set(v) => ActiveValue::Set(v),
            DbField::NotSet => ActiveValue::NotSet,
            DbField::Unchanged(v) => ActiveValue::Unchanged(v),
        }
    }
}

pub trait DbEntity: Send + Sync + 'static {}

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
    async fn update(&self, entity: T::UpdateEntity) -> anyhow::Result<()> {
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
