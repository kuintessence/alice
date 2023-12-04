pub mod config;

#[cfg(feature = "sea-orm-db")]
pub mod data;

#[cfg(feature = "event-system")]
pub mod event_system;

#[cfg(any(feature = "kafka-mq", feature = "flume-mq"))]
pub mod message_queue;

#[cfg(feature = "flume-mq")]
pub use message_queue::{ConsumerFn, ConsumerReturn};

#[cfg(feature = "error")]
pub mod error;

#[cfg(feature = "http-client")]
pub mod http_client;

#[cfg(feature = "actix-middleware")]
pub mod middleware;

#[cfg(feature = "telemetry")]
pub mod telemetry;
