use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::convert::TryFrom;
use tokio::sync::oneshot;

use crate::game::messages::{
    client::{ClientMessage, GameConnectionRequest, JoinZoneRequest, Move, SetHotbarSlot},
    server::{
        LocalChat, RemoveEntities, ServerMessage, SpawnEntityMonster, SpawnEntityNpc, Whisper,
    },
};
use crate::protocol::{packet::Packet, Client, ProtocolClient, ProtocolError};

mod common_packets;

mod client_packets;
mod server_packets;

use client_packets::*;
use server_packets::*;

pub struct GameClient {}

impl GameClient {
    pub fn new() -> Self {
        Self {}
    }

    async fn handle_packet<'a>(
        &self,
        client: &mut Client<'a>,
        packet: Packet,
    ) -> Result<(), ProtocolError> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ClientPackets::ConnectRequest) => {
                let request = PacketClientConnectRequest::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::GameConnectionRequest(
                        GameConnectionRequest {
                            login_token: request.login_token,
                            password_md5: String::from(request.password_md5),
                            response_tx: response_tx,
                        },
                    ))?;
                match response_rx.await? {
                    Ok(response) => {
                        client
                            .connection
                            .write_packet(Packet::from(&PacketConnectionReply {
                                result: ConnectResult::Ok,
                                packet_sequence_id: response.packet_sequence_id,
                                pay_flags: 0xff,
                            }))
                            .await?;

                        client
                            .connection
                            .write_packet(Packet::from(&PacketServerSelectCharacter {
                                character_info: &response.character_info,
                                position: &response.position,
                                equipment: &response.equipment,
                                basic_stats: &response.basic_stats,
                                level: &response.level,
                                skill_list: &response.skill_list,
                                hotbar: &response.hotbar,
                            }))
                            .await?;

                        client
                            .connection
                            .write_packet(Packet::from(&PacketServerCharacterInventory {
                                inventory: &response.inventory,
                                equipment: &response.equipment,
                            }))
                            .await?;

                        client
                            .connection
                            .write_packet(Packet::from(&PacketServerCharacterQuestData {}))
                            .await?;
                    }
                    Err(_) => {
                        client
                            .connection
                            .write_packet(Packet::from(&PacketConnectionReply {
                                result: ConnectResult::Failed,
                                packet_sequence_id: 0,
                                pay_flags: 0,
                            }))
                            .await?;
                    }
                };
            }
            Some(ClientPackets::JoinZone) => {
                let _request = PacketClientJoinZone::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::JoinZoneRequest(JoinZoneRequest {
                        response_tx,
                    }))?;
                let response = response_rx.await?;
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerJoinZone {
                        entity_id: response.entity_id,
                        level: &response.level,
                    }))
                    .await?;
            }
            Some(ClientPackets::Chat) => {
                let packet = PacketClientChat::try_from(&packet)?;
                client
                    .client_message_tx
                    .send(ClientMessage::Chat(String::from(packet.text)))?;
            }
            Some(ClientPackets::Move) => {
                let packet = PacketClientMove::try_from(&packet)?;
                client.client_message_tx.send(ClientMessage::Move(Move {
                    target_entity_id: packet.target_entity_id,
                    x: packet.x,
                    y: packet.y,
                    z: packet.z,
                }))?;
            }
            Some(ClientPackets::SetHotbarSlot) => {
                let request = PacketClientSetHotbarSlot::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::SetHotbarSlot(SetHotbarSlot {
                        slot_index: request.slot_index as usize,
                        slot: request.slot.clone(),
                        response_tx,
                    }))?;
                if let Ok(_) = response_rx.await? {
                    client
                        .connection
                        .write_packet(Packet::from(&PacketServerSetHotbarSlot {
                            slot_index: request.slot_index,
                            slot: request.slot,
                        }))
                        .await?;
                }
            }
            _ => println!("Unhandled packet {}", packet.command),
        }
        Ok(())
    }

    async fn handle_server_message<'a>(
        &self,
        client: &mut Client<'a>,
        message: ServerMessage,
    ) -> Result<(), ProtocolError> {
        match message {
            ServerMessage::MoveEntity(message) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerMoveEntity {
                        entity_id: message.entity_id,
                        target_entity_id: message.target_entity_id,
                        distance: message.distance,
                        x: message.x,
                        y: message.y,
                        z: message.z,
                    }))
                    .await?;
            }
            ServerMessage::StopMoveEntity(message) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerStopMoveEntity {
                        entity_id: message.entity_id,
                        x: message.x,
                        y: message.y,
                        z: message.z,
                    }))
                    .await?;
            }
            ServerMessage::Teleport(message) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerTeleport {
                        entity_id: message.entity_id,
                        zone_no: message.zone_no,
                        x: message.x,
                        y: message.y,
                        run_mode: message.run_mode,
                        ride_mode: message.ride_mode,
                    }))
                    .await?;
            }
            ServerMessage::LocalChat(LocalChat { entity_id, text }) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerLocalChat {
                        entity_id,
                        text: &text,
                    }))
                    .await?;
            }
            ServerMessage::Whisper(Whisper { from, text }) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerWhisper {
                        from: &from,
                        text: &text,
                    }))
                    .await?;
            }
            ServerMessage::SpawnEntityNpc(SpawnEntityNpc {
                entity_id,
                npc,
                position,
            }) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerSpawnEntityNpc {
                        entity_id,
                        npc: &npc,
                        position: &position,
                    }))
                    .await?;
            }
            ServerMessage::SpawnEntityMonster(SpawnEntityMonster {
                entity_id,
                monster,
                position,
            }) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerSpawnEntityMonster {
                        entity_id,
                        monster: &monster,
                        position: &position,
                    }))
                    .await?;
            }
            ServerMessage::RemoveEntities(RemoveEntities { entity_ids }) => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerRemoveEntities {
                        entity_ids: &entity_ids,
                    }))
                    .await?;
            }
            _ => {
                panic!("Unimplemented message for irose game server!")
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ProtocolClient for GameClient {
    async fn run_client(&self, client: &mut Client) -> Result<(), ProtocolError> {
        loop {
            tokio::select! {
                packet = client.connection.read_packet() => {
                    match packet {
                        Ok(packet) => {
                            self.handle_packet(client, packet).await?;
                        },
                        Err(error) => {
                            return Err(error);
                        }
                    }
                },
                Some(message) = client.server_message_rx.recv() => {
                    self.handle_server_message(client, message).await?;
                }
            };
        }
    }
}
