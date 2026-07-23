pub mod api;
mod cmd;
pub mod config;
mod connection;
pub mod dispatch;
pub mod game_loop;
pub mod login;
pub mod persistence;
pub mod prompt;
pub mod registry;
mod server;
mod telnet;

pub use cmd::*;
pub use connection::*;
pub use game_loop::*;
pub use login::*;
pub use persistence::*;
pub use registry::*;
pub use server::*;
pub use telnet::*;

use oxide_core::Entity;

/// Implementation of core's MessageOutputBridge using the server's ConnectionRegistry
pub struct ServerMessageBridge;

impl oxide_core::MessageOutputBridge for ServerMessageBridge {
    fn send_to_entity(&self, entity: Entity, message: &str) {
        if let Some(registry_lock) = get_registry() {
            // Block on acquiring the lock synchronously
            if let Ok(reg) = registry_lock.try_lock() {
                if let Some(tx) = reg.sender(entity) {
                    let mut bytes = message.as_bytes().to_vec();
                    if !bytes.ends_with(b"\n") {
                        bytes.extend_from_slice(b"\r\n");
                    }
                    let _ = tx.send(bytes);
                }
            }
        }
    }

    fn echo_to_room(&self, room: Entity, message: &str) {
        if let Some(registry_lock) = get_registry() {
            if let Ok(reg) = registry_lock.try_lock() {
                if let Some(world_lock) = get_world() {
                    if let Ok(world) = world_lock.try_lock() {
                        let mut formatted = message.to_string();
                        if !formatted.ends_with('\n') {
                            formatted.push_str("\r\n");
                        }
                        reg.broadcast_to_room(&world, room, &formatted, None);
                    }
                }
            }
        }
    }

    fn echo_to_room_except(&self, room: Entity, message: &str, exclude: &[Entity]) {
        if let Some(registry_lock) = get_registry() {
            if let Ok(reg) = registry_lock.try_lock() {
                if let Some(world_lock) = get_world() {
                    if let Ok(world) = world_lock.try_lock() {
                        let mut formatted = message.to_string();
                        if !formatted.ends_with('\n') {
                            formatted.push_str("\r\n");
                        }
                        reg.broadcast_to_room_except(&world, room, &formatted, exclude);
                    }
                }
            }
        }
    }
}
