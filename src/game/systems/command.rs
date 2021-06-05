use std::time::Duration;

use legion::{system, systems::CommandBuffer, world::SubWorld, Entity, EntityStore};
use nalgebra::Point3;

use crate::game::{
    components::{
        ClientEntity, Command, CommandAttack, CommandData, CommandMove, Destination, NextCommand,
        Position,
    },
    messages::server::{self, ServerMessage},
    resources::{DeltaTime, ServerMessages},
};

fn set_command_stop(
    command: &mut Command,
    cmd: &mut CommandBuffer,
    entity: &Entity,
    entity_id: &ClientEntity,
    position: &Position,
    server_messages: &mut ServerMessages,
) {
    // Remove all components associated with other actions
    cmd.remove_component::<Destination>(*entity);

    server_messages.send_entity_message(
        *entity,
        ServerMessage::StopMoveEntity(server::StopMoveEntity {
            entity_id: entity_id.id.0,
            x: position.position.x,
            y: position.position.y,
            z: position.position.z as u16,
        }),
    );

    *command = Command::new(CommandData::Stop, None);
}

#[system(for_each)]
#[read_component(ClientEntity)]
#[read_component(Position)]
pub fn command(
    world: &SubWorld,
    cmd: &mut CommandBuffer,
    entity: &Entity,
    entity_id: &ClientEntity,
    position: &Position,
    command: &mut Command,
    next_command: Option<&NextCommand>,
    #[resource] delta_time: &DeltaTime,
    #[resource] server_messages: &mut ServerMessages,
) {
    command.duration += delta_time.delta;

    let command_complete = if let Some(required_duration) = command.required_duration.as_ref() {
        command.duration > *required_duration
    } else {
        true
    };

    if command_complete {
        if let Some(next_command) = next_command {
            match next_command.0 {
                CommandData::Stop => {
                    set_command_stop(command, cmd, entity, entity_id, position, server_messages);
                    cmd.remove_component::<NextCommand>(*entity);
                }
                CommandData::Move(CommandMove {
                    destination,
                    target,
                }) => {
                    cmd.add_component(
                        *entity,
                        Destination {
                            position: destination,
                        },
                    );

                    let mut target_entity_id = 0;
                    if let Some(target_entity) = target {
                        if let Ok(entry) = world.entry_ref(target_entity) {
                            if let Ok(target_client_entity) = entry.get_component::<ClientEntity>()
                            {
                                target_entity_id = target_client_entity.id.0;
                            }
                        }
                    }

                    let distance = (destination.xy() - position.position.xy()).magnitude();
                    server_messages.send_entity_message(
                        *entity,
                        ServerMessage::MoveEntity(server::MoveEntity {
                            entity_id: entity_id.id.0,
                            target_entity_id,
                            distance: distance as u16,
                            x: destination.x,
                            y: destination.y,
                            z: destination.z as u16,
                        }),
                    );

                    *command = Command::new(
                        CommandData::Move(CommandMove {
                            destination,
                            target,
                        }),
                        None,
                    );
                    cmd.remove_component::<NextCommand>(*entity);
                }
                CommandData::Attack(CommandAttack { target }) => {
                    let mut valid_attack_target = false;
                    if let Ok(entry) = world.entry_ref(target) {
                        if let Ok(target_client_entity) = entry.get_component::<ClientEntity>() {
                            if let Ok(target_position) = entry.get_component::<Position>() {
                                if target_position.zone == position.zone {
                                    let distance = (target_position.position.xy()
                                        - position.position.xy())
                                    .magnitude();
                                    let attack_range = 70.0 + 120.0; // TODO: Get correct attack range for entity

                                    // Check if we have just started attacking this target
                                    let attack_started = match command.command {
                                        CommandData::Attack(CommandAttack {
                                            target: current_attack_target,
                                            ..
                                        }) => current_attack_target != target,
                                        CommandData::Move(CommandMove {
                                            target: Some(current_attack_target),
                                            ..
                                        }) => current_attack_target != target,
                                        _ => true,
                                    };

                                    if distance < attack_range {
                                        println!("CMD: In attack range, starting attack animation");
                                        // TODO: Get correct attack duration for entity
                                        let attack_duration = Duration::from_secs(1);
                                        *command = Command::new(
                                            CommandData::Attack(CommandAttack { target }),
                                            Some(attack_duration),
                                        );
                                        cmd.remove_component::<Destination>(*entity);
                                    } else {
                                        println!("CMD: Out of attack range, moving to target");
                                        *command = Command::new(
                                            CommandData::Move(CommandMove {
                                                destination: target_position.position,
                                                target: Some(target),
                                            }),
                                            None,
                                        );
                                        cmd.add_component(
                                            *entity,
                                            Destination {
                                                position: target_position.position,
                                            },
                                        );
                                    }

                                    if attack_started {
                                        println!("CMD: Sending attack packet");
                                        server_messages.send_entity_message(
                                            *entity,
                                            ServerMessage::AttackEntity(server::AttackEntity {
                                                entity_id: entity_id.id.0,
                                                target_entity_id: target_client_entity.id.0,
                                                distance: distance as u16,
                                                x: target_position.position.x,
                                                y: target_position.position.y,
                                                z: target_position.position.z as u16,
                                            }),
                                        );
                                    }

                                    valid_attack_target = true;
                                }
                            }
                        }
                    }

                    if !valid_attack_target {
                        println!("CMD: Invalid attack target, stopping");
                        set_command_stop(
                            command,
                            cmd,
                            entity,
                            entity_id,
                            position,
                            server_messages,
                        );
                        cmd.remove_component::<NextCommand>(*entity);
                    }
                }
            }
        }
    } else {
        println!("CMD: Current command duration: {:?}", command.duration);
    }
}
