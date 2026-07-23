// content/scripts/skills/parry.rhai
// Parry skill script.
// Restrictions (allowed_classes: ["warrior"]) are enforced automatically by engine CommandRestrictions.
// Parry must be explicitly activated during combat, and automatically deactivates when combat ends.
// Deflect success rate scales with practice level (max 60% success chance at 100% practice level).

fn on_use() {
    let rank = get_skill_rank(actor, "parry");
    if rank == 0 {
        send("You have not practiced the art of parrying.");
        return;
    }

    if has_script_effect(actor, "parrying") {
        remove_script_effect(actor, "parrying");
        send("You lower your guard and stop attempting to parry incoming attacks.");
        echo(actor.name() + " lowers their guard.");
        return;
    }

    let chance = (rank * 60) / 100;
    apply_script_effect_full(
        actor,
        "parrying",
        "Parrying Stance",
        "Parry",
        3600,
        "Actively parrying incoming melee attacks (Max 60% deflect chance)",
        "",
        "",
        "",
        "",
        "You lower your guard and stop parrying.",
        #{
            "expire_conditions": ["exit_combat"]
        }
    );

    send("You focus on parrying incoming attacks! (Parry Rank: " + rank + "%, Deflect Chance: " + chance + "%)");
    echo(actor.name() + " readies their weapon, watching for incoming strikes.");
}

fn on_combat_hit() {
    // Parry only applies if the defender has actively enabled parrying stance
    if !has_script_effect(hit_ctx.target, "parrying") {
        return hit_ctx;
    }

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

        // Scoped room broadcast excluding defender and attacker
        echo_except(defender_name + " parries " + attacker_name + "'s attack!", [hit_ctx.target, hit_ctx.attacker]);

        hit_ctx.abort("");
    }

    return hit_ctx;
}
