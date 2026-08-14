use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_rooms(&self, table: &mut Table) {
        for area in self.registry.areas.values() {
            if let Some(room) = area.rooms.get(&self.template_id) {
                Self::add_field(table, "id", &room.id);
                Self::add_field(table, "name", &room.name);
                Self::add_field(table, "description", &room.description);
                Self::add_field(table, "area", &area.id);
                Self::add_field(table, "flags", room.flags.join(", "));
                Self::add_field(table, "allow_revive", room.allow_revive);

                // Exits (always display cardinal directions)
                let dirs = ["north", "south", "east", "west", "up", "down"];
                for dir in dirs {
                    let dest = room.exits.get(dir).map(|s| s.dest()).unwrap_or("");
                    Self::add_field(table, &format!("exit.{dir}"), dest);
                }
                // Custom exits
                for (dir, dest) in &room.exits {
                    if !dirs.contains(&dir.as_str()) {
                        Self::add_field(table, &format!("exit.{dir}"), dest.dest());
                    }
                }

                // Portals
                Self::add_array_header(table, "portal", room.portals.len());
                for (i, portal) in room.portals.iter().enumerate() {
                    Self::add_array_item(table, &format!("portal[{i}].keyword"), &portal.keyword);
                    Self::add_field(table, &format!("  portal[{i}].destination"), &portal.dest);
                    Self::add_field(
                        table,
                        &format!("  portal[{i}].description"),
                        &portal.description,
                    );
                    Self::add_field(
                        table,
                        &format!("  portal[{i}].flags"),
                        portal.flags.join(", "),
                    );
                }

                // Mobs Spawn
                Self::add_array_header(table, "content.mobs", room.content.mobs.len());
                for (i, spawn) in room.content.mobs.iter().enumerate() {
                    Self::add_array_item(
                        table,
                        &format!("content.mobs[{i}].template_id"),
                        &spawn.template_id,
                    );
                    Self::add_field(table, &format!("  content.mobs[{i}].count"), spawn.count);
                    let respawn_secs = spawn
                        .respawn_secs
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    Self::add_field(
                        table,
                        &format!("  content.mobs[{i}].respawn_secs"),
                        respawn_secs,
                    );
                }

                // Items Spawn
                Self::add_array_header(table, "content.items", room.content.items.len());
                for (i, spawn) in room.content.items.iter().enumerate() {
                    Self::add_array_item(
                        table,
                        &format!("content.items[{i}].template_id"),
                        &spawn.template_id,
                    );
                    Self::add_field(table, &format!("  content.items[{i}].count"), spawn.count);
                }

                return;
            }
        }
    }

    pub(super) fn update_rooms(&mut self, field: &str, value: &str) -> Result<(), String> {
        if field == "area" && !self.registry.areas.contains_key(value) {
            let valid: Vec<String> = self.registry.areas.keys().cloned().collect();
            return Err(format!(
                "area '{value}' does not exist. Valid areas: {}",
                valid.join(", ")
            ));
        }
        let room = self
            .registry
            .areas
            .values_mut()
            .find_map(|a| a.rooms.get_mut(&self.template_id))
            .ok_or_else(|| "room not found".to_string())?;
        match field {
            "id" => room.id = value.to_string(),
            "area" => {
                room.area = value.to_string();
            }
            "name" => room.name = value.to_string(),
            "description" => room.description = value.to_string(),
            "flags" => room.flags = value.split(',').map(|s| s.trim().to_string()).collect(),
            "allow_revive" => {
                room.allow_revive = value
                    .parse::<bool>()
                    .map_err(|_| "invalid boolean for allow_revive".to_string())?;
            }
            _ if field.starts_with("exit.") => {
                let dir = field.trim_start_matches("exit.").to_string();
                if value.is_empty() {
                    room.exits.remove(&dir);
                } else {
                    room.exits
                        .insert(dir, oxide_core::ExitTemplate::Simple(value.to_string()));
                }
            }
            _ if field.starts_with("portal[") => {
                let rest = field.trim_start_matches("portal[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid portal path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.portals.len() {
                    let p = &mut room.portals[idx];
                    if path_rest == ".keyword" {
                        p.keyword = value.to_string();
                    } else if path_rest == ".destination" {
                        p.dest = value.to_string();
                    } else if path_rest == ".description" {
                        p.description = value.to_string();
                    } else if path_rest == ".flags" {
                        p.flags = value.split(',').map(|s| s.trim().to_string()).collect();
                    }
                }
            }
            _ if field.starts_with("content.mobs[") => {
                let rest = field.trim_start_matches("content.mobs[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid mob spawn path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.content.mobs.len() {
                    let m = &mut room.content.mobs[idx];
                    if path_rest == ".template_id" {
                        m.template_id = value.to_string();
                    } else if path_rest == ".count" {
                        m.count = value.parse().map_err(|_| "invalid number")?;
                    } else if path_rest == ".respawn_secs" {
                        m.respawn_secs = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse().map_err(|_| "invalid number")?)
                        };
                    }
                }
            }
            _ if field.starts_with("content.items[") => {
                let rest = field.trim_start_matches("content.items[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid item spawn path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < room.content.items.len() {
                    let it = &mut room.content.items[idx];
                    if path_rest == ".template_id" {
                        it.template_id = value.to_string();
                    } else if path_rest == ".count" {
                        it.count = value.parse().map_err(|_| "invalid number")?;
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }

    pub(super) fn add_room_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let room = self
            .registry
            .areas
            .values_mut()
            .find_map(|a| a.rooms.get_mut(&self.template_id))
            .ok_or_else(|| "room not found".to_string())?;
        match prefix {
            "portal" => {
                room.portals.insert(
                    (index + 1).min(room.portals.len()),
                    oxide_core::templates::RoomPortalTemplate {
                        keyword: "door".to_string(),
                        dest: "room_id".to_string(),
                        description: "A wooden door.".to_string(),
                        flags: vec![],
                    },
                );
            }
            "content.mobs" => {
                room.content.mobs.insert(
                    (index + 1).min(room.content.mobs.len()),
                    oxide_core::templates::MobSpawnEntry {
                        template_id: "mob_id".to_string(),
                        count: 1,
                        respawn_secs: None,
                    },
                );
            }
            "content.items" => {
                room.content.items.insert(
                    (index + 1).min(room.content.items.len()),
                    oxide_core::templates::ItemSpawnEntry {
                        template_id: "item_id".to_string(),
                        count: 1,
                    },
                );
            }
            _ => return Err(format!("unknown room array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn remove_room_array(&mut self, prefix: &str, index: usize) -> Result<(), String> {
        let room = self
            .registry
            .areas
            .values_mut()
            .find_map(|a| a.rooms.get_mut(&self.template_id))
            .ok_or_else(|| "room not found".to_string())?;
        match prefix {
            "portal" => {
                if index < room.portals.len() {
                    room.portals.remove(index);
                }
            }
            "content.mobs" => {
                if index < room.content.mobs.len() {
                    room.content.mobs.remove(index);
                }
            }
            "content.items" => {
                if index < room.content.items.len() {
                    room.content.items.remove(index);
                }
            }
            _ => return Err(format!("unknown room array: {prefix}")),
        }
        Ok(())
    }

    pub(super) fn clear_room_array(&mut self, prefix: &str) -> Result<(), String> {
        let room = self
            .registry
            .areas
            .values_mut()
            .find_map(|a| a.rooms.get_mut(&self.template_id))
            .ok_or_else(|| "room not found".to_string())?;
        match prefix {
            "portal" => room.portals.clear(),
            "content.mobs" => room.content.mobs.clear(),
            "content.items" => room.content.items.clear(),
            _ => return Err(format!("unknown room array: {prefix}")),
        }
        Ok(())
    }
}
