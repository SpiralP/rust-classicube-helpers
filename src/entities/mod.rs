mod entity;

pub use self::entity::{Entity, ENTITY_SELF_ID};
use classicube_sys::{EntityEvents, Event_RegisterInt, Event_UnregisterInt};
use std::{
  collections::HashMap,
  os::raw::{c_int, c_void},
};

/// safe access to entities list
pub struct Entities {
  entities: HashMap<u8, Entity>,
}

impl Entities {
  /// register event listeners, listeners will unregister on drop
  pub fn register() -> Self {
    let mut this = {
      let mut entities = HashMap::with_capacity(256);
      // add self entity which always exists
      entities.insert(ENTITY_SELF_ID, Entity::from_id(ENTITY_SELF_ID));

      Self { entities }
    };

    this.register_listeners();

    this
  }

  fn register_listeners(&mut self) {
    let ptr: *mut Entities = self;

    unsafe {
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
  }

  fn unregister_listeners(&mut self) {
    let ptr: *mut Entities = self;

    unsafe {
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
  }

  pub fn get(&self, id: u8) -> Option<&Entity> {
    self.entities.get(&id)
  }
}

impl Drop for Entities {
  fn drop(&mut self) {
    self.unregister_listeners();
  }
}

extern "C" fn on_entity_added(obj: *mut c_void, id: c_int) {
  let module = obj as *mut Entities;
  let module = unsafe { &mut *module };
  let id = id as u8;

  module.entities.insert(id, Entity::from_id(id));
}

extern "C" fn on_entity_removed(obj: *mut c_void, id: c_int) {
  let module = obj as *mut Entities;
  let module = unsafe { &mut *module };
  let id = id as u8;

  module.entities.remove(&id);
}
