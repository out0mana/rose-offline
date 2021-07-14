use legion::{component, system, systems::CommandBuffer, Entity};
use log::warn;

use crate::{
    data::account::{AccountStorage, AccountStorageError},
    game::{
        components::{Account, LoginClient},
        messages::client::{
            ClientMessage, ConnectionRequestResponse, GetChannelListError, JoinServerError,
            JoinServerResponse, LoginError,
        },
        resources::{LoginTokens, ServerList},
    },
};

#[system(for_each)]
#[filter(!component::<Account>())]
pub fn login_server_authentication(
    cmd: &mut CommandBuffer,
    entity: &Entity,
    client: &mut LoginClient,
) {
    if let Ok(message) = client.client_message_rx.try_recv() {
        match message {
            ClientMessage::ConnectionRequest(message) => {
                message
                    .response_tx
                    .send(Ok(ConnectionRequestResponse {
                        packet_sequence_id: 123,
                    }))
                    .ok();
            }
            ClientMessage::LoginRequest(message) => {
                let result =
                    match AccountStorage::try_load(&message.username, &message.password_md5) {
                        Ok(account) => {
                            cmd.add_component(*entity, Account::from(account));
                            Ok(())
                        }
                        Err(error) => Err(match error {
                            AccountStorageError::NotFound => LoginError::InvalidAccount,
                            AccountStorageError::InvalidPassword => LoginError::InvalidPassword,
                            _ => LoginError::Failed,
                        }),
                    };
                message.response_tx.send(result).ok();
            }
            _ => panic!("Received unexpected client message {:?}", message),
        }
    }
}

#[system(for_each)]
pub fn login_server(
    account: &Account,
    client: &mut LoginClient,
    #[resource] server_list: &ServerList,
    #[resource] login_tokens: &mut LoginTokens,
) {
    if let Ok(message) = client.client_message_rx.try_recv() {
        match message {
            ClientMessage::GetWorldServerList(message) => {
                let mut servers = Vec::new();
                for (id, server) in server_list.world_servers.iter().enumerate() {
                    servers.push((id as u32, server.name.clone()));
                }
                message.response_tx.send(servers).ok();
            }
            ClientMessage::GetChannelList(message) => {
                let response = server_list
                    .world_servers
                    .get(message.server_id as usize)
                    .ok_or(GetChannelListError::InvalidServerId)
                    .map(|world_server| {
                        let mut channels = Vec::new();
                        for (id, channel) in world_server.channels.iter().enumerate() {
                            channels.push((id as u8, channel.name.clone()));
                        }
                        channels
                    });
                message.response_tx.send(response).ok();
            }
            ClientMessage::JoinServer(message) => {
                let response = server_list
                    .world_servers
                    .get(message.server_id as usize)
                    .ok_or(JoinServerError::InvalidServerId)
                    .and_then(|world_server| {
                        world_server
                            .channels
                            .get(message.channel_id as usize)
                            .ok_or(JoinServerError::InvalidChannelId)
                            .map(|game_server| {
                                client.login_token = login_tokens.generate(
                                    account.name.clone(),
                                    world_server.entity,
                                    game_server.entity,
                                );
                                JoinServerResponse {
                                    login_token: client.login_token,
                                    packet_codec_seed: world_server.packet_codec_seed,
                                    ip: world_server.ip.clone(),
                                    port: world_server.port,
                                }
                            })
                    });

                message.response_tx.send(response).ok();
            }
            _ => warn!("Received unimplemented client message {:?}", message),
        }
    }
}
