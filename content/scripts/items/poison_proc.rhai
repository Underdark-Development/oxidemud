// content/scripts/items/poison_proc.rhai
// Weapon on_hit proc trigger script for Venomous Serpent Dagger.

fn on_trigger() {
    if target.is_alive() {
        target.send("You feel a burning poison enter your bloodstream!");
        echo_except(target.name() + " gasps as venom enters their wounds!", [target]);

        apply_script_effect_full(
            target,
            "serpent_poison",
            "Poisoned",
            "Serpent Venom",
            2,
            "Suffering from deadly serpent poison (-2 Con, 5 poison damage/sec)",
            "",
            " (poisoned)",
            "",
            "looks pale and is shivering from poison.",
            "The serpent venom leaves your veins.",
            #{}
        );

        target.damage(5, "poison");
    }
}
