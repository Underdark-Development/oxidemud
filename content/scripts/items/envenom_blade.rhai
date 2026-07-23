// content/scripts/items/envenom_blade.rhai
// Action command granted by poison_dagger item when equipped

fn on_command() {
    if is_on_cooldown(world, actor, "envenom") {
        actor.send("The blade is already glistening with fresh venom.");
        return;
    }

    actor.send("You apply fresh venom to the serpent blade!");
    let room = actor.room();
    room.echo(actor.name() + " runs a vial of dark green venom along their blade.");

    apply_script_effect_full(
        world,
        self,
        "envenomed_blade",
        "Envenomed",
        "Envenom Blade",
        60,
        "Coated in virulent poison (+25% poison proc chance)",
        "a dripping venomous ",
        "",
        "a serpent dagger dripping with dark green venom",
        "drips with thick green poison.",
        "The extra venom wears off the blade.",
        #{}
    );

    set_cooldown(world, actor, "envenom", 45);
}
