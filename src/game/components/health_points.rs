use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct HealthPoints {
    pub hp: u32,
}

impl HealthPoints {
    pub fn new(hp: u32) -> Self {
        Self { hp }
    }
}
