// content/scripts/skills/parry.rhai
// Parry skill script.
// Restrictions (allowed_classes: ["warrior"]) are enforced automatically by engine CommandRestrictions.
// Deflect success rate scales with practice level (max 60% success chance at 100% practice level).

fn on_use() {
    let rank = world.get_skill_rank(actor, "parry");
    if rank == 0 {
        actor.send("You have not practiced the art of parrying.");
        return;
    }

    let chance = (rank * 60) / 100;
    actor.send("You focus on parrying incoming attacks! (Parry Rank: " + rank + "%, Deflect Chance: " + chance + "%)");
    let room = actor.room();
    room.echo(actor.name() + " readies their weapon, watching for incoming strikes.");
}

fn on_combat_hit() {
    let rank = world.get_skill_rank(hit_ctx.target, "parry");
    if rank == 0 {
        return hit_ctx;
    }

    // Success rate depends on practice level: max 60% chance at 100% practice
    let chance = (rank * 60) / 100;
    if rand(1, 100) <= chance {
        let defender_name = world.get_name(hit_ctx.target);
        let attacker_name = world.get_name(hit_ctx.attacker);

        // Notify defender, attacker, and room spectators
        send_to(hit_ctx.target, "You parry " + attacker_name + "'s attack!");
        send_to(hit_ctx.attacker, defender_name + " parries your attack!");
        echo_room(world, hit_ctx.target, defender_name + " parries " + attacker_name + "'s attack!");

        hit_ctx.abort("");
    }

    return hit_ctx;
}
