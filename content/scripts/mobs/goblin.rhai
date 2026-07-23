fn on_ai_pulse() {
    let hp = get_hp(self);
    let max_hp = get_max_hp(self);
    if hp > 0 && hp * 5 < max_hp {
        echo(get_name(self) + " screams: 'Arrgh! Me bleed! Run away!'");

        let room = self.room();
        let exits = room_exits(room);
        if exits.len() > 0 {
            let r_idx = rand(0, exits.len() - 1);
            let dir = exits[r_idx];
            let dest = room_exit_target(room, dir);

            move_entity(self, dest);
            echo(get_name(self) + " fled to the " + dir + ".");
        }
        return "wander";
    }
    return ();
}
