//! Cloud integration for streaming sessions to drinkcrabigator.com
//!
//! This module provides:
//! - Device identity management (device_id, device_secret)
//! - CloudClient for registering sessions and streaming events
//! - WebSocket connection for bidirectional communication
//! - Offline queue for when cloud is unreachable

mod client;
mod device;
mod events;
mod queue;
mod websocket;

pub use client::CloudClient;
pub use device::DeviceIdentity;
pub use events::{CloudEvent, SessionEventBuilder};
pub use queue::OfflineQueue;
