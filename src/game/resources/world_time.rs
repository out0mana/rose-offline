use std::time::Duration;

use crate::data::WorldTicks;

pub struct WorldTime {
    pub ticks: WorldTicks,
    pub time_since_last_tick: Duration,
}

impl WorldTime {
    pub fn new() -> Self {
        Self {
            ticks: WorldTicks(0),
            time_since_last_tick: Duration::from_secs(0),
        }
    }
}
