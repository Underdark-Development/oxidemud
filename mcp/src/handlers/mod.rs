//! Tool handler implementations grouped by domain.
//!
//! `server.rs` keeps the single `#[tool_router(server_handler)]` impl of thin
//! `#[tool]` wrappers; each wrapper delegates to the free functions here.

pub mod areas;
pub mod entities;
pub mod immortal;
pub mod simulation;
pub mod tools;
