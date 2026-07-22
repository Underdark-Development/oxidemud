use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use oxide_core::templates::{RoomTemplate, TemplateRegistry};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::components::CommandAction;
use crate::content::FileMap;
use crate::screens::{EntityContext, Screen, ScreenAction};

#[derive(Debug, Clone)]
struct GridCell {
    label: String,
    dir: String,
    dest: Option<(String, String)>,
    is_dig: bool,
}

pub struct RoomGridScreen {
    content_path: PathBuf,
    registry: TemplateRegistry,
    file_map: FileMap,
    active_room: Option<(String, String)>, // (area_id, room_id)
    selected_room: Option<(String, String)>,
    selected_cell: Option<(i32, i32)>, // (dx, dy) relative to center (0, 0)
    last_click: Option<((i32, i32), Instant)>,
    action: ScreenAction,
}

impl RoomGridScreen {
    pub fn new(content_path: PathBuf, registry: TemplateRegistry, file_map: FileMap) -> Self {
        let mut screen = RoomGridScreen {
            content_path,
            registry,
            file_map,
            active_room: None,
            selected_room: None,
            selected_cell: Some((0, 0)),
            last_click: None,
            action: ScreenAction::None,
        };
        screen.rebuild_graph();
        screen
    }

    pub fn rebuild_graph(&mut self) {
        if self.active_room.is_none() {
            self.active_room = self.find_first_room();
        } else if let Some((ref area_id, ref room_id)) = self.active_room {
            if !self.registry.areas.contains_key(area_id)
                || !self.registry.areas[area_id].rooms.contains_key(room_id)
            {
                self.active_room = self.find_first_room();
            }
        }

        // Reset selection to center (0, 0) when centering the map
        self.selected_cell = Some((0, 0));
        self.selected_room = self.active_room.clone();
    }

    fn find_first_room(&self) -> Option<(String, String)> {
        for (area_id, area) in &self.registry.areas {
            if let Some(room_id) = area.rooms.keys().next() {
                return Some((area_id.clone(), room_id.clone()));
            }
        }
        None
    }

    fn get_grid_cells(&self) -> HashMap<(i32, i32), GridCell> {
        let mut cells = HashMap::new();
        let (active_area_id, active_room_id) = match &self.active_room {
            Some(r) => r,
            None => return cells,
        };
        let area = match self.registry.areas.get(active_area_id) {
            Some(a) => a,
            None => return cells,
        };
        let room = match area.rooms.get(active_room_id) {
            Some(r) => r,
            None => return cells,
        };

        // Center cell
        cells.insert(
            (0, 0),
            GridCell {
                label: "Active".to_string(),
                dir: String::new(),
                dest: Some((active_area_id.clone(), active_room_id.clone())),
                is_dig: false,
            },
        );

        // Define adjacent directions + Up/Down offsets
        let dirs = &[
            ("north", "North", (0, -1)),
            ("south", "South", (0, 1)),
            ("east", "East", (1, 0)),
            ("west", "West", (-1, 0)),
            ("northeast", "NE", (1, -1)),
            ("northwest", "NW", (-1, -1)),
            ("southeast", "SE", (1, 1)),
            ("southwest", "SW", (-1, 1)),
            ("up", "Up", (-2, 0)),
            ("down", "Down", (2, 0)),
        ];

        let standard_dig_dirs = &["north", "south", "east", "west", "up", "down"];

        for &(dir_name, display_label, (dx, dy)) in dirs {
            let mut target_dest = None;
            for (ex_dir, dest_str) in &room.exits {
                let ex_lower = ex_dir.to_lowercase();
                if ex_lower == dir_name
                    || (dir_name == "north" && ex_lower == "n")
                    || (dir_name == "south" && ex_lower == "s")
                    || (dir_name == "east" && ex_lower == "e")
                    || (dir_name == "west" && ex_lower == "w")
                    || (dir_name == "up" && ex_lower == "u")
                    || (dir_name == "down" && ex_lower == "d")
                {
                    target_dest = Some(dest_str);
                    break;
                }
            }

            if let Some(dest_str) = target_dest {
                let dest_val = dest_str.dest();
                let (target_area, target_room) = if let Some((a, r)) = dest_val.split_once(':') {
                    (a.to_string(), r.to_string())
                } else {
                    (active_area_id.clone(), dest_val.to_string())
                };
                cells.insert(
                    (dx, dy),
                    GridCell {
                        label: display_label.to_string(),
                        dir: dir_name.to_string(),
                        dest: Some((target_area, target_room)),
                        is_dig: false,
                    },
                );
            } else if standard_dig_dirs.contains(&dir_name) {
                cells.insert(
                    (dx, dy),
                    GridCell {
                        label: display_label.to_string(),
                        dir: dir_name.to_string(),
                        dest: None,
                        is_dig: true,
                    },
                );
            }
        }

        // Portals (up to 4 mapped to outer corners)
        let portal_positions = &[(-2, -1), (2, -1), (-2, 1), (2, 1)];
        for (i, portal) in room.portals.iter().enumerate() {
            if i >= portal_positions.len() {
                break;
            }
            let (dx, dy) = portal_positions[i];
            let (target_area, target_room) = if let Some((a, r)) = portal.dest.split_once(':') {
                (a.to_string(), r.to_string())
            } else {
                (active_area_id.clone(), portal.dest.clone())
            };
            cells.insert(
                (dx, dy),
                GridCell {
                    label: format!("Portal:{}", portal.keyword),
                    dir: portal.keyword.clone(),
                    dest: Some((target_area, target_room)),
                    is_dig: false,
                },
            );
        }

        cells
    }

    fn move_selection(&mut self, ndx: i32, ndy: i32) {
        let (cx, cy) = self.selected_cell.unwrap_or((0, 0));
        let nx = (cx + ndx).clamp(-2, 2);
        let ny = (cy + ndy).clamp(-1, 1);
        self.selected_cell = Some((nx, ny));

        let cells = self.get_grid_cells();
        if let Some(cell) = cells.get(&(nx, ny)) {
            if let Some(ref dest) = cell.dest {
                self.selected_room = Some(dest.clone());
            } else {
                self.selected_room = None;
            }
        } else {
            self.selected_room = None;
        }
    }
}

impl Screen for RoomGridScreen {
    fn name(&self) -> &str {
        "Room Grid"
    }

    fn registry(&self) -> Option<&TemplateRegistry> {
        Some(&self.registry)
    }

    fn update_registry(&mut self, registry: &TemplateRegistry) {
        self.registry = registry.clone();
        self.rebuild_graph();
    }

    fn reload(&mut self) {
        let (registry, file_map) = crate::content::load_templates(&self.content_path);
        self.registry = registry;
        self.file_map = file_map;
        self.rebuild_graph();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(' ') => {
                if let Some(ref selected) = self.selected_room {
                    self.active_room = Some(selected.clone());
                    self.rebuild_graph();
                }
                true
            }
            KeyCode::Enter => {
                let cell_coord = self.selected_cell.unwrap_or((0, 0));
                let cells = self.get_grid_cells();
                if let Some(cell) = cells.get(&cell_coord) {
                    if cell.is_dig {
                        let _ =
                            self.handle_command_action(&CommandAction::DigRoom(cell.dir.clone()));
                    } else if let Some(ref selected) = self.selected_room {
                        self.action =
                            ScreenAction::Inspect("rooms".to_string(), selected.1.clone());
                    }
                }
                true
            }
            KeyCode::Up => {
                self.move_selection(0, -1);
                true
            }
            KeyCode::Down => {
                self.move_selection(0, 1);
                true
            }
            KeyCode::Right => {
                self.move_selection(1, 0);
                true
            }
            KeyCode::Left => {
                self.move_selection(-1, 0);
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        let instr = " [Arrows] Select cell  [Space] Center on room  [Enter / Double-Click] Edit room / Dig exit ";
        let instr_style = Style::default()
            .fg(Color::Indexed(245))
            .bg(Color::Indexed(236));
        set_str_safe(buf, area, area.x as i32, area.y as i32, instr, instr_style);

        for x in (area.x + instr.len() as u16)..area.x + area.width {
            set_char_safe(buf, area, x as i32, area.y as i32, ' ', instr_style);
        }

        let map_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        let center_x = map_area.x as i32 + map_area.width as i32 / 2;
        let center_y = map_area.y as i32 + map_area.height as i32 / 2;

        let cells = self.get_grid_cells();
        let selected_coord = self.selected_cell.unwrap_or((0, 0));

        // 1. Draw connection lines from center (0, 0)
        let line_style = Style::default().fg(Color::Indexed(240));
        let active_key = self.active_room.clone();

        if let Some((active_area, active_room)) = active_key {
            for (&(dx, dy), cell) in &cells {
                if (dx == 0 && dy == 0) || cell.is_dig {
                    continue;
                }

                // Find reverse exit back to active room
                let has_reverse = if let Some((ref target_area, ref target_room)) = cell.dest {
                    if let Some(target_room_tmpl) = self
                        .registry
                        .areas
                        .get(target_area)
                        .and_then(|a| a.rooms.get(target_room))
                    {
                        target_room_tmpl.exits.values().any(|dest_tpl| {
                            let dest_str = dest_tpl.dest();
                            let (ba, br) = if let Some((a, r)) = dest_str.split_once(':') {
                                (a, r)
                            } else {
                                (target_area.as_str(), dest_str)
                            };
                            ba == active_area && br == active_room
                        })
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Coordinates for cells
                let rx_a = center_x - 8; // Center cell x
                let ry_a = center_y - 2; // Center cell y

                let rx_b = center_x + dx * 20 - 8;
                let ry_b = center_y + dy * 7 - 2;

                if dx == 0 && dy == -1 {
                    // North
                    if has_reverse {
                        set_char_safe(buf, map_area, rx_a + 8, ry_a - 1, '│', line_style);
                        set_char_safe(buf, map_area, rx_a + 8, ry_a - 2, '│', line_style);
                    } else {
                        set_char_safe(buf, map_area, rx_a + 8, ry_a - 1, '^', line_style);
                        set_char_safe(buf, map_area, rx_a + 8, ry_a - 2, '│', line_style);
                    }
                } else if dx == 0 && dy == 1 {
                    // South
                    if has_reverse {
                        set_char_safe(buf, map_area, rx_a + 8, ry_a + 5, '│', line_style);
                        set_char_safe(buf, map_area, rx_a + 8, ry_a + 6, '│', line_style);
                    } else {
                        set_char_safe(buf, map_area, rx_a + 8, ry_a + 5, '│', line_style);
                        set_char_safe(buf, map_area, rx_a + 8, ry_a + 6, 'v', line_style);
                    }
                } else if dx == 1 && dy == 0 {
                    // East
                    let label = if has_reverse {
                        "────"
                    } else {
                        "───>"
                    };
                    set_str_safe(buf, map_area, rx_a + 16, ry_a + 2, label, line_style);
                } else if dx == -1 && dy == 0 {
                    // West
                    let label = if has_reverse {
                        "────"
                    } else {
                        "<───"
                    };
                    set_str_safe(buf, map_area, rx_b + 16, ry_b + 2, label, line_style);
                } else if dx == 1 && dy == -1 {
                    // Northeast
                    set_char_safe(buf, map_area, rx_a + 16, ry_a, '/', line_style);
                    set_char_safe(buf, map_area, rx_a + 17, ry_a - 1, '/', line_style);
                    set_char_safe(buf, map_area, rx_a + 18, ry_a - 2, '/', line_style);
                    set_char_safe(buf, map_area, rx_a + 19, ry_a - 3, '/', line_style);
                } else if dx == -1 && dy == -1 {
                    // Northwest
                    set_char_safe(buf, map_area, rx_a - 1, ry_a, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a - 2, ry_a - 1, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a - 3, ry_a - 2, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a - 4, ry_a - 3, '\\', line_style);
                } else if dx == 1 && dy == 1 {
                    // Southeast
                    set_char_safe(buf, map_area, rx_a + 16, ry_a + 4, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a + 17, ry_a + 5, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a + 18, ry_a + 6, '\\', line_style);
                    set_char_safe(buf, map_area, rx_a + 19, ry_a + 7, '\\', line_style);
                } else if dx == -1 && dy == 1 {
                    // Southwest
                    set_char_safe(buf, map_area, rx_a - 1, ry_a + 4, '/', line_style);
                    set_char_safe(buf, map_area, rx_a - 2, ry_a + 5, '/', line_style);
                    set_char_safe(buf, map_area, rx_a - 3, ry_a + 6, '/', line_style);
                    set_char_safe(buf, map_area, rx_a - 4, ry_a + 7, '/', line_style);
                }
            }
        }

        // 2. Draw cell boxes
        for (&(dx, dy), cell) in &cells {
            let rx = center_x + dx * 20 - 8;
            let ry = center_y + dy * 7 - 2;

            let is_selected = selected_coord == (dx, dy);

            if cell.is_dig {
                draw_dig_box(buf, map_area, rx, ry, &cell.label, is_selected);
            } else if let Some(ref dest) = cell.dest {
                let is_active = dx == 0 && dy == 0;
                let room_name = self
                    .registry
                    .areas
                    .get(&dest.0)
                    .and_then(|a| a.rooms.get(&dest.1))
                    .map(|r| r.name.as_str());
                draw_box(
                    buf,
                    map_area,
                    (rx, ry),
                    &dest.1,
                    &cell.label,
                    room_name,
                    (is_active, is_selected),
                );
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            let col = mouse.column as i32;
            let row = mouse.row as i32;

            let map_area = Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            );
            let center_x = map_area.x as i32 + map_area.width as i32 / 2;
            let center_y = map_area.y as i32 + map_area.height as i32 / 2;

            let mut clicked_coord = None;
            for dx in -2..=2 {
                for dy in -1..=1 {
                    let rx = center_x + dx * 20 - 8;
                    let ry = center_y + dy * 7 - 2;
                    if col >= rx && col < rx + 16 && row >= ry && row < ry + 5 {
                        clicked_coord = Some((dx, dy));
                        break;
                    }
                }
            }

            if let Some((dx, dy)) = clicked_coord {
                let now = Instant::now();
                let is_double_click = if let Some((ref last_pos, last_time)) = self.last_click {
                    last_pos == &(dx, dy)
                        && now.duration_since(last_time) < std::time::Duration::from_millis(500)
                } else {
                    false
                };

                self.selected_cell = Some((dx, dy));
                self.last_click = Some(((dx, dy), now));

                let cells = self.get_grid_cells();
                if let Some(cell) = cells.get(&(dx, dy)) {
                    if let Some(ref dest) = cell.dest {
                        self.selected_room = Some(dest.clone());
                    } else {
                        self.selected_room = None;
                    }

                    if is_double_click {
                        if cell.is_dig {
                            let _ = self
                                .handle_command_action(&CommandAction::DigRoom(cell.dir.clone()));
                        } else if let Some(ref dest) = cell.dest {
                            self.action =
                                ScreenAction::Inspect("rooms".to_string(), dest.1.clone());
                        }
                    }
                }
            }
        }
    }

    fn selection_context(&self) -> Option<EntityContext> {
        let (area_id, room_id) = self.selected_room.as_ref()?;
        let area = self.registry.areas.get(area_id)?;
        let room = area.rooms.get(room_id)?;
        Some(EntityContext {
            category: "rooms".to_string(),
            id: room_id.clone(),
            name: room.name.clone(),
        })
    }

    fn contextual_commands(&self) -> Vec<(String, CommandAction)> {
        // If we have selected_room, use it. Otherwise fall back to active_room.
        let (area_id, room_id) = match &self.selected_room {
            Some(s) => s.clone(),
            None => match &self.active_room {
                Some(a) => a.clone(),
                None => return Vec::new(),
            },
        };

        let area = match self.registry.areas.get(&area_id) {
            Some(a) => a,
            None => return Vec::new(),
        };
        let room = match area.rooms.get(&room_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut cmds = Vec::new();

        // 1. Movement options
        let mut sorted_exits: Vec<(&String, &oxide_core::ExitTemplate)> =
            room.exits.iter().collect();
        sorted_exits.sort_by_key(|(dir, _)| dir.to_lowercase());
        for (dir, dest) in sorted_exits {
            let dest_str = dest.dest().to_string();
            let label = format!("Go {} (to {})", dir, dest_str);
            cmds.push((label, CommandAction::MoveToRoom(dest_str)));
        }

        // Portals
        for portal in &room.portals {
            let label = format!("Portal '{}' (to {})", portal.keyword, portal.dest);
            cmds.push((label, CommandAction::MoveToRoom(portal.dest.clone())));
        }

        // 2. Digging options
        let standard_dirs = &["north", "south", "east", "west", "up", "down"];
        for dir in standard_dirs {
            let exists = room.exits.keys().any(|k| {
                let k_lower = k.to_lowercase();
                k_lower == *dir || (k_lower.len() == 1 && dir.starts_with(&k_lower))
            });
            if !exists {
                cmds.push((
                    format!("Dig {}", dir),
                    CommandAction::DigRoom(dir.to_string()),
                ));
            }
        }

        cmds
    }

    fn handle_command_action(&mut self, action: &CommandAction) -> Result<bool, String> {
        match action {
            CommandAction::MoveToRoom(dest) => {
                let current = self
                    .selected_room
                    .as_ref()
                    .or(self.active_room.as_ref())
                    .ok_or("No room selected")?;
                let (target_area, target_room) = if let Some((a, r)) = dest.split_once(':') {
                    (a.to_string(), r.to_string())
                } else {
                    (current.0.clone(), dest.clone())
                };

                self.active_room = Some((target_area.clone(), target_room.clone()));
                self.selected_room = Some((target_area, target_room));
                self.selected_cell = Some((0, 0));
                self.rebuild_graph();
                Ok(true)
            }
            CommandAction::DigRoom(dir) => {
                let (area_id, parent_id) = self
                    .selected_room
                    .as_ref()
                    .or(self.active_room.as_ref())
                    .ok_or("No room selected")?
                    .clone();

                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let new_room_id = format!("room_{ts}");

                let cp = &self.content_path;
                let parent_path = crate::content::room_path(&self.file_map, &area_id, &parent_id)
                    .unwrap_or_else(|| {
                        cp.join("areas")
                            .join(&area_id)
                            .join("rooms")
                            .join(format!("{parent_id}.toml"))
                    });

                let new_room_path = cp
                    .join("areas")
                    .join(&area_id)
                    .join("rooms")
                    .join(format!("{new_room_id}.toml"));

                // Update parent room exits in registry and save to disk
                {
                    let area = self
                        .registry
                        .areas
                        .get_mut(&area_id)
                        .ok_or_else(|| "Area not found in registry".to_string())?;
                    let parent_room = area
                        .rooms
                        .get_mut(&parent_id)
                        .ok_or_else(|| "Parent room not found in area".to_string())?;
                    parent_room.exits.insert(
                        dir.clone(),
                        oxide_core::ExitTemplate::Simple(new_room_id.clone()),
                    );

                    let parent_toml = toml::to_string_pretty(parent_room)
                        .map_err(|e| format!("failed to serialize parent room: {e}"))?;
                    if let Some(parent_dir) = parent_path.parent() {
                        fs::create_dir_all(parent_dir)
                            .map_err(|e| format!("cannot create parent dir: {e}"))?;
                    }
                    fs::write(&parent_path, parent_toml)
                        .map_err(|e| format!("failed to write parent room: {e}"))?;
                }

                // Create new room in registry and save to disk
                {
                    let rev = reverse_dir(dir);
                    let mut exits = HashMap::new();
                    exits.insert(rev, oxide_core::ExitTemplate::Simple(parent_id.clone()));

                    let new_room = RoomTemplate {
                        id: new_room_id.clone(),
                        area: area_id.clone(),
                        name: "A newly dug room".to_string(),
                        description: "You see a newly dug room here.".to_string(),
                        exits,
                        portals: Vec::new(),
                        flags: Vec::new(),
                        content: Default::default(),
                        allow_revive: false,
                        script: None,
                        params: HashMap::new(),
                    };

                    let new_toml = toml::to_string_pretty(&new_room)
                        .map_err(|e| format!("failed to serialize new room: {e}"))?;
                    if let Some(new_dir) = new_room_path.parent() {
                        fs::create_dir_all(new_dir)
                            .map_err(|e| format!("cannot create new room dir: {e}"))?;
                    }
                    fs::write(&new_room_path, new_toml)
                        .map_err(|e| format!("failed to write new room: {e}"))?;

                    // Insert into registry
                    let area = self
                        .registry
                        .areas
                        .get_mut(&area_id)
                        .ok_or_else(|| format!("area '{area_id}' not found"))?;
                    area.rooms.insert(new_room_id.clone(), new_room);
                }

                self.selected_room = Some((area_id.clone(), new_room_id.clone()));
                self.active_room = Some((area_id, new_room_id));
                self.selected_cell = Some((0, 0));

                self.reload();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn take_action(&mut self) -> ScreenAction {
        std::mem::replace(&mut self.action, ScreenAction::None)
    }
}

fn reverse_dir(dir: &str) -> String {
    match dir.to_lowercase().as_str() {
        "north" | "n" => "south".to_string(),
        "south" | "s" => "north".to_string(),
        "east" | "e" => "west".to_string(),
        "west" | "w" => "east".to_string(),
        "northeast" | "ne" => "southwest".to_string(),
        "northwest" | "nw" => "southeast".to_string(),
        "southeast" | "se" => "northwest".to_string(),
        "southwest" | "sw" => "northeast".to_string(),
        "up" | "u" => "down".to_string(),
        "down" | "d" => "up".to_string(),
        other => other.to_string(),
    }
}

fn set_char_safe(buf: &mut Buffer, area: Rect, x: i32, y: i32, ch: char, style: Style) {
    if x >= area.x as i32
        && x < (area.x + area.width) as i32
        && y >= area.y as i32
        && y < (area.y + area.height) as i32
    {
        if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}

fn set_str_safe(buf: &mut Buffer, area: Rect, x: i32, y: i32, s: &str, style: Style) {
    for (i, ch) in s.chars().enumerate() {
        set_char_safe(buf, area, x + i as i32, y, ch, style);
    }
}

fn draw_box(
    buf: &mut Buffer,
    area: Rect,
    pos: (i32, i32),
    room_id: &str,
    label: &str,
    room_name: Option<&str>,
    flags: (bool, bool), // (is_active, is_selected)
) {
    let (x, y) = pos;
    let (is_active, is_selected) = flags;
    let border_style = if is_active {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(245))
    };

    let text_style = if is_active || is_selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(250))
    };

    set_str_safe(buf, area, x, y, "┌──────────────┐", border_style);
    set_str_safe(buf, area, x, y + 1, "│              │", border_style);
    set_str_safe(buf, area, x, y + 2, "├──────────────┤", border_style);
    set_str_safe(buf, area, x, y + 3, "│              │", border_style);
    set_str_safe(buf, area, x, y + 4, "└──────────────┘", border_style);

    let padded_label = format!(" {} ", label);
    if padded_label.len() <= 14 {
        let pad = (14 - padded_label.len()) / 2;
        set_str_safe(
            buf,
            area,
            x + 1 + pad as i32,
            y,
            &padded_label,
            border_style.add_modifier(Modifier::BOLD),
        );
    }

    let display_id = if room_id.len() > 14 {
        &room_id[..14]
    } else {
        room_id
    };
    let pad = (14 - display_id.len()) / 2;
    set_str_safe(buf, area, x + 1 + pad as i32, y + 1, display_id, text_style);

    if let Some(name) = room_name {
        let display_name = if name.len() > 14 { &name[..14] } else { name };
        let pad_name = (14 - display_name.len()) / 2;
        set_str_safe(
            buf,
            area,
            x + 1 + pad_name as i32,
            y + 3,
            display_name,
            text_style,
        );
    }
}

fn draw_dig_box(buf: &mut Buffer, area: Rect, x: i32, y: i32, dir_label: &str, is_selected: bool) {
    let border_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(239))
    };

    let text_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(242))
    };

    set_str_safe(buf, area, x, y, "┌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐", border_style);
    set_str_safe(buf, area, x, y + 1, "╎              ╎", border_style);
    set_str_safe(buf, area, x, y + 2, "╎              ╎", border_style);
    set_str_safe(buf, area, x, y + 3, "╎              ╎", border_style);
    set_str_safe(buf, area, x, y + 4, "└╌╌╌╌╌╌╌╌╌╌╌╌╌╌┘", border_style);

    let display_str = format!("+ Dig {}", dir_label);
    if display_str.len() <= 14 {
        let pad = (14 - display_str.len()) / 2;
        set_str_safe(
            buf,
            area,
            x + 1 + pad as i32,
            y + 2,
            &display_str,
            text_style,
        );
    } else {
        // Fallback truncation
        let truncated = &display_str[..14];
        set_str_safe(buf, area, x + 1, y + 2, truncated, text_style);
    }
}
