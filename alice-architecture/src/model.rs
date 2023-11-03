#[cfg(feature = "derive")]
pub mod derive {
    pub use alice_architecture_derive::AggregateRoot;
}

pub trait AggregateRoot {}
