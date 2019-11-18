mod entry;
mod events;

pub use self::{entry::TabListEntry, events::*};
use crate::EventHandler;
use classicube_sys::{
  Event_RegisterInt, Event_RegisterVoid, Event_UnregisterInt, Event_UnregisterVoid, NetEvents,
  TabListEvents,
};
use std::{
  cell::UnsafeCell,
  collections::HashMap,
  os::raw::{c_int, c_void},
  pin::Pin,
  rc::Rc,
};

type EntriesType = HashMap<u8, TabListEntry>;

/// safe access to TabList
pub struct TabList {
  entries: Rc<UnsafeCell<EntriesType>>,
  event_handler: Pin<Box<EventHandler<TabListEvent>>>,
}

impl TabList {
  /// register event listeners, listeners will unregister on drop
  pub fn register() -> Self {
    let entries = HashMap::with_capacity(256);
    let entries = Rc::new(UnsafeCell::new(entries));

    let mut event_handler = Box::pin(EventHandler::new());

    {
      let entries = entries.clone();
      event_handler.on(TabListEventType::Added, move |event| {
        if let TabListEvent::Added(entry) = event {
          let entries = unsafe { &mut *entries.get() };
          entries.insert(entry.get_id(), entry.clone());
        }
      });
    }

    {
      let entries = entries.clone();
      event_handler.on(TabListEventType::Changed, move |event| {
        if let TabListEvent::Changed(entry) = event {
          let entries = unsafe { &mut *entries.get() };
          entries.insert(entry.get_id(), entry.clone());
        }
      });
    }

    {
      let entries = entries.clone();
      event_handler.on(TabListEventType::Removed, move |event| {
        if let TabListEvent::Removed(id) = event {
          let entries = unsafe { &mut *entries.get() };
          entries.remove(id);
        }
      });
    }

    {
      let entries = entries.clone();
      event_handler.on(TabListEventType::Disconnected, move |event| {
        if let TabListEvent::Disconnected = event {
          let entries = unsafe { &mut *entries.get() };
          entries.clear();
        }
      });
    }

    let mut this = Self {
      entries,
      event_handler,
    };

    unsafe {
      this.register_listeners();
    }

    this
  }

  pub fn on<F>(&mut self, event_type: TabListEventType, callback: F)
  where
    F: Fn(&TabListEvent),
    F: 'static,
  {
    self.event_handler.on(event_type, callback);
  }

  unsafe fn register_listeners(&mut self) {
    let ptr: *mut EventHandler<TabListEvent> = self.event_handler.as_mut().get_unchecked_mut();

    Event_RegisterInt(
      &mut TabListEvents.Added,
      ptr as *mut c_void,
      Some(on_tablist_added),
    );
    Event_RegisterInt(
      &mut TabListEvents.Changed,
      ptr as *mut c_void,
      Some(on_tablist_changed),
    );
    Event_RegisterInt(
      &mut TabListEvents.Removed,
      ptr as *mut c_void,
      Some(on_tablist_removed),
    );

    Event_RegisterVoid(
      &mut NetEvents.Disconnected,
      ptr as *mut c_void,
      Some(on_disconnected),
    );
  }

  unsafe fn unregister_listeners(&mut self) {
    let ptr: *mut EventHandler<TabListEvent> = self.event_handler.as_mut().get_unchecked_mut();

    Event_UnregisterInt(
      &mut TabListEvents.Added,
      ptr as *mut c_void,
      Some(on_tablist_added),
    );
    Event_UnregisterInt(
      &mut TabListEvents.Changed,
      ptr as *mut c_void,
      Some(on_tablist_changed),
    );
    Event_UnregisterInt(
      &mut TabListEvents.Removed,
      ptr as *mut c_void,
      Some(on_tablist_removed),
    );

    Event_UnregisterVoid(
      &mut NetEvents.Disconnected,
      ptr as *mut c_void,
      Some(on_disconnected),
    );
  }

  pub fn find_entity_id_by_name(&self, search: String) -> Option<u8> {
    self
      .get_all()
      .iter()
      .find_map(|(id, entry)| {
        // try exact match first
        let nick_name = entry.get_nick_name()?;
        if nick_name == search {
          Some(*id)
        } else {
          None
        }
      })
      .or_else(|| {
        // exact match failed,
        // match from the right, choose the one with most chars matched
        let mut id_positions: Vec<(_, usize)> = self
          .get_all()
          .iter()
          .filter_map(|(id, entry)| {
            let nick_name = entry.get_nick_name()?;

            // search: &0<Realm 7&0> &dAdo&elf Hit&aler
            // entry :               ^
            // &3[arsclacxe&3] &aPee&2birb
            //                 ^
            // &x<&xVIP&x> &x[&lGod's Architect&x] &x[&eΩ&x] Kylbert
            //                                     ^
            // &c[&4Co&4m&6mmu&4nist&c] TEHNOOBSHOW
            //                          ^ (notice the color is at "&c[")
            // &3SpiralP
            // ^ (matched by exact)
            // &7S0m
            // ^ (matched by exact)

            fn remove_beginning_color(s: &str) -> &str {
              if s.len() >= 2 && s.starts_with('&') {
                let (_color, s) = s.split_at(2);
                s
              } else {
                s
              }
            }

            // remove color at beginning
            let search = remove_beginning_color(&search);
            let real_nick = remove_beginning_color(&nick_name);

            search.rfind(&real_nick).map(|pos| (*id, pos))
          })
          .collect();

        // choose smallest position, or "most chars matched"
        id_positions.sort_unstable_by(|(id1, pos1), (id2, pos2)| {
          pos1
            .partial_cmp(pos2)
            .unwrap()
            .then_with(|| id1.partial_cmp(&id2).unwrap())
        });

        id_positions.first().map(|(id, _pos)| *id)
      })
      .map(|a| a as u8)
  }

  #[inline]
  pub fn get(&self, id: u8) -> Option<&TabListEntry> {
    self.get_all().get(&id)
  }

  #[inline]
  pub fn get_mut(&mut self, id: u8) -> Option<&mut TabListEntry> {
    self.get_all_mut().get_mut(&id)
  }

  #[inline]
  pub fn get_all(&self) -> &EntriesType {
    unsafe { &*self.entries.get() }
  }

  #[inline]
  pub fn get_all_mut(&mut self) -> &mut EntriesType {
    unsafe { &mut *self.entries.get() }
  }
}

impl Drop for TabList {
  fn drop(&mut self) {
    self.get_all_mut().clear();

    unsafe {
      self.unregister_listeners();
    }
  }
}

extern "C" fn on_tablist_added(obj: *mut c_void, id: c_int) {
  let event_handler = obj as *const EventHandler<TabListEvent>;
  let event_handler = unsafe { &*event_handler };
  let id = id as u8;

  event_handler.handle_event(TabListEvent::Added(TabListEntry::from_id(id)));
}

extern "C" fn on_tablist_changed(obj: *mut c_void, id: c_int) {
  let event_handler = obj as *const EventHandler<TabListEvent>;
  let event_handler = unsafe { &*event_handler };
  let id = id as u8;

  event_handler.handle_event(TabListEvent::Changed(TabListEntry::from_id(id)));
}

extern "C" fn on_tablist_removed(obj: *mut c_void, id: c_int) {
  let event_handler = obj as *const EventHandler<TabListEvent>;
  let event_handler = unsafe { &*event_handler };
  let id = id as u8;

  event_handler.handle_event(TabListEvent::Removed(id));
}

extern "C" fn on_disconnected(obj: *mut c_void) {
  let event_handler = obj as *const EventHandler<TabListEvent>;
  let event_handler = unsafe { &*event_handler };

  event_handler.handle_event(TabListEvent::Disconnected);
}
