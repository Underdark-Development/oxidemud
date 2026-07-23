use oxide_core::{DamageType, HitContext};
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    // HitContext bindings
    engine.register_get("attacker", |ctx: &mut HitContext| ctx.attacker);
    engine.register_get("target", |ctx: &mut HitContext| ctx.target);
    engine.register_get("is_offhand", |ctx: &mut HitContext| ctx.is_offhand);
    engine.register_get_set(
        "is_aborted",
        |ctx: &mut HitContext| ctx.is_aborted,
        |ctx: &mut HitContext, val: bool| ctx.is_aborted = val,
    );
    engine.register_get_set(
        "hit_modifier",
        |ctx: &mut HitContext| ctx.hit_modifier as i64,
        |ctx: &mut HitContext, val: i64| ctx.hit_modifier = val as i32,
    );
    engine.register_fn("abort", |ctx: &mut HitContext, reason: String| {
        ctx.is_aborted = true;
        ctx.abort_reason = Some(reason);
    });
    engine.register_fn("override_hit", |ctx: &mut HitContext, outcome: bool| {
        ctx.override_hit = Some(outcome);
    });

    // DamageType bindings
    engine.register_fn("to_string", |dt: DamageType| format!("{:?}", dt));
}
