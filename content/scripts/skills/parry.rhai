fn on_combat_hit() {
    let rank = world.get_skill_rank(hit_ctx.target, "parry");
    if rank == 0 {
        return hit_ctx;
    }
    let chance = rank / 2;
    if rand(1, 100) <= chance {
        let attacker_name = world.get_name(hit_ctx.attacker);
        hit_ctx.abort("You parry " + attacker_name + "'s attack!");
    }
    return hit_ctx;
}
