use super::conventions;
use super::parse_tags;
use super::{RichText, Segment};
use crate::templates::{ItemTemplate, MobTemplate};

/// Full room rendering as `look` would show it in-game.
///
/// Produces: room name (brightwhite bold), separator, description (color-tag
/// parsed), exits list (cyan), then mob and item names on separate lines.
pub fn room_look(
    name: &str,
    description: &str,
    exit_directions: &[String],
    mob_names: &[String],
    item_names: &[String],
) -> RichText {
    let mut rt = RichText::new();

    rt.extend(conventions::room_name(name));
    rt.push_str("\n");
    rt.extend(parse_tags(description));
    rt.push_str("\n");

    if !exit_directions.is_empty() {
        rt.push_str("[Exits: ");
        rt.extend(conventions::exit_dir(exit_directions.join(" ")));
        rt.push_str("]\n");
    }

    for mob_name in mob_names {
        rt.push(Segment::new(mob_name.to_string()));
        rt.push_str("\n");
    }
    for item_name in item_names {
        rt.push(Segment::new(item_name.to_string()));
        rt.push_str("\n");
    }

    rt
}

/// How a mob appears when seen in a room listing.
///
/// Returns the mob's name as a plain text segment, matching how `cmd_look`
/// lists NPCs in the room.
pub fn mob_room_appearance(name: &str) -> RichText {
    RichText::from(Segment::new(name.to_string()))
}

/// Full mob examination as `look at <mob>` would show it in-game.
pub fn mob_look_template(mob: &MobTemplate) -> RichText {
    let mut rt = RichText::new();

    rt.extend(parse_tags(&mob.description));
    rt.push_str("\n");

    rt
}

/// Full item examination as `look at <item>` would show it in-game.
pub fn item_look_template(item: &ItemTemplate) -> RichText {
    let mut rt = RichText::new();

    rt.extend(parse_tags(&item.description));
    rt.push_str("\n");

    rt
}
