#[cfg(feature = "derive")]
pub use alice_architecture_derive::AggregateRoot;

use crate::repository::DbEntity;

pub trait AggregateRoot {
    type UpdateEntity: DbEntity;
}
