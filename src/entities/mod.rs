mod entity;

pub use self::entity::{Entity, ENTITY_SELF_ID};
use crate::{
    create_callback,
    event_handler::{EventHandler, EventType},
    shared::SyncShared,
};
use classicube_sys::{EntityEvents, Event_RegisterInt, Event_UnregisterInt};
use std::{
    collections::HashMap,
    os::raw::{c_int, c_void},
    pin::Pin,
};

#[derive(Debug)]
pub enum EntityEvent {
    Added(c_int),
    Removed(c_int),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EntityEventType {
    Added,
    Removed,
}

impl EventType for EntityEvent {
    type EventType = EntityEventType;

    fn event_type(&self) -> Self::EventType {
        match self {
            EntityEvent::Added(..) => EntityEventType::Added,
            EntityEvent::Removed(..) => EntityEventType::Removed,
        }
    }
}

/// safe access to entities list and entity events
pub struct EntityEventListener {
    entities: SyncShared<HashMap<u8, Entity>>,
    event_handler: Pin<Box<EventHandler<EntityEvent>>>,
}

impl EntityEventListener {
    /// register event listeners, listeners will unregister on drop
    pub fn register() -> Self {
        let mut entities = SyncShared::new(HashMap::with_capacity(256));
        // add self entity which always exists
        entities.with(|entities| {
            entities.insert(ENTITY_SELF_ID, Entity::from_id(ENTITY_SELF_ID));
        });

        let mut event_handler = Box::pin(EventHandler::new());

        {
            let mut entities = entities.clone();
            event_handler.on(EntityEventType::Added, move |event| {
                if let EntityEvent::Added(id) = event {
                    let id = *id as u8;
                    let entity = Entity::from_id(id);
                    entities.lock_mut().insert(id, entity);
                }
            });
        }
        {
            let mut entities = entities.clone();
            event_handler.on(EntityEventType::Removed, move |event| {
                if let EntityEvent::Added(id) = event {
                    let id = *id as u8;
                    entities.lock_mut().remove(&id);
                }
            });
        }

        let mut this = Self {
            entities,
            event_handler,
        };

        unsafe {
            this.register_listeners();
        }

        this
    }

    unsafe fn register_listeners(&mut self) {
        let ptr: *mut EventHandler<EntityEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_RegisterInt(
            &mut EntityEvents.Added,
            ptr as *mut c_void,
            Some(on_entity_added),
        );

        Event_RegisterInt(
            &mut EntityEvents.Removed,
            ptr as *mut c_void,
            Some(on_entity_removed),
        );
    }

    unsafe fn unregister_listeners(&mut self) {
        let ptr: *mut EventHandler<EntityEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_UnregisterInt(
            &mut EntityEvents.Added,
            ptr as *mut c_void,
            Some(on_entity_added),
        );

        Event_UnregisterInt(
            &mut EntityEvents.Removed,
            ptr as *mut c_void,
            Some(on_entity_removed),
        );
    }

    pub fn get(&self, id: u8) -> Option<Entity> {
        self.entities.lock().get(&id).copied()
    }

    // TODO cloning is bad :(

    pub fn get_all(&self) -> HashMap<u8, Entity> {
        self.entities.lock().clone()
    }
}

impl Drop for EntityEventListener {
    fn drop(&mut self) {
        unsafe {
            self.unregister_listeners();
        }
    }
}

create_callback!(
    on_entity_added,
    (id: c_int),
    EntityEvent,
    EntityEvent::Added
);

create_callback!(
    on_entity_removed,
    (id: c_int),
    EntityEvent,
    EntityEvent::Removed
);
