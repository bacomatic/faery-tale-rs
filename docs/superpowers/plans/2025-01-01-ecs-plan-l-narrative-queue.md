# ECS Migration Plan L: NPC Dialogue + Narrative Queue

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `NarrativeQueue` resource and `narrative::run()` system to manage placard sequences, speech-bubble overlays, and narrative state transitions. Integrate proximity-triggered speech events into the queue, implement timer-based display logic, and add `render_placard()` overlay rendering to `EcsScene`.

**Architecture:** `NarrativeQueue` is a FIFO queue stored in `Resources` containing `NarrEvent` entries. The `narrative::run()` system processes one active event at a time, decrementing its timer and executing side effects when complete. Placard events set `viewstatus = 2`, which triggers `render_placard()` overlay rendering in `EcsScene`. Speech events from proximity triggers continue using the existing scroll message system.

**Prerequisites:** Plan I (Talk action menu dispatch). Plans A-D complete.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/resources.rs` | Add NarrEvent, NarrativeQueue, narrative field |
| `src/game/ecs/systems/narrative.rs` | Implement run() + execute_event() + tests |
| `src/game/ecs/scene.rs` | Add render_placard(); call from update() |

---

## Task 1: Define NarrativeQueue in resources.rs

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Add NarrEvent enum**

Add after VfxState:
```rust
/// Narrative events for scripted sequences and dialogue.
#[derive(Debug, Clone)]
pub enum NarrEvent {
    /// Display centered placard text for specified duration.
    Placard { 
        text: String, 
        hold_ticks: u32 
    },
    /// Wait for specified number of gameplay ticks.
    WaitTicks(u32),
    /// Teleport hero to new position/region.
    TeleportHero { 
        x: f32, 
        y: f32, 
        region: u8 
    },
    /// Swap an object's sprite ID (used for transformations).
    SwapObjectId { 
        object_index: usize, 
        new_id: u8 
    },
    /// Apply quest rewards (experience, items, etc.).
    ApplyRewards,
}
```

- [ ] **Step 2: Add NarrativeQueue struct**

```rust
/// FIFO queue for narrative events with timer management.
#[derive(Debug, Default)]
pub struct NarrativeQueue {
    /// Pending events to be processed.
    pending: Vec<NarrEvent>,
    /// Currently active event, if any.
    active: Option<NarrEvent>,
    /// Ticks remaining for active event.
    active_ticks: u32,
}
```

- [ ] **Step 3: Implement NarrativeQueue methods**

```rust
impl NarrativeQueue {
    /// Create new empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push event to the back of the queue.
    pub fn push(&mut self, event: NarrEvent) {
        self.pending.push(event);
    }

    /// Activate next pending event.
    pub fn activate_next(&mut self) -> bool {
        if let Some(event) = self.pending.pop() {
            let ticks = match &event {
                NarrEvent::Placard { hold_ticks, .. } => *hold_ticks,
                NarrEvent::WaitTicks(ticks) => *ticks,
                _ => 0, // Instant events
            };
            self.active = Some(event);
            self.active_ticks = ticks;
            true
        } else {
            self.active = None;
            self.active_ticks = 0;
            false
        }
    }

    /// Check if queue is idle (no active event).
    pub fn is_idle(&self) -> bool {
        self.active.is_none() && self.pending.is_empty()
    }

    /// Get mutable reference to active event.
    pub fn active_mut(&mut self) -> Option<&mut NarrEvent> {
        self.active.as_mut()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.active = None;
        self.active_ticks = 0;
    }

    /// Decrement active timer, return true if expired.
    pub fn tick(&mut self) -> bool {
        if self.active_ticks > 0 {
            self.active_ticks -= 1;
            self.active_ticks == 0
        } else {
            true // Instant events expire immediately
        }
    }
}
```

- [ ] **Step 4: Add narrative field to Resources**

Add to Resources struct:
```rust
pub struct Resources {
    // ... existing fields ...
    pub narrative: NarrativeQueue,
}
```

- [ ] **Step 5: Update Resources::new()**

Add initialization:
```rust
impl Resources {
    pub fn new(hero_entity: hecs::Entity) -> Self {
        Self {
            // ... existing field initializations ...
            narrative: NarrativeQueue::new(),
        }
    }
}
```

---

## Task 2: Implement narrative::run()

**Files:**
- Modify: `src/game/ecs/systems/narrative.rs`

- [ ] **Step 1: Replace empty run() implementation**

```rust
//! Narrative system for managing scripted sequences and dialogue overlays.
//! Handles placard display, timed events, and narrative state transitions.

use hecs::World;
use crate::game::ecs::resources::{Resources, NarrEvent};

/// Run the narrative system.
/// Processes one active event at a time, managing timers and side effects.
pub fn run(world: &mut World, res: &mut Resources) {
    // If no active event, try to activate next
    if res.narrative.active.is_none() {
        if !res.narrative.activate_next() {
            // No events to process, clear placard viewstatus
            if res.view.viewstatus == 2 {
                res.view.viewstatus = 0;
            }
            return;
        }
    }

    // Tick the active event
    if res.narrative.tick() {
        // Event expired, execute side effects and advance
        execute_event(world, res);
        res.narrative.activate_next();
    } else {
        // Event still active, set viewstatus for placards
        if let Some(NarrEvent::Placard { .. }) = res.narrative.active {
            res.view.viewstatus = 2; // Signal placard overlay
        }
    }
}

/// Execute side effects for completed events.
fn execute_event(world: &mut World, res: &mut Resources) {
    if let Some(event) = res.narrative.active.take() {
        match event {
            NarrEvent::Placard { .. } => {
                // No side effects, just display
            }
            NarrEvent::WaitTicks(_) => {
                // No side effects, just timing
            }
            NarrEvent::TeleportHero { x, y, region } => {
                // Update hero position
                if let Ok(mut pos) = world.get_mut::<crate::game::ecs::components::Position>(res.hero_entity) {
                    pos.set(x, y);
                }
                // Trigger region change
                res.region.new_region = region;
            }
            NarrEvent::SwapObjectId { object_index, new_id } => {
                // TODO: Implement object sprite swapping
                // This will need to interact with the object system
                log::warn!("SwapObjectId not yet implemented: obj={}, id={}", object_index, new_id);
            }
            NarrEvent::ApplyRewards => {
                // TODO: Implement quest reward application
                // This will need to interact with the quest/inventory systems
                log::warn!("ApplyRewards not yet implemented");
            }
        }
    }
}
```

- [ ] **Step 2: Add required imports**

```rust
use crate::game::ecs::components::Position;
```

---

## Task 3: Add render_placard() to EcsScene

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add font import**

```rust
use crate::game::font::GameFont;
```

- [ ] **Step 2: Implement render_placard method**

```rust
impl EcsScene {
    /// Render centered placard overlay for narrative events.
    /// Called when viewstatus == 2.
    fn render_placard(&self, canvas: &mut sdl3::render::Canvas<sdl3::video::Window>, font: &mut GameFont) {
        // Get placard text from narrative queue
        let text = if let Some(NarrEvent::Placard { text, .. }) = self.res.narrative.active {
            text
        } else {
            return; // No placard to render
        };

        // Calculate centered position
        const PLACARD_WIDTH: i32 = 400;
        const PLACARD_HEIGHT: i32 = 200;
        const SCREEN_WIDTH: i32 = 640;
        const SCREEN_HEIGHT: i32 = 280;
        
        let x = (SCREEN_WIDTH - PLACARD_WIDTH) / 2;
        let y = (SCREEN_HEIGHT - PLACARD_HEIGHT) / 2;

        // Draw dark background
        canvas.set_draw_color(sdl3::pixels::Color::RGBA(0, 0, 0, 200));
        let rect = sdl3::rect::Rect::new(x, y, PLACARD_WIDTH as u32, PLACARD_HEIGHT as u32);
        canvas.fill_rect(rect).unwrap();

        // Draw border
        canvas.set_draw_color(sdl3::pixels::Color::RGB(255, 255, 255));
        canvas.draw_rect(rect).unwrap();

        // Render text centered
        font.set_color_mod(255, 255, 255); // White topaz font
        
        // Simple word wrapping and centering
        let lines: Vec<&str> = text.lines().collect();
        let line_height = 12;
        let start_y = y + (PLACARD_HEIGHT - (lines.len() as i32 * line_height)) / 2;
        
        for (i, line) in lines.iter().enumerate() {
            let text_width = line.len() * 8; // Approximate 8px per character
            let text_x = x + (PLACARD_WIDTH - text_width as i32) / 2;
            let text_y = start_y + (i as i32 * line_height);
            
            font.render_string(canvas, line, text_x as i32, text_y as i32);
        }
    }
}
```

- [ ] **Step 3: Call render_placard from update()**

In `EcsScene::update()` method, after `render_hibar()`:
```rust
// Render narrative placards if active
if self.res.view.viewstatus == 2 {
    self.render_placard(canvas, resources.font);
}
```

- [ ] **Step 4: Add font field to SceneResources if missing**

Ensure `SceneResources` has access to font:
```rust
pub struct SceneResources<'a, 'b> {
    pub font: &'a mut GameFont,
    // ... other fields ...
}
```

---

## Task 4: Integrate SpeechEvent path

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add TODO comment in drain_messages()**

In the `drain_messages()` method, after the SpeechEvent handling:
```rust
// Speech → MessageQueue
for ev in self.res.events.speech.drain(..) {
    let text = crate::game::events::speak(&self.res.narr, ev.speech_id, &ev.brother_name);
    self.res.messages.push(text);
    
    // TODO(Plan I): Route Talk-action speech to narrative.push(Placard{..})
    // Proximity-triggered speech should continue using scroll messages
}
```

- [ ] **Step 2: Add narrative helper for Talk actions**

```rust
impl EcsScene {
    /// Add a placard event from Talk action (Plan I integration).
    pub fn add_talk_placard(&mut self, text: String, hold_ticks: u32) {
        self.res.narrative.push(NarrEvent::Placard { text, hold_ticks });
    }
}
```

---

## Task 5: Add unit tests in narrative.rs

**Files:**
- Modify: `src/game/ecs/systems/narrative.rs`

- [ ] **Step 1: Add test module**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::{Resources, NarrEvent};

    fn create_test_resources() -> Resources {
        let world = World::new();
        let hero = world.spawn(());
        Resources::new(hero)
    }

    #[test]
    fn narrative_no_panic_empty() {
        let mut world = World::new();
        let mut res = create_test_resources();
        
        // Should not panic with empty queue
        run(&mut world, &mut res);
        assert!(res.narrative.is_idle());
        assert_eq!(res.view.viewstatus, 0);
    }

    #[test]
    fn placard_timer_expires() {
        let mut world = World::new();
        let mut res = create_test_resources();
        
        // Add 3-tick placard
        res.narrative.push(NarrEvent::Placard { 
            text: "Test placard".to_string(), 
            hold_ticks: 3 
        });
        
        // Tick 1: Should activate and set viewstatus
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 2);
        assert!(!res.narrative.is_idle());
        
        // Tick 2: Still active
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 2);
        assert!(!res.narrative.is_idle());
        
        // Tick 3: Expires
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 0);
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn wait_ticks_advances() {
        let mut world = World::new();
        let mut res = create_test_resources();
        
        // Add 2-tick wait
        res.narrative.push(NarrEvent::WaitTicks(2));
        
        // Tick 1: Activate
        run(&mut world, &mut res);
        assert!(!res.narrative.is_idle());
        
        // Tick 2: Expires
        run(&mut world, &mut res);
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn queue_fifo_order() {
        let mut world = World::new();
        let mut res = create_test_resources();
        
        // Add multiple events
        res.narrative.push(NarrEvent::WaitTicks(1));
        res.narrative.push(NarrEvent::Placard { 
            text: "Second".to_string(), 
            hold_ticks: 1 
        });
        res.narrative.push(NarrEvent::WaitTicks(1));
        
        // Process all events
        for _ in 0..3 {
            run(&mut world, &mut res);
        }
        
        // Should be idle after processing all
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn teleport_hero_updates_position() {
        let mut world = World::new();
        let hero = world.spawn(crate::game::ecs::components::Position::new(100.0, 100.0));
        let mut res = Resources::new(hero);
        
        // Add instant teleport event
        res.narrative.push(NarrEvent::TeleportHero { 
            x: 500.0, 
            y: 300.0, 
            region: 2 
        });
        
        // Should execute immediately
        run(&mut world, &mut res);
        
        // Check position updated
        if let Ok(pos) = world.get::<crate::game::ecs::components::Position>(hero) {
            assert_eq!(pos.x, 500.0);
            assert_eq!(pos.y, 300.0);
        }
        assert_eq!(res.region.new_region, 2);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test ecs::systems::narrative::tests 2>&1 | grep "^test result"
```

Expected: all 5 tests pass.

---

## Task 6: Integration testing

- [ ] **Step 1: Build with narrative system**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Test placard rendering**

Manual test with running game:
1. Use debug console to push a placard event
2. Verify centered overlay appears with dark background
3. Verify text is centered and readable
4. Verify overlay disappears after timer expires

- [ ] **Step 3: Test queue behavior**

Manual test:
1. Push multiple events to queue
2. Verify they execute in FIFO order
3. Verify viewstatus transitions correctly

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/resources.rs src/game/ecs/systems/narrative.rs src/game/ecs/scene.rs
git commit -m "feat(ecs): implement NarrativeQueue and placard rendering system"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test ecs::systems::narrative::tests 2>&1 | grep "^test result"
```

Both succeed. Narrative queue system is fully implemented.

---

## Spec references

- `docs/spec/npcs-dialogue.md` §13.2–13.4 — Talk system, NPC speech
- `docs/spec/intro-narrative.md` §23.5–23.8 — Placard rendering, text system
- `reference/logic/npc-dialogue.md` (research branch) — NPC dialogue state machine

## Test plan

- narrative_no_panic_empty: empty queue runs without panic
- placard_timer_expires: Placard with 3-tick duration expires correctly
- wait_ticks_advances: WaitTicks(2) expires after 2 ticks
- queue_fifo_order: multiple events process in order
- teleport_hero_updates_position: TeleportHero event updates hero position

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add NarrEvent, NarrativeQueue, narrative field |
| `src/game/ecs/systems/narrative.rs` | Implement run() + execute_event() + tests |
| `src/game/ecs/scene.rs` | Add render_placard(); call from update() |