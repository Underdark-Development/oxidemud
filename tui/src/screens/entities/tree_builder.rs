use std::collections::{HashMap, HashSet};

use crate::components::{Tree, TreeNode};
use crate::screens::entities::{EntitiesScreen, NodeInfo};

impl EntitiesScreen {
    pub(super) fn rebuild_tree(&mut self) {
        let filter = self
            .search
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());

        let matches = |name: &str| -> bool {
            filter
                .as_ref()
                .is_none_or(|f| name.to_lowercase().contains(f.as_str()))
        };

        let mut roots = Vec::new();

        if !self.registry.areas.is_empty() {
            let mut area_count = 0usize;
            let mut area_node = TreeNode::new(
                "🧭 Areas".to_string(),
                NodeInfo {
                    category: String::new(),
                    id: String::new(),
                },
            );
            let mut ids: Vec<&String> = self.registry.areas.keys().collect();
            ids.sort();
            for id in ids {
                let area = &self.registry.areas[id];
                let area_matches = matches(&area.name) || filter.is_none();
                let mut area_child = TreeNode::new(
                    area.name.clone(),
                    NodeInfo {
                        category: "areas_folder".into(),
                        id: id.clone(),
                    },
                );
                if area_matches {
                    let area_dirty = self.unsaved.contains(&("areas".into(), id.clone()));
                    area_child.add_child(TreeNode {
                        label: "area.toml".to_string(),
                        data: NodeInfo {
                            category: "areas".into(),
                            id: id.clone(),
                        },
                        children: Vec::new(),
                        collapsed: false,
                        dirty: area_dirty,
                    });
                }
                let mut room_count = 0usize;
                let mut room_ids: Vec<&String> = area.rooms.keys().collect();
                room_ids.sort();
                for room_id in room_ids {
                    let room = &area.rooms[room_id];
                    if matches(&room.name) {
                        let room_dirty = self.unsaved.contains(&("rooms".into(), room_id.clone()));
                        area_child.add_child(TreeNode {
                            label: room.name.clone(),
                            data: NodeInfo {
                                category: "rooms".into(),
                                id: room_id.clone(),
                            },
                            children: Vec::new(),
                            collapsed: false,
                            dirty: room_dirty,
                        });
                        room_count += 1;
                    }
                }
                if area_matches || room_count > 0 {
                    area_node.add_child(area_child);
                    area_count += 1;
                }
            }
            if area_count > 0 {
                area_node.label = format!("🧭 Areas ({area_count})");
                roots.push(area_node);
            }
        }

        let filter_str = filter.as_deref();
        add_group(
            &mut roots,
            &self.registry.items,
            "📦 Items",
            "items",
            |i| i.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.mobs,
            "🧟 Mobs",
            "mobs",
            |m| m.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.quests,
            "📜 Quests",
            "quests",
            |q| q.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.factions,
            "🛡️ Factions",
            "factions",
            |f| f.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.recipes,
            "🛠️ Recipes",
            "recipes",
            |r| r.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.races,
            "🧬 Races",
            "races",
            |r| r.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.classes,
            "⚔️ Classes",
            "classes",
            |c| c.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.skills,
            "✨ Skills",
            "skills",
            |s| s.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.stances,
            "🥋 Stances",
            "stances",
            |s| s.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.sets,
            "👑 Sets",
            "sets",
            |s| s.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.affixes,
            "💎 Affixes",
            "affixes",
            |a| a.name.clone(),
            filter_str,
            &self.unsaved,
        );
        add_group(
            &mut roots,
            &self.registry.passives,
            "🔮 Passives",
            "passives",
            |p| p.name.clone(),
            filter_str,
            &self.unsaved,
        );

        self.tree = Tree::new(roots);
        self.tree.search_filter = self.search.clone();
        self.tree.selected = Some(0);
    }
}

fn add_group<T, F>(
    roots: &mut Vec<TreeNode<NodeInfo>>,
    items: &HashMap<String, T>,
    label: &str,
    category: &str,
    display: F,
    filter: Option<&str>,
    unsaved: &HashSet<(String, String)>,
) where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return;
    }
    let mut count = 0usize;
    let mut node = TreeNode::new(
        label.to_string(),
        NodeInfo {
            category: String::new(),
            id: String::new(),
        },
    );
    let mut ids: Vec<&String> = items.keys().collect();
    ids.sort();
    for id in ids {
        if let Some(item) = items.get(id) {
            let name = display(item);
            let is_dirty = unsaved.contains(&(category.to_string(), id.clone()));
            if filter.is_none_or(|f| name.to_lowercase().contains(&f.to_lowercase())) {
                node.add_child(TreeNode {
                    label: name,
                    data: NodeInfo {
                        category: category.to_string(),
                        id: id.clone(),
                    },
                    children: Vec::new(),
                    collapsed: false,
                    dirty: is_dirty,
                });
                count += 1;
            }
        }
    }
    if count > 0 {
        node.label = format!("{label} ({count})");
        roots.push(node);
    }
}
