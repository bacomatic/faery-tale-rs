//! ECS world types for the faery-tale-rs rearchitecture.
//! See docs/superpowers/plans/2025-01-01-ecs-plan-b-components.md

pub mod components;
pub mod debug_commands;
pub mod events;
pub mod resources;
pub mod scene;
pub mod spawn;
pub mod systems;

pub use scene::EcsScene;

pub use components::*;
pub use events::Events;
pub use resources::Resources;
