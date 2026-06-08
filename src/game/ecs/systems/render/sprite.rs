//! SpriteRenderSystem — blits entity sprites into the framebuffer.
//! Port of blit_actors_to_framebuf() from gameplay_scene/rendering.rs.
//! See docs/spec/characters-animation.md and docs/spec/display-rendering.md.
//!
//! This system writes to the pixel framebuffer, not the SDL canvas directly.
//! The framebuffer is composited to the SDL canvas by the main render loop.
//!
//! TODO(Plan D): Full implementation requires SpriteSheets in Resources to be
//! populated on region load. Currently a structural stub.

use hecs::World;
use crate::game::ecs::resources::Resources;

/// Framebuffer dimensions (from display-rendering.md).
pub const FB_WIDTH:  usize = 320;
pub const FB_HEIGHT: usize = 200;

/// Blit all entity sprites into the pixel framebuffer.
pub fn run(_world: &World, _res: &Resources, _framebuf: &mut [u32]) {
    // TODO(Plan D): iterate Enemy/Hero/SetFig entities with Position + SpriteRef,
    // compute facing frame, and call blit_sprite_to_framebuf() for each.
    // Requires sprite sheets loaded into res.sprites.sheets[cfile_idx].
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use super::{run, FB_WIDTH, FB_HEIGHT};

    #[test]
    fn sprite_render_no_panic_empty() {
        let mut world = World::new();
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        let res = Resources::new(hero);
        let mut framebuf = vec![0u32; FB_WIDTH * FB_HEIGHT];
        run(&world, &res, &mut framebuf);
        // Stub: framebuf stays zeroed (no sprites loaded)
        assert_eq!(framebuf[0], 0);
    }
}
