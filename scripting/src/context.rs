use oxide_core::{Entity, World};

thread_local! {
    pub(crate) static CURRENT_SCRIPT_CONTEXT: std::cell::RefCell<Option<ScriptExecContext>> = const { std::cell::RefCell::new(None) };
}

#[derive(Debug, Clone, Copy)]
pub struct ScriptExecContext {
    pub entity: Entity,
    pub actor: Option<Entity>,
    pub target: Option<Entity>,
    pub room: Option<Entity>,
    world_ptr: *mut World,
}

pub struct ScriptContextGuard;

impl Drop for ScriptContextGuard {
    fn drop(&mut self) {
        CURRENT_SCRIPT_CONTEXT.with(|c| *c.borrow_mut() = None);
    }
}

pub fn push_script_context(
    entity: Entity,
    actor: Option<Entity>,
    target: Option<Entity>,
    world: &mut World,
) -> ScriptContextGuard {
    let active_entity = actor.unwrap_or(entity);
    let room = world
        .query_one::<&oxide_core::Position>(active_entity)
        .ok()
        .and_then(|mut q| q.get().map(|p| p.room))
        .or_else(|| {
            world
                .query_one::<&oxide_core::Position>(entity)
                .ok()
                .and_then(|mut q| q.get().map(|p| p.room))
        });

    CURRENT_SCRIPT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ScriptExecContext {
            entity,
            actor,
            target,
            room,
            world_ptr: world as *mut World,
        });
    });

    ScriptContextGuard
}

pub fn with_current_world<R>(f: impl FnOnce(&mut World) -> R) -> Option<R> {
    CURRENT_SCRIPT_CONTEXT.with(|c| {
        if let Some(ctx) = *c.borrow() {
            // SAFETY: `world_ptr` is set from a valid `&mut World` in `push_script_context`
            // on the current thread, and cleared when `ScriptContextGuard` drops at the end of
            // the script execution scope. The reference is non-null and valid for the duration
            // of `with_current_world` because script execution is synchronous and single-threaded.
            unsafe {
                let w = &mut *ctx.world_ptr;
                Some(f(w))
            }
        } else {
            None
        }
    })
}
