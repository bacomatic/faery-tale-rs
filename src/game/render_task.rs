
use sdl2::{rect::Rect, render::Canvas, video::Window};

/**
 * Render task handles periodic rendering for various game elements.
 */

pub trait RenderTask {
    /**
     * A RenderTask is a task called periodically to update a portion of the screen.
     * For time-sensitive tasks, the delta_ticks parameter indicates how much time has passed since
     * the last update. The area parameter indicates which portion of the screen needs updating.
     * A task may choose to ignore the area.
     *
     * @param delta_ticks Number of game ticks since last update, these are 1/60 second ticks.
     * @param area Optional area that needs updating. If None, the entire area should be updated.
     * @return true if the task needs to continue running, false if it is complete and can be removed.
     */
    fn update(self: &mut Self, _canvas: &mut Canvas<Window>, _delta_ticks: i32, _area: Option<Rect>) -> bool
    {
        false
    }

    /**
     * Cancel the render task, cleaning up any resources it may have allocated. Most of the time this shouldn't
     * be necessary, but in some cases it may be needed.
     */
    fn cancel(self: &mut Self) {}
}
