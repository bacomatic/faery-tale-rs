//! PaletteSystem — recomputes the display palette each tick from light level and spell state.
//! Port of compute_current_palette() from gameplay_scene/region.rs.
//! See docs/spec/palettes-daynight-visuals.md.

use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(_world: &World, res: &mut Resources) {
    // The palette computation depends on base_colors_palette being set
    // (done by RegionSystem on region transition). If it's not set, no-op.
    let base = match res.palette.base_colors_palette {
        Some(b) => b,
        None => return,
    };

    let lightlevel = res.clock.lightlevel;
    let light_on = res.clock.light_timer > 0;
    let secret_on = res.clock.secret_timer > 0;
    let region_num = res.region.region_num;

    // Compute new palette from base + light level.
    // This delegates to the existing gameplay_scene function via a call.
    // TODO(Plan D): move compute_current_palette here from gameplay_scene/region.rs.
    // For now call through the existing static function if accessible, else recompute inline.
    let new_palette = compute_palette(base, lightlevel, light_on, secret_on, region_num);

    if new_palette != res.palette.current_palette {
        res.palette.current_palette = new_palette;
        res.palette.dirty = true;
    }
}

/// Compute display palette from base colors + day/night light level.
/// Mirrors GameplayScene::compute_current_palette().
fn compute_palette(
    base: [u32; 32],
    lightlevel: u16,
    light_on: bool,
    secret_on: bool,
    region_num: u8,
) -> [u32; 32] {
    // Day/night fade: lightlevel 0–300, map to percentage 0–100.
    let base_pct = (lightlevel as u32 * 100 / 300).min(100) as u8;
    let light_boost = if light_on { 60u8 } else { 0 };
    let pct = base_pct.saturating_add(light_boost).min(100);

    let mut result = base;
    // Apply fade to colors 0–30 (color 31 is special per region).
    for i in 0..31 {
        result[i] = fade_color(base[i], pct);
    }
    // Color 31: region-specific (dark areas, dungeon, secret zone).
    result[31] = if secret_on && region_num == 9 {
        0x00FF00FF // bright green in secret zone
    } else {
        fade_color(base[31], pct)
    };
    result
}

/// Fade an RGBA color to `pct`% brightness (0 = black, 100 = full).
fn fade_color(rgba: u32, pct: u8) -> u32 {
    let r = (((rgba >> 24) & 0xFF) as u32 * pct as u32 / 100) as u8;
    let g = (((rgba >> 16) & 0xFF) as u32 * pct as u32 / 100) as u8;
    let b = (((rgba >>  8) & 0xFF) as u32 * pct as u32 / 100) as u8;
    let a = (rgba & 0xFF) as u8;
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use super::run;

    fn make_res(world: &mut World) -> Resources {
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        Resources::new(hero)
    }

    #[test]
    fn palette_not_computed_without_base() {
        let mut world = World::new();
        let mut res = make_res(&mut world);
        // No base_colors_palette set
        assert!(res.palette.base_colors_palette.is_none());
        run(&world, &mut res);
        // Should be a no-op - dirty starts true and should remain true
        assert!(res.palette.dirty);
    }

    #[test]
    fn palette_dirty_when_changed() {
        let mut world = World::new();
        let mut res = make_res(&mut world);
        // Set a non-zero base palette
        let mut base = [0u32; 32];
        base[0] = 0xFF0000FF; // red
        res.palette.base_colors_palette = Some(base);
        res.palette.current_palette = [0u32; 32]; // different from what will be computed
        res.palette.dirty = false;
        res.clock.lightlevel = 150; // half brightness
        run(&world, &mut res);
        assert!(res.palette.dirty, "Palette should be dirty after change");
    }
}