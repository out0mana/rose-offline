use crossbeam_channel::Receiver;
use legion::*;
use std::time::Duration;

use super::components::GameClient;
use super::messages::control::ControlMessage;
use super::resources::{
    ClientEntityIdList, ControlChannel, LoginTokens, ServerList, ServerMessages,
};
use super::systems::*;

pub struct Game {
    tick_rate_hz: u64,
    control_rx: Receiver<ControlMessage>,
}

#[system(for_each)]
pub fn cleanup_unread_messages(client: &mut GameClient) {
    client.pending_messages.clear();
}

impl Game {
    pub fn new(control_rx: Receiver<ControlMessage>) -> Game {
        Game {
            tick_rate_hz: 30,
            control_rx,
        }
    }

    pub fn run(&mut self) {
        let mut world = World::default();

        let mut resources = Resources::default();
        resources.insert(ControlChannel::new(self.control_rx.clone()));
        resources.insert(ServerList::new());
        resources.insert(LoginTokens::new());
        resources.insert(ClientEntityIdList::new());
        resources.insert(ServerMessages::new());

        let mut schedule = Schedule::builder()
            .add_system(control_server_system())
            .add_system(login_server_authentication_system())
            .add_system(login_server_system())
            .add_system(world_server_authentication_system())
            .add_system(world_server_system())
            .add_system(game_server_authentication_system())
            .add_system(game_server_join_system())
            .add_system(game_server_move_system())
            .flush()
            .add_system(server_messages_sender_system())
            .add_system(cleanup_unread_messages_system())
            .build();

        let min_tick_duration = Duration::from_millis(1000 / self.tick_rate_hz);

        loop {
            schedule.execute(&mut world, &mut resources);
            std::thread::sleep(min_tick_duration); // TODO: This should account for duration of execution
        }
    }
}
