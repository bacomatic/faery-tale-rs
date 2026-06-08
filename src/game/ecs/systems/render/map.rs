//! MapRenderSystem — composes the terrain tile map into the framebuffer.
//! Port of MapRenderer::compose() from map_renderer.rs.
//! See docs/spec/display-rendering.md.

use hecs::World;
use crate::game::ecs::resources::Resources;

/// Output framebuffer size constants (from display-rendering.md).
pub const FB_WIDTH:  usize = 320;
pub const FB_HEIGHT: usize = 200;

pub fn run(_world: &World, res: &mut Resources, framebuf: &mut [u32]) {
    let renderer = match res.map.renderer.as_mut() {
        Some(r) => r,
        None => return, // no map loaded
    };
    let world_data = match res.map.world.as_ref() {
        Some(w) => w,
        None => return,
    };

    renderer.compose(
        res.camera.map_x as u16,
        res.camera.map_y as u16,
        world_data,
    );
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use super::run;

    #[test]
    fn map_render_no_panic_without_map() {
        let mut world = World::new();
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        let mut res = Resources::new(hero);
        let mut framebuf = vec![0u32; super::FB_WIDTH * super::FB_HEIGHT];
        run(&world, &mut res, &mut framebuf);
        // No panic = success; framebuf stays zeroed when no map loaded
        assert_eq!(framebuf[0], 0);
    }
}