//! Shared JSON-RPC 2.0 framing and client for WebSocket bridges.
pub mod error;
pub mod types;

pub use error::RpcError;
pub use types::{Request, Response, RpcErrorBody};
