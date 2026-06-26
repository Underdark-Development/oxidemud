fn on_say() {
    if message.to_lower() == "open sesame" {
        if world.is_exit_closed(self, "north") || world.is_exit_locked(self, "north") {
            world.set_exit_locked(self, "north", false);
            world.set_exit_closed(self, "north", false);
            world.echo_room(self, "At the magic words, the north door opens with a grinding rumble!");
        }
    }
}
