mod entity;

pub use self::entity::{Entity, ENTITY_SELF_ID};
use crate::{callback_handler::CallbackHandler, events::entity::*};
use classicube_sys::{Entities, ENTITIES_MAX_COUNT};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

/// safe access to entities list and entity events
pub struct Entities {
    entities: Rc<RefCell<HashMap<u8, Rc<Entity>>>>,

    added_callbacks: Rc<RefCell<CallbackHandler<Weak<Entity>>>>,
    removed_callbacks: Rc<RefCell<CallbackHandler<u8>>>,

    #[allow(dead_code)]
    added_handler: AddedEventHandler,
    #[allow(dead_code)]
    removed_handler: RemovedEventHandler,
}

impl Entities {
    /// register event listeners, listeners will unregister on drop
    pub fn new() -> Self {
        let mut entities = HashMap::with_capacity(256);

        // add self entity which always exists
        unsafe {
            entities.insert(
                ENTITY_SELF_ID,
                Rc::new(Entity::from_id(ENTITY_SELF_ID).expect("Entity::from_id(ENTITY_SELF_ID)")),
            );
        }

        let entities = Rc::new(RefCell::new(entities));

        let mut added_handler = AddedEventHandler::new();
        let mut removed_handler = RemovedEventHandler::new();
        let added_callbacks = Rc::new(RefCell::new(CallbackHandler::new()));
        let removed_callbacks = Rc::new(RefCell::new(CallbackHandler::new()));

        {
            let entities = entities.clone();
            let added_callbacks = added_callbacks.clone();
            added_handler.on(move |AddedEvent { id }| {
                let entity = unsafe { Rc::new(Entity::from_id(*id).expect("Entity::from_id")) };
                let weak = Rc::downgrade(&entity);

                {
                    let mut entities = entities.borrow_mut();
                    entities.insert(entity.get_id(), entity);
                }

                let mut added_callbacks = added_callbacks.borrow_mut();
                added_callbacks.handle_event(weak);
            });
        }

        {
            let entities = entities.clone();
            let removed_callbacks = removed_callbacks.clone();
            removed_handler.on(move |RemovedEvent { id }| {
                {
                    let mut entities = entities.borrow_mut();
                    entities.remove(id);
                }

                let mut removed_callbacks = removed_callbacks.borrow_mut();
                removed_callbacks.handle_event(*id);
            });
        }

        let mut s = Self {
            entities,
            added_handler,
            removed_handler,
            added_callbacks,
            removed_callbacks,
        };

        s.update_to_real_entities();

        s
    }

    fn update_to_real_entities(&mut self) {
        let mut entities = self.entities.borrow_mut();
        entities.clear();

        for id in 0..ENTITIES_MAX_COUNT {
            unsafe {
                if !Entities.List[id as usize].is_null() {
                    if let Some(entity) = Entity::from_id(id as u8) {
                        entities.insert(id as u8, Rc::new(entity));
                    }
                }
            }
        }
    }

    pub fn on_added<F>(&mut self, callback: F)
    where
        F: FnMut(&Weak<Entity>),
        F: 'static,
    {
        let mut added_callbacks = self.added_callbacks.borrow_mut();
        added_callbacks.on(callback)
    }

    pub fn on_removed<F>(&mut self, callback: F)
    where
        F: FnMut(&u8),
        F: 'static,
    {
        let mut removed_callbacks = self.removed_callbacks.borrow_mut();
        removed_callbacks.on(callback)
    }

    pub fn get(&self, id: u8) -> Option<Weak<Entity>> {
        let entities = self.entities.borrow();
        let entity = entities.get(&id)?;
        Some(Rc::downgrade(entity))
    }

    pub fn with_all<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&HashMap<u8, Rc<Entity>>) -> R,
    {
        let entities = self.entities.borrow();
        f(&*entities)
    }

    pub fn with_all_mut<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<u8, Rc<Entity>>) -> R,
    {
        let mut entities = self.entities.borrow_mut();
        f(&mut *entities)
    }
}

impl Default for Entities {
    fn default() -> Self {
        Self::new()
    }
}
