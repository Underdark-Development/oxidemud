fn on_say() {
    // If dynamic parameters keyword and direction are not provided, fallback to defaults
    let kw = if is_def_var("keyword") { keyword } else { "open sesame" };
    let dir = if is_def_var("direction") { direction } else { "north" };

    if message.to_lower() == kw.to_lower() {
        if is_exit_closed(self, dir) || is_exit_locked(self, dir) {
            set_exit_locked(self, dir, false);
            set_exit_closed(self, dir, false);
            echo("At the magic words, the " + dir + " door opens with a grinding rumble!");
        }
    }
}
