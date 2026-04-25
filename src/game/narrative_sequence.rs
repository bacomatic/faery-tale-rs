use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NarrativeStep {
    WaitTicks { remaining: u32 },
    ShowPlacard { key: String, substitution: Option<String>, hold_ticks: u32 },
    ClearInnerRect,
    ShowRescueHomeText { line17: String, hero_name: String, line18: String },
    TeleportHero { x: i32, y: i32, region: u8 },
    MoveExtent { index: usize, x: i32, y: i32 },
    SwapObjectId { object_index: usize, new_id: u8 },
    ApplyRescueRewardsAndFlags,
}

#[derive(Default)]
pub struct NarrativeQueue {
    steps: VecDeque<NarrativeStep>,
    active_step: Option<NarrativeStep>,
    active_step_index: Option<usize>,
    next_step_index: usize,
}

impl NarrativeQueue {
    pub fn reset(&mut self, steps: Vec<NarrativeStep>) {
        self.steps = steps.into();
        self.active_step = None;
        self.active_step_index = None;
        self.next_step_index = 0;
    }

    pub fn tick_one(&mut self) {
        if self.active_step.is_none() {
            self.activate_next_step();
        }

        let should_advance = match self.active_step.as_mut() {
            Some(NarrativeStep::WaitTicks { remaining }) => {
                if *remaining > 1 {
                    *remaining -= 1;
                    false
                } else {
                    true
                }
            }
            Some(NarrativeStep::ShowPlacard { hold_ticks, .. }) => {
                if *hold_ticks > 0 {
                    *hold_ticks -= 1;
                }
                false
            }
            // Non-wait steps must stay active until their explicit execution hook
            // completes and advances the queue.
            Some(_) => false,
            None => false,
        };

        if should_advance {
            self.advance_active_step();
        }
    }

    pub fn advance_active_step(&mut self) {
        self.active_step = None;
        self.active_step_index = None;
        self.activate_next_step();
    }

    pub fn active_step_index(&self) -> Option<usize> {
        self.active_step_index
    }

    pub fn active_step(&self) -> Option<&NarrativeStep> {
        self.active_step.as_ref()
    }

    fn activate_next_step(&mut self) {
        if let Some(next) = self.steps.pop_front() {
            self.active_step = Some(next);
            self.active_step_index = Some(self.next_step_index);
            self.next_step_index += 1;
        }
    }

    #[cfg(test)]
    pub fn debug_snapshot_steps(&self) -> Vec<NarrativeStep> {
        let mut out = Vec::with_capacity(self.steps.len() + usize::from(self.active_step.is_some()));
        if let Some(step) = self.active_step.as_ref() {
            out.push(step.clone());
        }
        out.extend(self.steps.iter().cloned());
        out
    }
}