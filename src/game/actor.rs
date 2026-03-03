#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActorKind {
    #[default]
    Player,
    Enemy,
    Object,
    Raft,
    SetFig,
    Carrier,
    Dragon,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActorState {
    Still,
    Walking,
    Fighting(u8),
    Dying,
    Dead,
    Shooting(u8),
    Sinking,
    Falling,
    Sleeping,
}

impl Default for ActorState {
    fn default() -> Self {
        ActorState::Still
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Goal {
    User,
    Attack1,
    Attack2,
    Archer1,
    Archer2,
    Flee,
    Follower,
    Leader,
    Stand,
    Guard,
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Tactic {
    Pursue,
    Shoot,
    Random,
    BumbleSeek,
    Backup,
    Follow,
    Evade,
    EggSeek,
    Frust,
    #[default]
    None,
}

#[derive(Debug, Clone, Default)]
pub struct Actor {
    pub abs_x: u16,
    pub abs_y: u16,
    pub rel_x: i16,
    pub rel_y: i16,
    pub kind: ActorKind,
    pub race: u8,
    pub state: ActorState,
    pub goal: Goal,
    pub tactic: Tactic,
    pub facing: u8, // 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
    pub vitality: i16,
    pub weapon: u8,
    pub environ: i8,
    pub vel_x: i16,
    pub vel_y: i16,
    pub moving: bool,
}

impl Actor {
    pub fn is_active(&self) -> bool {
        !matches!(self.state, ActorState::Dead)
    }

    pub fn clear(&mut self) {
        *self = Actor::default();
    }
}
