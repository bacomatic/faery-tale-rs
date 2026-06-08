//! Cross-system event queues. Emitted by systems during a tick, consumed
//! by downstream systems or drained at tick start.

/// All event queues for one tick. Cleared at the start of each gameplay tick.
#[derive(Default)]
pub struct Events {
    pub clock:   Vec<ClockEvent>,
    pub damage:  Vec<DamageEvent>,
    pub died:    Vec<EnemyDiedEvent>,
    pub brother: Vec<BrotherDiedEvent>,
    pub sfx:     Vec<SfxEvent>,
    pub message: Vec<MessageEvent>,
    pub speech:  Vec<SpeechEvent>,
    pub zone:    Vec<ZoneEvent>,
    pub region:  Vec<RegionTransitionEvent>,
    pub item:    Vec<ItemEvent>,
}

impl Events {
    /// Drain all queues. Called at the start of each gameplay tick.
    pub fn clear(&mut self) {
        self.clock.clear();
        self.damage.clear();
        self.died.clear();
        self.brother.clear();
        self.sfx.clear();
        self.message.clear();
        self.speech.clear();
        self.zone.clear();
        self.region.clear();
        self.item.clear();
    }
}

// ── Event types ──────────────────────────────────────────────────────────────

/// Emitted by ClockSystem on time-period boundaries.
#[derive(Debug, Clone)]
pub enum ClockEvent {
    /// A new time period (day/night bucket) has begun.
    NewPeriod { period: u8 },
    /// Hunger threshold crossed.
    HungerWarning,
    /// Fatigue threshold crossed.
    FatigueWarning,
}

/// Emitted by CombatSystem or MissileSystem when an entity takes damage.
#[derive(Debug, Clone)]
pub struct DamageEvent {
    pub target:   hecs::Entity,
    pub amount:   i16,
    /// Weapon type code that dealt the damage (0 = unarmed).
    pub weapon:   u8,
    pub is_friendly_fire: bool,
}

/// Emitted by DeathSystem or CombatSystem when an enemy reaches vitality ≤ 0.
#[derive(Debug, Clone)]
pub struct EnemyDiedEvent {
    pub entity: hecs::Entity,
    /// NPC race (for loot table lookup).
    pub race:   u8,
    /// NPC weapon (for body-search logic).
    pub weapon: u8,
    /// Gold carried.
    pub gold:   i16,
    pub x:      f32,
    pub y:      f32,
}

/// Emitted by DeathSystem when the hero's vitality reaches ≤ 0.
#[derive(Debug, Clone)]
pub struct BrotherDiedEvent {
    pub brother_id: u8,
    pub x:          f32,
    pub y:          f32,
    /// Inventory at time of death (for Bones entity).
    pub stuff:      [u8; 36],
}

/// Audio cue request.
#[derive(Debug, Clone)]
pub struct SfxEvent {
    pub sfx_id: u8,
}

/// Scroll-area message to display.
#[derive(Debug, Clone)]
pub struct MessageEvent {
    pub text: String,
}

/// Proximity auto-speech triggered.
#[derive(Debug, Clone)]
pub struct SpeechEvent {
    pub speech_id: usize,
    pub brother_name: String,
}

/// Hero entered or exited a zone.
#[derive(Debug, Clone)]
pub enum ZoneEvent {
    Entered { zone_idx: usize },
    Exited  { zone_idx: usize },
}

/// Region transition requested.
#[derive(Debug, Clone)]
pub struct RegionTransitionEvent {
    pub new_region: u8,
    pub dest_x:     f32,
    pub dest_y:     f32,
}

/// Item interaction (TAKE action resolved).
#[derive(Debug, Clone)]
pub enum ItemEvent {
    /// Hero picks up a ground item.
    TakeItem { entity: hecs::Entity },
    /// Hero searches an enemy body.
    SearchBody { entity: hecs::Entity },
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::Events;

    #[test]
    fn events_clear() {
        let mut ev = Events::default();
        ev.message.push(super::MessageEvent { text: "hello".into() });
        ev.sfx.push(super::SfxEvent { sfx_id: 3 });
        assert_eq!(ev.message.len(), 1);
        ev.clear();
        assert!(ev.message.is_empty());
        assert!(ev.sfx.is_empty());
    }
}
