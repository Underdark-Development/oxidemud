// content/scripts/spells/chain_lightning.rhai
// Chain Lightning spell script.
// Restrictions (allowed_classes: ["mage", "wizard"]) enforced automatically by CommandRestrictions.
// Hits 1st target for 3d8 lightning damage, 75% chance to bounce to 2nd target for 2d6, 50% chance to bounce to 3rd for 1d6.

fn on_cast() {
    if is_on_cooldown(actor, "chain_lightning") {
        send("Your hands are still tingling from the last spell cast.");
        return;
    }

    let occupants = actor.room().entities();
    let targets = [];
    for ent in occupants {
        if ent != actor && ent.is_alive() {
            targets.push(ent);
        }
    }

    if targets.len() == 0 {
        send("There is no one here to strike with chain lightning!");
        return;
    }

    let primary = targets[0];
    let primary_name = primary.name();

    send("A crackling arc of electric energy surges from your fingertips toward " + primary_name + "!");
    echo_except(actor.name() + " unleashes a blinding arc of chain lightning at " + primary_name + "!", [actor]);

    let dmg1 = rand(3, 24);
    primary.damage(dmg1, "lightning");
    primary.send("A devastating bolt of lightning strikes you for " + dmg1 + " damage!");

    set_cooldown(actor, "chain_lightning", 10);

    if targets.len() > 1 && rand(1, 100) <= 75 {
        let sec = targets[1];
        let sec_name = sec.name();
        let dmg2 = rand(2, 12);
        echo("The lightning arcs sharply from " + primary_name + " to strike " + sec_name + "!");
        sec.damage(dmg2, "lightning");
        sec.send("The arcing lightning shocks you for " + dmg2 + " damage!");

        if targets.len() > 2 && rand(1, 100) <= 50 {
            let tert = targets[2];
            let tert_name = tert.name();
            let dmg3 = rand(1, 6);
            echo("A residual spark jumps from " + sec_name + " to singe " + tert_name + "!");
            tert.damage(dmg3, "lightning");
            tert.send("A residual spark singes you for " + dmg3 + " damage!");
        }
    }
}
