use async_trait::async_trait;
use log::warn;
use num_traits::FromPrimitive;
use std::convert::TryFrom;

use crate::{
    game::messages::{client::*, server::ServerMessage},
    protocol::{Client, Packet, ProtocolClient, ProtocolError},
};

mod client_packets;
mod server_packets;

use client_packets::*;
use server_packets::*;
use tokio::sync::oneshot;

pub struct WorldClient {}

impl WorldClient {
    pub fn new() -> Self {
        Self {}
    }

    async fn handle_packet(
        &self,
        client: &mut Client<'_>,
        packet: Packet,
    ) -> Result<(), ProtocolError> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ClientPackets::ConnectRequest) => {
                let request = PacketClientConnectRequest::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::ConnectionRequest(ConnectionRequest {
                        login_token: request.login_token,
                        password_md5: String::from(request.password_md5),
                        response_tx,
                    }))?;
                let packet = match response_rx.await? {
                    Ok(result) => Packet::from(&PacketConnectionReply {
                        result: ConnectResult::Ok,
                        packet_sequence_id: result.packet_sequence_id,
                        pay_flags: 0xff,
                    }),
                    Err(_) => Packet::from(&PacketConnectionReply {
                        result: ConnectResult::Failed,
                        packet_sequence_id: 0,
                        pay_flags: 0,
                    }),
                };
                client.connection.write_packet(packet).await?;
            }
            Some(ClientPackets::CharacterListRequest) => {
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::GetCharacterList(GetCharacterList {
                        response_tx,
                    }))?;
                let response = response_rx.await?;
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerCharacterList {
                        characters: &response[..],
                    }))
                    .await?;
            }
            Some(ClientPackets::CreateCharacter) => {
                let request = PacketClientCreateCharacter::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::CreateCharacter(CreateCharacter {
                        gender: request.gender,
                        birth_stone: request.birth_stone,
                        hair: request.hair,
                        face: request.face,
                        name: String::from(request.name),
                        response_tx,
                    }))?;
                let response = match response_rx.await? {
                    Ok(slot) => Packet::from(&PacketServerCreateCharacterReply {
                        result: CreateCharacterResult::Ok,
                        is_platinum: slot >= 3,
                    }),
                    Err(CreateCharacterError::Failed) => {
                        Packet::from(&PacketServerCreateCharacterReply {
                            result: CreateCharacterResult::Failed,
                            is_platinum: false,
                        })
                    }
                    Err(CreateCharacterError::AlreadyExists) => {
                        Packet::from(&PacketServerCreateCharacterReply {
                            result: CreateCharacterResult::NameAlreadyExists,
                            is_platinum: false,
                        })
                    }
                    Err(CreateCharacterError::InvalidValue) => {
                        Packet::from(&PacketServerCreateCharacterReply {
                            result: CreateCharacterResult::InvalidValue,
                            is_platinum: false,
                        })
                    }
                    Err(CreateCharacterError::NoMoreSlots) => {
                        Packet::from(&PacketServerCreateCharacterReply {
                            result: CreateCharacterResult::NoMoreSlots,
                            is_platinum: false,
                        })
                    }
                };
                client.connection.write_packet(response).await?;
            }
            Some(ClientPackets::DeleteCharacter) => {
                let request = PacketClientDeleteCharacter::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::DeleteCharacter(DeleteCharacter {
                        slot: request.slot,
                        name: String::from(request.name),
                        is_delete: request.is_delete,
                        response_tx,
                    }))?;
                let packet = match response_rx.await? {
                    Ok(response) => Packet::from(&PacketServerDeleteCharacterReply {
                        seconds_until_delete: Some(
                            response
                                .map(|t| t.get_time_until_delete().as_secs())
                                .unwrap_or(0) as u32,
                        ),
                        name: request.name,
                    }),
                    Err(DeleteCharacterError::Failed) => {
                        Packet::from(&PacketServerDeleteCharacterReply {
                            seconds_until_delete: None,
                            name: request.name,
                        })
                    }
                };
                client.connection.write_packet(packet).await?;
            }
            Some(ClientPackets::SelectCharacter) => {
                let request = PacketClientSelectCharacter::try_from(&packet)?;
                let (response_tx, response_rx) = oneshot::channel();
                client
                    .client_message_tx
                    .send(ClientMessage::SelectCharacter(SelectCharacter {
                        slot: request.slot,
                        name: String::from(request.name),
                        response_tx,
                    }))?;
                let packet = match response_rx.await? {
                    Ok(response) => Packet::from(&PacketServerMoveServer {
                        login_token: response.login_token,
                        packet_codec_seed: response.packet_codec_seed,
                        ip: &response.ip,
                        port: response.port,
                    }),
                    Err(_) => return Err(ProtocolError::InvalidPacket),
                };
                client.connection.write_packet(packet).await?;
            }
            _ => warn!(
                "[WS] Unhandled packet [{:#03X}] {:02x?}",
                packet.command,
                &packet.data[..]
            ),
        }

        Ok(())
    }

    async fn handle_server_message(
        &self,
        client: &mut Client<'_>,
        message: ServerMessage,
    ) -> Result<(), ProtocolError> {
        match message {
            ServerMessage::ReturnToCharacterSelect => {
                client
                    .connection
                    .write_packet(Packet::from(&PacketServerReturnToCharacterSelect {}))
                    .await?;
            }
            _ => panic!("Received unexpected server message for world server"),
        }
        Ok(())
    }
}

#[async_trait]
impl ProtocolClient for WorldClient {
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
                server_message = client.server_message_rx.recv() => {
                    if let Some(message) = server_message {
                        self.handle_server_message(client, message).await?;
                    } else {
                        return Err(ProtocolError::ServerInitiatedDisconnect);
                    }
                }
            };
        }
    }
}
