// content/scripts/spells/chain_lightning.rhai
// Chain Lightning spell script. Usage restrictions (allowed_classes: ["mage", "wizard"]) are
// enforced automatically by the engine's built-in CommandRestrictions prior to script execution.

fn on_cast() {
    if is_on_cooldown(world, actor, "chain_lightning") {
        actor.send("Your hands are still tingling from the last spell cast.");
        return;
    }

    let room = actor.room();
    let occupants = room.entities();
    
    // Find valid hostile/target entities in room (excluding actor)
    let targets = [];
    for ent in occupants {
        if ent != actor && ent.is_alive() {
            targets.push(ent);
        }
    }

    if targets.len() == 0 {
        actor.send("There is no one here to strike with chain lightning!");
        return;
    }

    // Target selection: primary target
    let primary = targets[0];
    let primary_name = primary.name();

    actor.send("A crackling arc of electric energy surges from your fingertips toward " + primary_name + "!");
    room.echo(actor.name() + " unleashes a blinding arc of chain lightning at " + primary_name + "!");

    // Primary hit (3d8 lightning damage)
    let dmg1 = rand(3, 24);
    primary.damage(dmg1, "lightning");
    primary.send("A devastating bolt of lightning strikes you for " + dmg1 + " damage!");

    // Set cooldown
    set_cooldown(world, actor, "chain_lightning", 10);

    // Bounce 1: 75% chance to hit a 2nd target for reduced damage (2d6)
    if targets.len() > 1 && rand(1, 100) <= 75 {
        let sec = targets[1];
        let sec_name = sec.name();
        let dmg2 = rand(2, 12);
        room.echo("The lightning arcs sharply from " + primary_name + " to strike " + sec_name + "!");
        sec.damage(dmg2, "lightning");
        sec.send("The arcing lightning shocks you for " + dmg2 + " damage!");

        // Bounce 2: 50% chance to hit a 3rd target for reduced damage (1d6)
        if targets.len() > 2 && rand(1, 100) <= 50 {
            let tert = targets[2];
            let tert_name = tert.name();
            let dmg3 = rand(1, 6);
            room.echo("A residual spark jumps from " + sec_name + " to singe " + tert_name + "!");
            tert.damage(dmg3, "lightning");
            tert.send("A residual spark singes you for " + dmg3 + " damage!");
        }
    }
}
