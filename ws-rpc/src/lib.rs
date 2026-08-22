//! Shared JSON-RPC 2.0 framing and client for WebSocket bridges.
pub mod client;
pub mod error;
pub mod types;

pub use client::RpcClient;
pub use error::RpcError;
pub use types::{Request, Response, RpcErrorBody};
