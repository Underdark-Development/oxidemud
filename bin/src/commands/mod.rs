pub mod abilities;
pub mod alias;
pub mod bank;
pub mod builder;
pub mod character;
pub mod combat;
pub mod common;
pub mod communication;
pub mod general;
pub mod group;
pub mod items;
pub mod movement;
pub mod reports;
pub mod shop;
pub mod social;
pub mod test_helpers;

pub use movement::cmd_look;
use oxide_server::Server;

pub fn register_all_commands(server: &mut Server) {
    general::register(server);
    movement::register(server);
    communication::register(server);
    combat::register(server);
    group::register(server);
    items::register(server);
    character::register(server);
    abilities::register(server);
    builder::register(server);
    reports::register(server);
    social::register(server);
    alias::register(server);
    bank::register(server);
    shop::register(server);
}
