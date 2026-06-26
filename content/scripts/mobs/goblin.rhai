fn on_ai_pulse() {
    let hp = world.get_hp(self);
    let max_hp = world.get_max_hp(self);
    if hp > 0 && hp * 5 < max_hp {
        world.echo_room(self, world.get_name(self) + " screams: 'Arrgh! Me bleed! Run away!'");
        
        let room = world.get_room(self);
        let exits = world.room_exits(room);
        if exits.len() > 0 {
            let r_idx = rand(0, exits.len() - 1);
            let dir = exits[r_idx];
            let dest = world.room_exit_target(room, dir);
            
            world.move_entity(self, dest);
            world.echo_room(self, world.get_name(self) + " fled to the " + dir + ".");
        }
        return "wander";
    }
    return ();
}
