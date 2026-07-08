use crate::{Entity, PlayerState, RestState, World};
use std::time::Duration;

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
            let entity = Entity::from(raw);
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

    for (entity, old_state, remaining_ms) in targets {
        let new_ms = remaining_ms.saturating_sub(elapsed_ms);
        let expired = new_ms == 0;
        let new_state = if expired {
            PlayerState::Resting(RestState::Standing)
        } else {
            match old_state {
                PlayerState::Stunned { .. } => PlayerState::Stunned {
                    remaining_ms: new_ms,
                },
                PlayerState::Casting { .. } => PlayerState::Casting {
                    remaining_ms: new_ms,
                },
                _ => unreachable!(),
            }
        };

        if new_state != old_state {
            let _ = world.insert(entity, (new_state.clone(), crate::Dirty));

            if expired {
                // Emit transition event
                let _event = crate::GameEvent::PlayerStateChanged {
                    entity,
                    from: old_state.clone(),
                    to: new_state.clone(),
                };

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
        let entity = world.spawn((
            PlayerState::Casting { remaining_ms: 2000 },
            Name::new("TestCaster"),
        ));

        // 2000ms passes
        let transitions = run_player_state_decay(&mut world, Duration::from_millis(2000));
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].2, PlayerState::Resting(RestState::Standing));
    }
}
