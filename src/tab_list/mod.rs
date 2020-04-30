mod entry;

pub use self::entry::TabListEntry;
use crate::events::{net, tab_list};
use std::{cell::UnsafeCell, collections::HashMap, rc::Rc};

type EntriesType = HashMap<u8, TabListEntry>;

/// safe access to TabList
#[derive(Default)]
pub struct TabList {
    // we can use UnsafeCell because these simple HashMap tasks
    // won't cause any extra events to be emitted, which would cause
    // recursion
    //
    // the exposed on_* methods should not emit any events or we break
    // borrowing rules
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
                log::debug!(
                    "TabList AddedEvent {:?} {:?}",
                    [
                        entry.get_real_name(),
                        entry.get_nick_name(),
                        entry.get_group(),
                    ],
                    entry.get_rank(),
                );
                let entries = unsafe { &mut *entries.get() };
                entries.insert(entry.get_id(), *entry);
            });
        }

        {
            let entries = entries.clone();
            changed.on(move |tab_list::ChangedEvent { entry }| {
                let entries = unsafe { &mut *entries.get() };
                entries.insert(entry.get_id(), *entry);
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

    fn best_match(&self, search: &str) -> Option<&TabListEntry> {
        // tablist doesn't include <Local map chat> or [xtitles], so match from right to left
        let mut positions: Vec<_> = self
            .get_all()
            .iter()
            .filter_map(|(_id, entry)| {
                let nick_name = entry.get_nick_name()?.replace(" &7(AFK)", "");

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

                // fn remove_beginning_color(s: &str) -> &str {
                //     if s.len() >= 2 && s.starts_with('&') {
                //         let (_color, s) = s.split_at(2);
                //         s
                //     } else {
                //         s
                //     }
                // }

                // remove color
                let search = remove_color(&search);
                let real_nick = remove_color(&nick_name);

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
    }

    pub fn find_entry_by_nick_name(&self, search: &str) -> Option<&TabListEntry> {
        let option = self.get_all().iter().find_map(|(_id, entry)| {
            // try exact match first
            // this should match if there are no <Local> or tags on the front
            let nick_name = entry.get_nick_name()?.replace(" &7(AFK)", "");
            if nick_name == search {
                Some(entry)
            } else {
                // compare with colors removed
                if remove_color(&nick_name) == remove_color(&search) {
                    Some(entry)
                } else {
                    None
                }
            }
        });

        if let Some(a) = option {
            Some(a)
        } else {
            // exact match failed,
            // match from the right, choose the one with most chars matched

            self.best_match(search)
        }
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

#[cfg(not(feature = "ci"))]
#[test]
fn test_find_entry_by_nick_name() {
    use classicube_sys::*;

    let tab_list = TabList::new();

    let pairs = [
        ("goodlyay", "&a&f�&a Goodly"),
        ("SpiralP", "&u&bs&fp&6i&fr&ba"),
        ("", "&9&9\u{b}&9 &rSp&9&3a&9c&1e"),
    ];

    for (i, (real_nick, nick_name)) in pairs.iter().enumerate() {
        unsafe {
            let player = OwnedString::new(*real_nick);
            let list = OwnedString::new(*nick_name);
            let group = OwnedString::new("group");
            TabList_Set(
                i as _,
                player.as_cc_string(),
                list.as_cc_string(),
                group.as_cc_string(),
                0,
            );

            Event_RaiseInt(&mut TabListEvents.Added, i as _);
        }
    }

    println!(
        "{:#?}",
        tab_list
            .find_entry_by_nick_name("&7<Local>&u&u[&9Agg.Boo&9&u] &bs&fp&6i&fr&ba")
            .unwrap()
    );

    println!(
        "{:#?}",
        tab_list
            .find_entry_by_nick_name("&9[&b� &rBi&9r&1d &b�&9] &9\u{b}&9 &rSp&3a&9c&1e")
            .unwrap()
    );
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

pub fn remove_color<T: AsRef<str>>(text: T) -> String {
    let mut found_ampersand = false;

    text.as_ref()
        .chars()
        .filter(|&c| {
            if c == '&' {
                // we remove all amps but they're kept in chat if repeated
                found_ampersand = true;
                false
            } else if found_ampersand {
                found_ampersand = false;
                false
            } else {
                true
            }
        })
        .collect()
}

#[test]
fn test_remove_color() {
    let pairs = [
        ("SpiralP", "SpiralP"),
        ("SpiralP", "SpiralP"),
        ("SpiralP", "SpiralP"),
        ("SpiralP", "&bS&fp&6i&fr&balP"),
    ];

    for (a, b) in &pairs {
        assert_eq!(remove_color(b), *a);
    }
}
