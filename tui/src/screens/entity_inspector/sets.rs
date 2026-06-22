use crate::components::Table;
use crate::screens::entity_inspector::EntityInspectorScreen;

impl EntityInspectorScreen {
    pub(super) fn load_sets(&self, table: &mut Table) {
        let set = match self.registry.sets.get(&self.template_id) {
            Some(s) => s,
            None => return,
        };
        Self::add_field(table, "id", &set.id);
        Self::add_field(table, "name", &set.name);
        for (i, bonus) in set.bonuses.iter().enumerate() {
            Self::add_field(table, &format!("bonuses[{i}].min_pieces"), bonus.min_pieces);
            for (j, cond) in bonus.conditions.iter().enumerate() {
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].conditions[{j}].piece_type"),
                    &cond.piece_type,
                );
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].conditions[{j}].min"),
                    cond.min,
                );
            }
            for (j, effect) in bonus.effects.iter().enumerate() {
                Self::add_field(
                    table,
                    &format!("bonuses[{i}].effects[{j}].effect_type"),
                    &effect.effect_type,
                );
                if let Some(ref stat) = effect.stat {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].stat"), stat);
                }
                if let Some(ref amt) = effect.amount {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].amount"), amt);
                }
                if let Some(ref aura) = effect.aura_id {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].aura_id"), aura);
                }
                if let Some(ref radius) = effect.radius {
                    Self::add_field(table, &format!("bonuses[{i}].effects[{j}].radius"), radius);
                }
            }
        }
    }

    pub(super) fn update_sets(&mut self, field: &str, value: &str) -> Result<(), String> {
        let set = self
            .registry
            .sets
            .get_mut(&self.template_id)
            .ok_or_else(|| "set not found".to_string())?;
        match field {
            "id" => set.id = value.to_string(),
            "name" => set.name = value.to_string(),
            _ if field.starts_with("bonuses[") => {
                let rest = field.trim_start_matches("bonuses[");
                let (idx_str, path_rest) = rest.split_once(']').ok_or("invalid bonus path")?;
                let idx: usize = idx_str.parse().map_err(|_| "invalid index")?;
                if idx < set.bonuses.len() {
                    let bonus = &mut set.bonuses[idx];
                    if path_rest == ".min_pieces" || path_rest.starts_with(".min_pieces") {
                        bonus.min_pieces = value.parse().map_err(|_| "invalid number")?;
                    } else if path_rest.starts_with(".conditions[") {
                        let cond_rest = path_rest.trim_start_matches(".conditions[");
                        let (cidx_str, cpath_rest) =
                            cond_rest.split_once(']').ok_or("invalid cond path")?;
                        let cidx: usize = cidx_str.parse().map_err(|_| "invalid index")?;
                        if cidx < bonus.conditions.len() {
                            let cond = &mut bonus.conditions[cidx];
                            if cpath_rest == ".piece_type" {
                                cond.piece_type = value.to_string();
                            } else if cpath_rest == ".min" || cpath_rest.starts_with(".min") {
                                cond.min = value.parse().map_err(|_| "invalid number")?;
                            }
                        }
                    } else if path_rest.starts_with(".effects[") {
                        let eff_rest = path_rest.trim_start_matches(".effects[");
                        let (eidx_str, epath_rest) =
                            eff_rest.split_once(']').ok_or("invalid effect path")?;
                        let eidx: usize = eidx_str.parse().map_err(|_| "invalid index")?;
                        if eidx < bonus.effects.len() {
                            let eff = &mut bonus.effects[eidx];
                            if epath_rest == ".effect_type" {
                                eff.effect_type = value.to_string();
                            } else if epath_rest == ".stat" {
                                eff.stat = Some(value.to_string());
                            } else if epath_rest == ".amount" {
                                eff.amount = Some(value.parse().map_err(|_| "invalid number")?);
                            } else if epath_rest == ".aura_id" {
                                eff.aura_id = Some(value.to_string());
                            } else if epath_rest == ".radius" {
                                eff.radius = Some(value.parse().map_err(|_| "invalid number")?);
                            }
                        }
                    }
                }
            }
            _ => return Err(format!("unknown field: {field}")),
        }
        Ok(())
    }
}
