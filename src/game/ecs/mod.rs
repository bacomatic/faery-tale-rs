//! ECS world types for the faery-tale-rs rearchitecture.
//! See docs/superpowers/plans/2025-01-01-ecs-plan-b-components.md

pub mod components;
pub mod events;
pub mod resources;
pub mod spawn;
pub mod systems;

pub use components::*;
pub use events::Events;
pub use resources::Resources;
