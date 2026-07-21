pub mod api;
pub mod commands;
pub mod diar_pickup_loop;

pub use api::*;
pub use diar_pickup_loop::spawn_diar_pickup_loop;
// Don't re-export commands to avoid conflicts - lib.rs will import directly
