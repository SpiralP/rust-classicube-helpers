mod entry;

pub use self::entry::TabListEntry;
use classicube_sys::{Event_RegisterInt, Event_UnregisterInt, TabListEvents};
use std::{
  collections::HashMap,
  os::raw::{c_int, c_void},
};

/// safe access to TabList
pub struct TabList {
  entries: HashMap<u8, TabListEntry>,
}

impl TabList {
  /// register event listeners, listeners will unregister on drop
  pub fn register() -> Self {
    let mut this = Self {
      entries: HashMap::with_capacity(256),
    };

    this.register_listeners();

    this
  }

  fn register_listeners(&mut self) {
    let ptr: *mut TabList = self;

    unsafe {
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
    }
  }

  fn unregister_listeners(&mut self) {
    let ptr: *mut TabList = self;

    unsafe {
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
    }
  }

  pub fn find_entity_id_by_name(&self, search: String) -> Option<u8> {
    self
      .entries
      .iter()
      .find_map(|(id, entry)| {
        // try exact match first
        let nick_name = entry.get_nick_name();
        if nick_name == search {
          Some(*id)
        } else {
          None
        }
      })
      .or_else(|| {
        // exact match failed,
        // match from the right, choose the one with most chars matched
        let mut id_positions: Vec<(u8, usize)> = self
          .entries
          .iter()
          .filter_map(|(id, entry)| {
            let nick_name = entry.get_nick_name();

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
  }
}

impl Drop for TabList {
  fn drop(&mut self) {
    self.unregister_listeners();
  }
}

extern "C" fn on_tablist_added(obj: *mut c_void, id: c_int) {
  let module = obj as *mut TabList;
  let module = unsafe { &mut *module };
  let id = id as u8;

  module.entries.insert(id, TabListEntry::from_id(id));
}

extern "C" fn on_tablist_changed(obj: *mut c_void, id: c_int) {
  let module = obj as *mut TabList;
  let module = unsafe { &mut *module };
  let id = id as u8;

  module.entries.insert(id, TabListEntry::from_id(id));
}

extern "C" fn on_tablist_removed(obj: *mut c_void, id: c_int) {
  let module = obj as *mut TabList;
  let module = unsafe { &mut *module };
  let id = id as u8;

  module.entries.remove(&id);
}
