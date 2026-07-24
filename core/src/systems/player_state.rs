use crate::{Entity, PlayerState, RestState, World};
use std::time::Duration;

/// Triggers that cause a transition in the player state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlayerStateTrigger {
    Sit,
    Rest,
    Sleep,
    Wake,
    Stand,
    Knockout,
    Revive,
    Die,
    Stun { duration_ms: u64 },
    Cast { duration_ms: u64 },
    TickDecay { elapsed_ms: u64 },
}

/// Centralized state transition matrix for player resting/activity states.
pub fn transition_player_state(
    current: &PlayerState,
    trigger: PlayerStateTrigger,
) -> Result<PlayerState, &'static str> {
    match current {
        PlayerState::Dead => match trigger {
            PlayerStateTrigger::Revive => Ok(PlayerState::Resting(RestState::Standing)),
            PlayerStateTrigger::Die => Err("You are already dead."),
            _ => Err("You are a ghost and cannot do that."),
        },
        PlayerState::Stunned { remaining_ms } => match trigger {
            PlayerStateTrigger::Die => Ok(PlayerState::Dead),
            PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
            PlayerStateTrigger::TickDecay { elapsed_ms } => {
                if elapsed_ms >= *remaining_ms {
                    Ok(PlayerState::Resting(RestState::Standing))
                } else {
                    Ok(PlayerState::Stunned {
                        remaining_ms: remaining_ms.saturating_sub(elapsed_ms),
                    })
                }
            }
            _ => Err("You are stunned and cannot move."),
        },
        PlayerState::Casting { remaining_ms } => match trigger {
            PlayerStateTrigger::Die => Ok(PlayerState::Dead),
            PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
            PlayerStateTrigger::Stun { duration_ms } => Ok(PlayerState::Stunned {
                remaining_ms: duration_ms,
            }),
            PlayerStateTrigger::TickDecay { elapsed_ms } => {
                if elapsed_ms >= *remaining_ms {
                    Ok(PlayerState::Resting(RestState::Standing))
                } else {
                    Ok(PlayerState::Casting {
                        remaining_ms: remaining_ms.saturating_sub(elapsed_ms),
                    })
                }
            }
            _ => Err("You are too busy casting to do that."),
        },
        PlayerState::Resting(rest_state) => match rest_state {
            RestState::Unconscious => match trigger {
                PlayerStateTrigger::Die => Ok(PlayerState::Dead),
                PlayerStateTrigger::Revive => Ok(PlayerState::Resting(RestState::Standing)),
                _ => Err("You are unconscious."),
            },
            RestState::Dead => match trigger {
                PlayerStateTrigger::Revive => Ok(PlayerState::Resting(RestState::Standing)),
                PlayerStateTrigger::Die => Err("You are already dead."),
                _ => Err("You are a ghost and cannot do that."),
            },
            RestState::Sleeping => match trigger {
                PlayerStateTrigger::Die => Ok(PlayerState::Dead),
                PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
                PlayerStateTrigger::Wake | PlayerStateTrigger::Stand => {
                    Ok(PlayerState::Resting(RestState::Standing))
                }
                PlayerStateTrigger::Rest => Ok(PlayerState::Resting(RestState::Resting)),
                PlayerStateTrigger::Sleep => Err("You are already sleeping."),
                PlayerStateTrigger::Sit => Ok(PlayerState::Resting(RestState::Sitting)),
                PlayerStateTrigger::Stun { duration_ms } => Ok(PlayerState::Stunned {
                    remaining_ms: duration_ms,
                }),
                _ => Err("You must wake up first."),
            },
            RestState::Resting => match trigger {
                PlayerStateTrigger::Die => Ok(PlayerState::Dead),
                PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
                PlayerStateTrigger::Stand | PlayerStateTrigger::Wake => {
                    Ok(PlayerState::Resting(RestState::Standing))
                }
                PlayerStateTrigger::Sit => Ok(PlayerState::Resting(RestState::Sitting)),
                PlayerStateTrigger::Rest => Err("You are already resting."),
                PlayerStateTrigger::Sleep => Ok(PlayerState::Resting(RestState::Sleeping)),
                PlayerStateTrigger::Stun { duration_ms } => Ok(PlayerState::Stunned {
                    remaining_ms: duration_ms,
                }),
                PlayerStateTrigger::Cast { duration_ms } => Ok(PlayerState::Casting {
                    remaining_ms: duration_ms,
                }),
                _ => Ok(current.clone()),
            },
            RestState::Sitting => match trigger {
                PlayerStateTrigger::Die => Ok(PlayerState::Dead),
                PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
                PlayerStateTrigger::Stand | PlayerStateTrigger::Wake => {
                    Ok(PlayerState::Resting(RestState::Standing))
                }
                PlayerStateTrigger::Rest => Ok(PlayerState::Resting(RestState::Resting)),
                PlayerStateTrigger::Sit => Err("You are already sitting."),
                PlayerStateTrigger::Sleep => Ok(PlayerState::Resting(RestState::Sleeping)),
                PlayerStateTrigger::Stun { duration_ms } => Ok(PlayerState::Stunned {
                    remaining_ms: duration_ms,
                }),
                PlayerStateTrigger::Cast { duration_ms } => Ok(PlayerState::Casting {
                    remaining_ms: duration_ms,
                }),
                _ => Ok(current.clone()),
            },
            RestState::Standing => match trigger {
                PlayerStateTrigger::Die => Ok(PlayerState::Dead),
                PlayerStateTrigger::Knockout => Ok(PlayerState::Resting(RestState::Unconscious)),
                PlayerStateTrigger::Sit => Ok(PlayerState::Resting(RestState::Sitting)),
                PlayerStateTrigger::Rest => Ok(PlayerState::Resting(RestState::Resting)),
                PlayerStateTrigger::Sleep => Ok(PlayerState::Resting(RestState::Sleeping)),
                PlayerStateTrigger::Stand => Err("You are already standing."),
                PlayerStateTrigger::Wake => Err("You are already awake."),
                PlayerStateTrigger::Stun { duration_ms } => Ok(PlayerState::Stunned {
                    remaining_ms: duration_ms,
                }),
                PlayerStateTrigger::Cast { duration_ms } => Ok(PlayerState::Casting {
                    remaining_ms: duration_ms,
                }),
                _ => Ok(current.clone()),
            },
        },
    }
}

/// Attempt to transition a player's state. If successful, updates the component
/// and returns the new state. Also logs/emits PlayerStateChanged.
pub fn try_transition_player_state(
    world: &mut World,
    entity: Entity,
    trigger: PlayerStateTrigger,
) -> Result<PlayerState, &'static str> {
    let current_state = world
        .query_one::<&PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned())
        .unwrap_or_default();

    let new_state = transition_player_state(&current_state, trigger)?;

    if new_state != current_state {
        let _ = world.insert(entity, (new_state.clone(), crate::Dirty));
    }

    Ok(new_state)
}

/// Decay the remaining duration of Stunned and Casting states for all entities.
/// If a timer reaches 0, the entity transitions back to Resting(RestState::Standing).
/// Returns a list of entities that transitioned, along with their old and new states.
pub fn run_player_state_decay(
    world: &mut World,
    elapsed: Duration,
) -> Vec<(Entity, PlayerState, PlayerState)> {
    let mut transitions = Vec::new();
    let elapsed_ms = elapsed.as_millis() as u64;

    let mut targets = Vec::new();
    {
        let mut q = world.query::<&PlayerState>();
        for (raw, state) in q.iter() {
            let entity = raw;
            match state {
                PlayerState::Stunned { remaining_ms } => {
                    targets.push((entity, state.clone(), *remaining_ms));
                }
                PlayerState::Casting { remaining_ms } => {
                    targets.push((entity, state.clone(), *remaining_ms));
                }
                _ => {}
            }
        }
    }

    for (entity, old_state, _) in targets {
        if let Ok(new_state) =
            try_transition_player_state(world, entity, PlayerStateTrigger::TickDecay { elapsed_ms })
        {
            let expired = new_state == PlayerState::Resting(RestState::Standing);
            if expired && new_state != old_state {
                transitions.push((entity, old_state, new_state));
            }
        }
    }

    transitions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Name;

    #[test]
    fn test_stun_decay() {
        let mut world = World::new();
        let entity = world.spawn((
            PlayerState::Stunned { remaining_ms: 1000 },
            Name::new("TestPlayer"),
        ));

        // 500ms passes
        let transitions = run_player_state_decay(&mut world, Duration::from_millis(500));
        assert!(transitions.is_empty());

        let state = world
            .query_one::<&PlayerState>(entity)
            .unwrap()
            .get()
            .unwrap()
            .clone();
        assert_eq!(state, PlayerState::Stunned { remaining_ms: 500 });

        // Another 600ms passes (exceeding remaining 500ms)
        let transitions = run_player_state_decay(&mut world, Duration::from_millis(600));
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].0, entity);
        assert_eq!(transitions[0].1, PlayerState::Stunned { remaining_ms: 500 });
        assert_eq!(transitions[0].2, PlayerState::Resting(RestState::Standing));

        let state = world
            .query_one::<&PlayerState>(entity)
            .unwrap()
            .get()
            .unwrap()
            .clone();
        assert_eq!(state, PlayerState::Resting(RestState::Standing));
    }

    #[test]
    fn test_casting_decay() {
        let mut world = World::new();
        let _entity = world.spawn((
            PlayerState::Casting { remaining_ms: 2000 },
            Name::new("TestCaster"),
        ));

        // 2000ms passes
        let transitions = run_player_state_decay(&mut world, Duration::from_millis(2000));
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].2, PlayerState::Resting(RestState::Standing));
    }

    #[test]
    fn test_transition_matrix() {
        // 1. Standing transitions
        let standing = PlayerState::Resting(RestState::Standing);
        assert_eq!(
            transition_player_state(&standing, PlayerStateTrigger::Sit),
            Ok(PlayerState::Resting(RestState::Sitting))
        );
        assert_eq!(
            transition_player_state(&standing, PlayerStateTrigger::Rest),
            Ok(PlayerState::Resting(RestState::Resting))
        );
        assert_eq!(
            transition_player_state(&standing, PlayerStateTrigger::Sleep),
            Ok(PlayerState::Resting(RestState::Sleeping))
        );
        assert!(transition_player_state(&standing, PlayerStateTrigger::Stand).is_err());

        // 2. Sleeping transitions
        let sleeping = PlayerState::Resting(RestState::Sleeping);
        assert_eq!(
            transition_player_state(&sleeping, PlayerStateTrigger::Wake),
            Ok(PlayerState::Resting(RestState::Standing))
        );
        assert_eq!(
            transition_player_state(&sleeping, PlayerStateTrigger::Stand),
            Ok(PlayerState::Resting(RestState::Standing))
        );
        assert_eq!(
            transition_player_state(&sleeping, PlayerStateTrigger::Rest),
            Ok(PlayerState::Resting(RestState::Resting))
        );
        assert_eq!(
            transition_player_state(&sleeping, PlayerStateTrigger::Sit),
            Ok(PlayerState::Resting(RestState::Sitting))
        );

        // 3. Dead transitions
        let dead = PlayerState::Dead;
        assert_eq!(
            transition_player_state(&dead, PlayerStateTrigger::Revive),
            Ok(PlayerState::Resting(RestState::Standing))
        );
        assert!(transition_player_state(&dead, PlayerStateTrigger::Sit).is_err());

        // 4. Stunned / Casting transitions
        let stunned = PlayerState::Stunned { remaining_ms: 1000 };
        assert!(transition_player_state(&stunned, PlayerStateTrigger::Sit).is_err());
        assert_eq!(
            transition_player_state(&stunned, PlayerStateTrigger::Knockout),
            Ok(PlayerState::Resting(RestState::Unconscious))
        );

        let casting = PlayerState::Casting { remaining_ms: 2000 };
        assert_eq!(
            transition_player_state(&casting, PlayerStateTrigger::Stun { duration_ms: 1000 }),
            Ok(PlayerState::Stunned { remaining_ms: 1000 })
        );
        assert!(transition_player_state(&casting, PlayerStateTrigger::Sit).is_err());
    }
}
