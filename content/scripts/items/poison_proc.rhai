// content/scripts/items/poison_proc.rhai
// Proc trigger script when attacking with poison_dagger

fn on_trigger() {
    if target.is_alive() {
        target.send("You feel a burning poison enter your bloodstream!");
        let room = target.room();
        room.echo(target.name() + " gasps as venom enters their wounds!");

        // Apply 2-second (2 round) poison status effect
        apply_script_effect_full(
            world,
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
