use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use mud_core::templates::{RoomTemplate, TemplateRegistry};
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
pub struct GraphRoom {
    pub area_id: String,
    pub room_id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub exits: HashMap<String, String>,
}

pub struct RoomGraphScreen {
    content_path: PathBuf,
    registry: TemplateRegistry,
    file_map: FileMap,
    active_room: Option<(String, String)>, // (area_id, room_id)
    selected_room: Option<(String, String)>,
    graph: HashMap<(String, String), GraphRoom>,
    scroll_x: i16,
    scroll_y: i16,
    last_click: Option<((String, String), Instant)>,
    action: ScreenAction,
}

impl RoomGraphScreen {
    pub fn new(content_path: PathBuf, registry: TemplateRegistry, file_map: FileMap) -> Self {
        let mut screen = RoomGraphScreen {
            content_path,
            registry,
            file_map,
            active_room: None,
            selected_room: None,
            graph: HashMap::new(),
            scroll_x: 0,
            scroll_y: 0,
            last_click: None,
            action: ScreenAction::None,
        };
        screen.rebuild_graph();
        screen
    }

    pub fn rebuild_graph(&mut self) {
        // If active_room is None or not in registry, default to first room
        if self.active_room.is_none() {
            self.active_room = self.find_first_room();
        } else if let Some((ref area_id, ref room_id)) = self.active_room {
            if !self.registry.areas.contains_key(area_id)
                || !self.registry.areas[area_id].rooms.contains_key(room_id)
            {
                self.active_room = self.find_first_room();
            }
        }

        if let Some((ref area_id, ref room_id)) = self.active_room {
            self.graph = build_graph(&self.registry, area_id, room_id, 4);
        } else {
            self.graph.clear();
        }

        // Keep selected room in sync
        if self.selected_room.is_none() {
            self.selected_room = self.active_room.clone();
        } else if let Some((ref area_id, ref room_id)) = self.selected_room {
            if !self.registry.areas.contains_key(area_id)
                || !self.registry.areas[area_id].rooms.contains_key(room_id)
            {
                self.selected_room = self.active_room.clone();
            }
        }
    }

    fn find_first_room(&self) -> Option<(String, String)> {
        for (area_id, area) in &self.registry.areas {
            if let Some(room_id) = area.rooms.keys().next() {
                return Some((area_id.clone(), room_id.clone()));
            }
        }
        None
    }

    fn navigate_selection(&mut self, dir: &str) {
        let current = match &self.selected_room {
            Some(s) => s,
            None => return,
        };
        let room = match self.graph.get(current) {
            Some(r) => r,
            None => return,
        };

        // Find matching exit
        let mut target_dest = None;
        for (exit_dir, dest) in &room.exits {
            let ed_lower = exit_dir.to_lowercase();
            if ed_lower == dir
                || (dir == "north" && ed_lower == "n")
                || (dir == "south" && ed_lower == "s")
                || (dir == "east" && ed_lower == "e")
                || (dir == "west" && ed_lower == "w")
            {
                target_dest = Some(dest);
                break;
            }
        }

        if let Some(dest) = target_dest {
            let (target_area, target_room) = if let Some((a, r)) = dest.split_once(':') {
                (a.to_string(), r.to_string())
            } else {
                (current.0.clone(), dest.clone())
            };
            self.selected_room = Some((target_area, target_room));
        }
    }
}

impl Screen for RoomGraphScreen {
    fn name(&self) -> &str {
        "Room Graph"
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
            KeyCode::Char('w') => {
                self.scroll_y = self.scroll_y.saturating_add(2);
                true
            }
            KeyCode::Char('s') => {
                self.scroll_y = self.scroll_y.saturating_sub(2);
                true
            }
            KeyCode::Char('a') => {
                self.scroll_x = self.scroll_x.saturating_add(4);
                true
            }
            KeyCode::Char('d') => {
                self.scroll_x = self.scroll_x.saturating_sub(4);
                true
            }
            KeyCode::Char(' ') => {
                if let Some(ref selected) = self.selected_room {
                    self.active_room = Some(selected.clone());
                    self.scroll_x = 0;
                    self.scroll_y = 0;
                    self.rebuild_graph();
                }
                true
            }
            KeyCode::Enter => {
                if let Some(ref selected) = self.selected_room {
                    self.action = ScreenAction::Inspect("rooms".to_string(), selected.1.clone());
                }
                true
            }
            KeyCode::Up => {
                self.navigate_selection("north");
                true
            }
            KeyCode::Down => {
                self.navigate_selection("south");
                true
            }
            KeyCode::Right => {
                self.navigate_selection("east");
                true
            }
            KeyCode::Left => {
                self.navigate_selection("west");
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, _mouse_pos: Option<(u16, u16)>) {
        // Render a header instruction bar
        let instr = " [w/a/s/d] Pan map  [Arrows] Select room  [Space] Center map  [Enter] Edit in Entities Editor ";
        let instr_style = Style::default()
            .fg(Color::Indexed(245))
            .bg(Color::Indexed(236));
        set_str_safe(buf, area, area.x as i32, area.y as i32, instr, instr_style);

        // Fill instruction bar background
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

        let active_key = self.active_room.clone();
        let selected_key = self.selected_room.clone();

        // 1. Draw connection lines first so they sit underneath boxes
        let line_style = Style::default().fg(Color::Indexed(240));
        for ((ax, _ay), room_a) in &self.graph {
            let rx_a = center_x + room_a.x * 16 - 6 + self.scroll_x as i32;
            let ry_a = center_y + room_a.y * 5 - 1 + self.scroll_y as i32;

            for dest in room_a.exits.values() {
                let (target_area, target_room) = if let Some((a, r)) = dest.split_once(':') {
                    (a.to_string(), r.to_string())
                } else {
                    (ax.clone(), dest.clone())
                };

                if let Some(room_b) = self.graph.get(&(target_area, target_room)) {
                    let rx_b = center_x + room_b.x * 16 - 6 + self.scroll_x as i32;
                    let ry_b = center_y + room_b.y * 5 - 1 + self.scroll_y as i32;

                    // East exit
                    if room_b.x == room_a.x + 1 && room_b.y == room_a.y {
                        let has_reverse = room_b.exits.iter().any(|(bd, bdest)| {
                            let bd_lower = bd.to_lowercase();
                            let (ba, br) = if let Some((a, r)) = bdest.split_once(':') {
                                (a, r)
                            } else {
                                (room_b.area_id.as_str(), bdest.as_str())
                            };
                            (bd_lower == "west" || bd_lower == "w")
                                && ba == ax
                                && br == room_a.room_id
                        });
                        let label = if has_reverse {
                            "────"
                        } else {
                            "───>"
                        };
                        set_str_safe(buf, map_area, rx_a + 12, ry_a + 1, label, line_style);
                    }
                    // West exit (only draw if A -> B one-way; if B -> A, drawn via B's East check)
                    else if room_b.x == room_a.x - 1 && room_b.y == room_a.y {
                        let has_reverse = room_b.exits.iter().any(|(bd, bdest)| {
                            let bd_lower = bd.to_lowercase();
                            let (ba, br) = if let Some((a, r)) = bdest.split_once(':') {
                                (a, r)
                            } else {
                                (room_b.area_id.as_str(), bdest.as_str())
                            };
                            (bd_lower == "east" || bd_lower == "e")
                                && ba == ax
                                && br == room_a.room_id
                        });
                        if !has_reverse {
                            set_str_safe(buf, map_area, rx_b + 12, ry_b + 1, "<───", line_style);
                        }
                    }
                    // South exit
                    else if room_b.x == room_a.x && room_b.y == room_a.y + 1 {
                        let has_reverse = room_b.exits.iter().any(|(bd, bdest)| {
                            let bd_lower = bd.to_lowercase();
                            let (ba, br) = if let Some((a, r)) = bdest.split_once(':') {
                                (a, r)
                            } else {
                                (room_b.area_id.as_str(), bdest.as_str())
                            };
                            (bd_lower == "north" || bd_lower == "n")
                                && ba == ax
                                && br == room_a.room_id
                        });
                        if has_reverse {
                            set_char_safe(buf, map_area, rx_a + 6, ry_a + 3, '│', line_style);
                            set_char_safe(buf, map_area, rx_a + 6, ry_a + 4, '│', line_style);
                        } else {
                            set_char_safe(buf, map_area, rx_a + 6, ry_a + 3, '│', line_style);
                            set_char_safe(buf, map_area, rx_a + 6, ry_a + 4, 'v', line_style);
                        }
                    }
                    // North exit
                    else if room_b.x == room_a.x && room_b.y == room_a.y - 1 {
                        let has_reverse = room_b.exits.iter().any(|(bd, bdest)| {
                            let bd_lower = bd.to_lowercase();
                            let (ba, br) = if let Some((a, r)) = bdest.split_once(':') {
                                (a, r)
                            } else {
                                (room_b.area_id.as_str(), bdest.as_str())
                            };
                            (bd_lower == "south" || bd_lower == "s")
                                && ba == ax
                                && br == room_a.room_id
                        });
                        if !has_reverse {
                            set_char_safe(buf, map_area, rx_a + 6, ry_a - 1, '^', line_style);
                            set_char_safe(buf, map_area, rx_a + 6, ry_a - 2, '│', line_style);
                        }
                    }
                    // Diagonal NE
                    else if room_b.x == room_a.x + 1 && room_b.y == room_a.y - 1 {
                        set_char_safe(buf, map_area, rx_a + 12, ry_a, '/', line_style);
                        set_char_safe(buf, map_area, rx_a + 13, ry_a - 1, '/', line_style);
                    }
                    // Diagonal NW
                    else if room_b.x == room_a.x - 1 && room_b.y == room_a.y - 1 {
                        set_char_safe(buf, map_area, rx_a - 1, ry_a, '\\', line_style);
                        set_char_safe(buf, map_area, rx_a - 2, ry_a - 1, '\\', line_style);
                    }
                    // Diagonal SE
                    else if room_b.x == room_a.x + 1 && room_b.y == room_a.y + 1 {
                        set_char_safe(buf, map_area, rx_a + 12, ry_a + 2, '\\', line_style);
                        set_char_safe(buf, map_area, rx_a + 13, ry_a + 3, '\\', line_style);
                    }
                    // Diagonal SW
                    else if room_b.x == room_a.x - 1 && room_b.y == room_a.y + 1 {
                        set_char_safe(buf, map_area, rx_a - 1, ry_a + 2, '/', line_style);
                        set_char_safe(buf, map_area, rx_a - 2, ry_a + 3, '/', line_style);
                    }
                }
            }
        }

        // 2. Draw room boxes
        for (key, room) in &self.graph {
            let rx = center_x + room.x * 16 - 6 + self.scroll_x as i32;
            let ry = center_y + room.y * 5 - 1 + self.scroll_y as i32;

            let is_active = Some(key) == active_key.as_ref();
            let is_selected = Some(key) == selected_key.as_ref();

            draw_box(buf, map_area, rx, ry, &room.room_id, is_active, is_selected);
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

            let mut clicked_room = None;
            for (key, room) in &self.graph {
                let rx = center_x + room.x * 16 - 6 + self.scroll_x as i32;
                let ry = center_y + room.y * 5 - 1 + self.scroll_y as i32;

                if col >= rx && col < rx + 12 && row >= ry && row < ry + 3 {
                    clicked_room = Some(key.clone());
                    break;
                }
            }

            if let Some(room_key) = clicked_room {
                let now = Instant::now();
                let is_double_click = if let Some((ref last_key, last_time)) = self.last_click {
                    last_key == &room_key
                        && now.duration_since(last_time) < std::time::Duration::from_millis(500)
                } else {
                    false
                };

                self.selected_room = Some(room_key.clone());
                self.last_click = Some((room_key, now));

                if is_double_click {
                    self.action = ScreenAction::Inspect(
                        "rooms".to_string(),
                        self.selected_room.as_ref().unwrap().1.clone(),
                    );
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
        let (area_id, room_id) = match &self.selected_room {
            Some(s) => s,
            None => return Vec::new(),
        };
        let area = match self.registry.areas.get(area_id) {
            Some(a) => a,
            None => return Vec::new(),
        };
        let room = match area.rooms.get(room_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut cmds = Vec::new();

        // 1. Movement options
        let mut sorted_exits: Vec<(&String, &String)> = room.exits.iter().collect();
        sorted_exits.sort_by_key(|(dir, _)| dir.to_lowercase());
        for (dir, dest) in sorted_exits {
            let label = format!("Go {} (to {})", dir, dest);
            cmds.push((label, CommandAction::MoveToRoom(dest.clone())));
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
                let current = self.selected_room.as_ref().ok_or("No room selected")?;
                let (target_area, target_room) = if let Some((a, r)) = dest.split_once(':') {
                    (a.to_string(), r.to_string())
                } else {
                    (current.0.clone(), dest.clone())
                };

                self.active_room = Some((target_area.clone(), target_room.clone()));
                self.selected_room = Some((target_area, target_room));
                self.scroll_x = 0;
                self.scroll_y = 0;
                self.rebuild_graph();
                Ok(true)
            }
            CommandAction::DigRoom(dir) => {
                let (area_id, parent_id) = self
                    .selected_room
                    .as_ref()
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
                    parent_room.exits.insert(dir.clone(), new_room_id.clone());

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
                    exits.insert(rev, parent_id.clone());

                    let new_room = RoomTemplate {
                        id: new_room_id.clone(),
                        area: area_id.clone(),
                        name: "A newly dug room".to_string(),
                        description: "You see a newly dug room here.".to_string(),
                        exits,
                        portals: Vec::new(),
                        flags: Vec::new(),
                        content: Default::default(),
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
                    let area = self.registry.areas.get_mut(&area_id).unwrap();
                    area.rooms.insert(new_room_id.clone(), new_room);
                }

                self.selected_room = Some((area_id.clone(), new_room_id.clone()));
                self.active_room = Some((area_id, new_room_id));
                self.scroll_x = 0;
                self.scroll_y = 0;

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

fn build_graph(
    registry: &TemplateRegistry,
    start_area: &str,
    start_room: &str,
    max_depth: usize,
) -> HashMap<(String, String), GraphRoom> {
    let mut graph = HashMap::new();
    let mut visited = HashSet::new();
    let mut occupied = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((start_area.to_string(), start_room.to_string(), 0, 0, 0));
    visited.insert((start_area.to_string(), start_room.to_string()));
    occupied.insert((0, 0));

    while let Some((area_id, room_id, x, y, depth)) = queue.pop_front() {
        let area = match registry.areas.get(&area_id) {
            Some(a) => a,
            None => continue,
        };
        let room = match area.rooms.get(&room_id) {
            Some(r) => r,
            None => continue,
        };

        graph.insert(
            (area_id.clone(), room_id.clone()),
            GraphRoom {
                area_id: area_id.clone(),
                room_id: room_id.clone(),
                name: room.name.clone(),
                x,
                y,
                exits: room.exits.clone(),
            },
        );

        if depth >= max_depth {
            continue;
        }

        for (dir, dest) in &room.exits {
            let (target_area, target_room) = if let Some((a, r)) = dest.split_once(':') {
                (a.to_string(), r.to_string())
            } else {
                (area_id.clone(), dest.clone())
            };

            let target_key = (target_area.clone(), target_room.clone());
            if visited.contains(&target_key) {
                continue;
            }

            let (dx, dy) = match dir.to_lowercase().as_str() {
                "north" | "n" => (0, -1),
                "south" | "s" => (0, 1),
                "east" | "e" => (1, 0),
                "west" | "w" => (-1, 0),
                "northeast" | "ne" => (1, -1),
                "northwest" | "nw" => (-1, -1),
                "southeast" | "se" => (1, 1),
                "southwest" | "sw" => (-1, 1),
                _ => (0, 0),
            };

            if dx == 0 && dy == 0 {
                visited.insert(target_key);
                continue;
            }

            let nx = x + dx;
            let ny = y + dy;

            if !occupied.contains(&(nx, ny)) {
                visited.insert(target_key.clone());
                occupied.insert((nx, ny));
                queue.push_back((target_area, target_room, nx, ny, depth + 1));
            }
        }
    }

    graph
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
    x: i32,
    y: i32,
    room_id: &str,
    is_active: bool,
    is_selected: bool,
) {
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

    set_str_safe(buf, area, x, y, "┌──────────┐", border_style);
    set_str_safe(buf, area, x, y + 1, "│          │", border_style);
    set_str_safe(buf, area, x, y + 2, "└──────────┘", border_style);

    let display_id = if room_id.len() > 10 {
        &room_id[..10]
    } else {
        room_id
    };
    let pad = (10 - display_id.len()) / 2;
    set_str_safe(buf, area, x + 1 + pad as i32, y + 1, display_id, text_style);
}
