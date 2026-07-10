use crate::Entity;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootMode {
    FreeForAll,
    RoundRobin,
    MasterLooter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formation {
    Default,
    Line,
    Scattered,
    Column,
    Wedge,
    ShieldWall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupRole {
    Leader,
    Member,
}

#[derive(Debug, Clone)]
pub struct GroupMemberInfo {
    pub entity: Option<Entity>, // None if disconnected
    pub db_id: i64,
    pub name: String,
    pub joined_at: Instant,
    pub disconnected_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub leader: Entity,
    pub members: Vec<GroupMemberInfo>,
    pub loot_mode: LootMode,
    pub formation: Formation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupMember {
    pub group_id: Entity, // points to group entity
    pub role: GroupRole,
}

#[derive(Debug, Clone)]
pub struct GroupInvite {
    pub target: Entity,
    pub from: Entity,
    pub group_id: Option<Entity>,
    pub expires_at: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct GroupManager {
    pub invites: Vec<GroupInvite>,
}
