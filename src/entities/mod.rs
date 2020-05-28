mod entity;

pub use self::entity::{Entity, ENTITY_SELF_ID};
use crate::events::entity::*;
use classicube_sys::{Entities, ENTITIES_MAX_COUNT};
use std::{cell::UnsafeCell, collections::HashMap, rc::Rc};

/// safe access to entities list and entity events
#[derive(Default)]
pub struct Entities {
    entities: Rc<UnsafeCell<HashMap<u8, Entity>>>,

    added: AddedEventHandler,
    removed: RemovedEventHandler,
}

impl Entities {
    /// register event listeners, listeners will unregister on drop
    pub fn new() -> Self {
        let mut entities = HashMap::with_capacity(256);

        // add self entity which always exists
        entities.insert(ENTITY_SELF_ID, Entity::from_id(ENTITY_SELF_ID));

        let entities = Rc::new(UnsafeCell::new(entities));

        let mut added = AddedEventHandler::new();
        let mut removed = RemovedEventHandler::new();

        {
            let entities = entities.clone();
            added.on(move |AddedEvent { entity }| {
                let entities = unsafe { &mut *entities.get() };
                entities.insert(entity.get_id(), *entity);
            });
        }

        {
            let entities = entities.clone();
            removed.on(move |RemovedEvent { entity }| {
                let entities = unsafe { &mut *entities.get() };
                entities.remove(&entity.get_id());
            });
        }

        let mut s = Self {
            entities,
            added,
            removed,
        };

        s.update_to_real_entities();

        s
    }

    fn update_to_real_entities(&mut self) {
        let entities = unsafe { &mut *self.entities.get() };
        entities.clear();

        for id in 0..ENTITIES_MAX_COUNT {
            if !unsafe { Entities.List[id as usize] }.is_null() {
                entities.insert(id as u8, Entity::from_id(id as u8));
            }
        }
    }

    pub fn on_added<F>(&mut self, callback: F)
    where
        F: FnMut(&AddedEvent),
        F: 'static,
    {
        self.added.on(callback)
    }

    pub fn on_removed<F>(&mut self, callback: F)
    where
        F: FnMut(&RemovedEvent),
        F: 'static,
    {
        self.removed.on(callback)
    }

    pub fn get(&self, id: u8) -> Option<&Entity> {
        self.get_all().get(&id)
    }

    pub fn get_mut(&mut self, id: u8) -> Option<&mut Entity> {
        self.get_all_mut().get_mut(&id)
    }

    pub fn get_all(&self) -> &HashMap<u8, Entity> {
        unsafe { &*self.entities.get() }
    }

    pub fn get_all_mut(&mut self) -> &mut HashMap<u8, Entity> {
        unsafe { &mut *self.entities.get() }
    }
}
