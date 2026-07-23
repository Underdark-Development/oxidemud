// content/scripts/skills/parry.rhai
// Parry skill script.
// Restrictions (allowed_classes: ["warrior"]) are enforced automatically by engine CommandRestrictions.
// Deflect success rate scales with practice level (max 60% success chance at 100% practice level).

fn on_use() {
    let rank = get_skill_rank(actor, "parry");
    if rank == 0 {
        send("You have not practiced the art of parrying.");
        return;
    }

    let chance = (rank * 60) / 100;
    send("You focus on parrying incoming attacks! (Parry Rank: " + rank + "%, Deflect Chance: " + chance + "%)");
    echo(actor.name() + " readies their weapon, watching for incoming strikes.");
}

fn on_combat_hit() {
    let rank = get_skill_rank(hit_ctx.target, "parry");
    if rank == 0 {
        return hit_ctx;
    }

    // Success rate depends on practice level: max 60% chance at 100% practice
    let chance = (rank * 60) / 100;
    if rand(1, 100) <= chance {
        let defender_name = hit_ctx.target.name();
        let attacker_name = hit_ctx.attacker.name();

        // Direct messages to defender and attacker
        hit_ctx.target.send("You parry " + attacker_name + "'s attack!");
        hit_ctx.attacker.send(defender_name + " parries your attack!");

        // Scoped room broadcast excluding defender and attacker (no room entity parameter needed)
        echo_except(defender_name + " parries " + attacker_name + "'s attack!", [hit_ctx.target, hit_ctx.attacker]);

        hit_ctx.abort("");
    }

    return hit_ctx;
}
