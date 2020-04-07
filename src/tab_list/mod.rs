mod entry;

pub use self::entry::TabListEntry;
use crate::events::{net, tab_list};
use std::{cell::UnsafeCell, collections::HashMap, rc::Rc};

type EntriesType = HashMap<u8, TabListEntry>;

/// safe access to TabList
#[derive(Default)]
pub struct TabList {
    entries: Rc<UnsafeCell<EntriesType>>,
    added: tab_list::AddedEventHandler,
    changed: tab_list::ChangedEventHandler,
    removed: tab_list::RemovedEventHandler,
    disconnected: net::DisconnectedEventHandler,
}

impl TabList {
    /// register event listeners, listeners will unregister on drop
    pub fn new() -> Self {
        let entries = HashMap::with_capacity(256);
        let entries = Rc::new(UnsafeCell::new(entries));

        let mut added = tab_list::AddedEventHandler::new();
        let mut changed = tab_list::ChangedEventHandler::new();
        let mut removed = tab_list::RemovedEventHandler::new();
        let mut disconnected = net::DisconnectedEventHandler::new();

        {
            let entries = entries.clone();
            added.on(move |tab_list::AddedEvent { entry }| {
                let entries = unsafe { &mut *entries.get() };
                entries.insert(entry.get_id(), entry.clone());
            });
        }

        {
            let entries = entries.clone();
            changed.on(move |tab_list::ChangedEvent { entry }| {
                let entries = unsafe { &mut *entries.get() };
                entries.insert(entry.get_id(), entry.clone());
            });
        }

        {
            let entries = entries.clone();
            removed.on(move |tab_list::RemovedEvent { id }| {
                let entries = unsafe { &mut *entries.get() };
                entries.remove(id);
            });
        }

        {
            let entries = entries.clone();
            disconnected.on(move |_| {
                let entries = unsafe { &mut *entries.get() };
                entries.clear();
            });
        }

        Self {
            entries,
            added,
            changed,
            removed,
            disconnected,
        }
    }

    pub fn on_added<F>(&mut self, callback: F)
    where
        F: FnMut(&tab_list::AddedEvent),
        F: 'static,
    {
        self.added.on(callback)
    }

    pub fn on_changed<F>(&mut self, callback: F)
    where
        F: FnMut(&tab_list::ChangedEvent),
        F: 'static,
    {
        self.changed.on(callback)
    }

    pub fn on_removed<F>(&mut self, callback: F)
    where
        F: FnMut(&tab_list::RemovedEvent),
        F: 'static,
    {
        self.removed.on(callback)
    }

    pub fn on_disconnected<F>(&mut self, callback: F)
    where
        F: FnMut(&net::DisconnectedEvent),
        F: 'static,
    {
        self.disconnected.on(callback)
    }

    pub fn find_entry_by_nick_name(&self, search: String) -> Option<&TabListEntry> {
        self.get_all()
            .iter()
            .find_map(|(_id, entry)| {
                // try exact match first
                // this should match if there are no <Local> or tags on the front
                let nick_name = entry.get_nick_name()?;
                if nick_name == search {
                    Some(entry)
                } else {
                    None
                }
            })
            .or_else(|| {
                // exact match failed,
                // match from the right, choose the one with most chars matched
                let mut positions: Vec<_> = self
                    .get_all()
                    .iter()
                    .filter_map(|(_id, entry)| {
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

                        // search in reverse
                        let search: String = search.chars().rev().collect();
                        let real_nick: String = real_nick.chars().rev().collect();

                        search.find(&real_nick).map(|pos| (entry, nick_name, pos))
                    })
                    .collect();

                // searching from the end, right to left
                // search = NotSpiralP
                // SpiralP     pos = 0
                // NotSpiralP  pos = 0
                // SpiralP2    not found

                // choose smallest find position (most matched from end to start)
                // then choose largest name size for equal positions
                positions.sort_unstable_by(|(_entry1, name1, pos1), (_entry2, name2, pos2)| {
                    pos1.partial_cmp(pos2)
                        .unwrap()
                        .then_with(|| name2.len().partial_cmp(&name1.len()).unwrap())
                });

                positions.first().map(|(entry, _name, _pos)| *entry)
            })
    }

    pub fn get(&self, id: u8) -> Option<&TabListEntry> {
        self.get_all().get(&id)
    }

    pub fn get_mut(&mut self, id: u8) -> Option<&mut TabListEntry> {
        self.get_all_mut().get_mut(&id)
    }

    pub fn get_all(&self) -> &EntriesType {
        unsafe { &*self.entries.get() }
    }

    pub fn get_all_mut(&mut self) -> &mut EntriesType {
        unsafe { &mut *self.entries.get() }
    }
}

#[test]
fn test_match_names() {
    let search = "NotSpiralP";
    let names = vec!["hello", "SpiralP", "SpiralP2", "NotSpiralP", "SpiralP2"];

    let mut positions: Vec<_> = names
        .iter()
        .filter_map(|nick_name| {
            fn remove_beginning_color(s: &str) -> &str {
                if s.len() >= 2 && s.starts_with('&') {
                    let (_color, s) = s.split_at(2);
                    s
                } else {
                    s
                }
            }

            let search = remove_beginning_color(&search);
            let real_nick = remove_beginning_color(&nick_name);

            // search in reverse
            let search: String = search.chars().rev().collect();
            let real_nick: String = real_nick.chars().rev().collect();

            search.find(&real_nick).map(|pos| (nick_name, pos))
        })
        .collect();

    positions.sort_unstable_by(|(name1, pos1), (name2, pos2)| {
        pos1.partial_cmp(pos2)
            .unwrap()
            .then_with(|| name2.len().partial_cmp(&name1.len()).unwrap())
    });
    println!("{:#?}", positions);
}
