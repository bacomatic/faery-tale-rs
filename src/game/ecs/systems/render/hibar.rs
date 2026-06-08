//! HiBarRenderSystem — renders the HI bar (stats display) to the SDL canvas.
//! Port of render_hibar() from gameplay_scene/rendering.rs.
//! See docs/spec/ui-menus.md.
//!
//! This system renders directly to the SDL canvas using font textures.
//! It cannot be unit-tested without a real SDL context.
//!
//! TODO(Plan D): Full implementation requires font/texture resources in Resources.

use hecs::World;
use crate::game::ecs::resources::Resources;

/// Render the HI bar (vitality, gold, brother indicator) to the SDL canvas.
///
/// `_world` — ECS world (hero stats queried here in the full impl).
/// `_res`   — game resources (HeroStats, BrotherRoster, etc.).
///
/// TODO(Plan D): accept canvas and font texture resources here.
pub fn run(_world: &World, _res: &Resources) {
    // TODO(Plan D): port render_hibar() from gameplay_scene/rendering.rs.
    // Requires:
    //   - SDL canvas reference
    //   - font texture (currently in GameplayScene)
    //   - hero stats from world query
    //   - brother indicator from res.brother.active_brother
}

#[cfg(test)]
mod tests {
    #[test]
    fn hibar_stub_compiles() {
        // This system requires SDL context for real testing.
        // Integration test is deferred to Plan D.
    }
}
