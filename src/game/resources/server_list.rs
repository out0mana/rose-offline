use bevy_ecs::prelude::Entity;

pub struct GameServer {
    pub entity: Entity,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub packet_codec_seed: u32,
}

pub struct WorldServer {
    pub entity: Entity,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub packet_codec_seed: u32,
    pub channels: Vec<GameServer>,
}

#[derive(Default)]
pub struct ServerList {
    pub world_servers: Vec<WorldServer>,
}

impl ServerList {
    pub fn new() -> Self {
        Default::default()
    }
}
