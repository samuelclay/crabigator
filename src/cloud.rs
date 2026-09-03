//! Cloud integration for streaming sessions to the configured service
//!
//! This module provides:
//! - Device identity management (device_id, device_secret)
//! - CloudClient for registering sessions and streaming events
//! - WebSocket connection for bidirectional communication
//! - Offline queue for when cloud is unreachable

mod client;
mod device;
mod endpoints;
mod events;
mod queue;
mod websocket;

pub use client::{
    add_watched_pr_standalone, fetch_pr_board_standalone, fetch_pr_overrides_standalone,
    fetch_watched_prs_standalone, relay_watched_pr_stats_standalone, CloudBoard, CloudClient,
    CloudStatus, CloudWatchedPr, PairingSnapshot, PairingStatusResponse,
};
pub use device::DeviceIdentity;
pub use endpoints::{print_cloud_status, reset_cloud, set_cloud, CloudEndpoints};
pub use events::{KeyStep, SessionEventBuilder};
