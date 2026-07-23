// content/scripts/skills/parry.rhai
// Parry skill script. Usage restrictions (allowed_classes: ["warrior"]) are enforced
// automatically by the engine's built-in CommandRestrictions prior to script execution.

fn on_use() {
    if is_on_cooldown(world, actor, "parry") {
        actor.send("You must wait before parrying again.");
        return;
    }

    actor.send("You adopt a tight defensive stance, preparing to parry incoming blows!");
    let room = actor.room();
    room.echo(actor.name() + " readies their weapon in a defensive stance.");

    // Apply temporary defensive parry effect for 12 seconds
    apply_script_effect_full(
        world,
        actor,
        "parrying",
        "Parrying Stance",
        "Parry",
        12,
        "Ready to parry incoming attacks (+4 Armor Class)",
        "",
        "",
        "",
        "is holding a tight parrying guard.",
        "Your parrying guard drops.",
        #{}
    );

    set_cooldown(world, actor, "parry", 15);
}
