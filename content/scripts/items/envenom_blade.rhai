// content/scripts/items/envenom_blade.rhai
// Custom item command script for "envenom" on the Venomous Serpent Dagger.

fn on_command() {
    if is_on_cooldown(actor, "envenom") {
        send("The blade is already glistening with fresh venom.");
        return;
    }

    send("You apply fresh venom to the serpent blade!");
    echo(actor.name() + " runs a vial of dark green venom along their blade.");

    apply_script_effect_full(
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

    set_cooldown(actor, "envenom", 45);
}
