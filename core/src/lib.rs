#![allow(ambiguous_glob_reexports)]

mod components;
pub mod dice;
mod events;
pub mod format;
mod resources;
pub mod systems;
pub mod templates;

pub use components::*;
pub use events::*;
pub use resources::*;

use hecs as _hecs;

pub struct World {
    inner: _hecs::World,
}

impl World {
    pub fn new() -> Self {
        World {
            inner: _hecs::World::new(),
        }
    }

    pub fn spawn(&mut self, bundle: impl _hecs::DynamicBundle) -> Entity {
        Entity(self.inner.spawn(bundle))
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.despawn(entity.0)
    }

    pub fn query<T: _hecs::Query>(&self) -> _hecs::QueryBorrow<'_, T> {
        self.inner.query::<T>()
    }

    pub fn query_one<T: _hecs::Query>(
        &self,
        entity: Entity,
    ) -> Result<_hecs::QueryOne<'_, T>, _hecs::NoSuchEntity> {
        self.inner.query_one::<T>(entity.0)
    }

    pub fn insert<T: _hecs::DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: T,
    ) -> Result<(), _hecs::NoSuchEntity> {
        self.inner.insert(entity.0, bundle)
    }

    pub fn remove_one<T: _hecs::Component>(
        &mut self,
        entity: Entity,
    ) -> Result<T, _hecs::ComponentError> {
        self.inner.remove_one::<T>(entity.0)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(_hecs::Entity);

impl Entity {
    pub fn id(&self) -> u32 {
        self.0.id()
    }
}

impl From<_hecs::Entity> for Entity {
    fn from(e: _hecs::Entity) -> Self {
        Entity(e)
    }
}
