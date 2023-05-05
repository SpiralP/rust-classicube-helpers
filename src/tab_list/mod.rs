mod entry;

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use classicube_sys::{TabList, TABLIST_MAX_NAMES};
use tracing::warn;

pub use self::entry::TabListEntry;
use crate::{
    callback_handler::CallbackHandler,
    events::{net, tab_list},
};

/// safe access to `TabList`
#[derive(Default)]
pub struct TabList {
    entries: Rc<RefCell<HashMap<u8, Rc<TabListEntry>>>>,

    #[allow(clippy::type_complexity)]
    added_callbacks: Rc<RefCell<CallbackHandler<(u8, Weak<TabListEntry>)>>>,
    #[allow(dead_code)]
    added_handler: tab_list::AddedEventHandler,

    #[allow(clippy::type_complexity)]
    changed_callbacks: Rc<RefCell<CallbackHandler<(u8, Weak<TabListEntry>)>>>,
    #[allow(dead_code)]
    changed_handler: tab_list::ChangedEventHandler,

    removed_callbacks: Rc<RefCell<CallbackHandler<u8>>>,
    #[allow(dead_code)]
    removed_handler: tab_list::RemovedEventHandler,

    disconnected_handler: net::DisconnectedEventHandler,
}

impl TabList {
    /// register event listeners, listeners will unregister on drop
    #[must_use]
    pub fn new() -> Self {
        let entries = HashMap::with_capacity(256);
        let entries = Rc::new(RefCell::new(entries));

        let added_callbacks = Rc::new(RefCell::new(CallbackHandler::new()));
        let mut added_handler = tab_list::AddedEventHandler::new();
        {
            let entries = entries.clone();
            let added_callbacks = added_callbacks.clone();
            added_handler.on(move |tab_list::AddedEvent { id }| {
                let id = *id;
                let entry = Rc::new(match unsafe { TabListEntry::from_id(id) } {
                    None => {
                        warn!(?id, "AddedEvent TabListEntry::from_id returned None");
                        return;
                    }
                    Some(entry) => entry,
                });
                let weak = Rc::downgrade(&entry);

                {
                    let mut entries = entries.borrow_mut();
                    entries.insert(id, entry);
                }

                let mut added_callbacks = added_callbacks.borrow_mut();
                added_callbacks.handle_event((id, weak));
            });
        }

        let changed_callbacks = Rc::new(RefCell::new(CallbackHandler::new()));
        let mut changed_handler = tab_list::ChangedEventHandler::new();
        {
            let entries = entries.clone();
            let changed_callbacks = changed_callbacks.clone();
            changed_handler.on(move |tab_list::ChangedEvent { id }| {
                let id = *id;

                let entry = Rc::new(match unsafe { TabListEntry::from_id(id) } {
                    None => {
                        warn!(?id, "ChangedEvent TabListEntry::from_id returned None");
                        return;
                    }
                    Some(entry) => entry,
                });
                let weak = Rc::downgrade(&entry);
                {
                    let mut entries = entries.borrow_mut();
                    entries.entry(id).or_insert(entry);
                }

                let mut changed_callbacks = changed_callbacks.borrow_mut();
                changed_callbacks.handle_event((id, weak));
            });
        }

        let removed_callbacks = Rc::new(RefCell::new(CallbackHandler::new()));
        let mut removed_handler = tab_list::RemovedEventHandler::new();
        {
            let entries = entries.clone();
            let removed_callbacks = removed_callbacks.clone();
            removed_handler.on(move |tab_list::RemovedEvent { id }| {
                {
                    let mut entries = entries.borrow_mut();
                    entries.remove(id);
                }

                let mut removed_callbacks = removed_callbacks.borrow_mut();
                removed_callbacks.handle_event(*id);
            });
        }

        let mut disconnected_handler = net::DisconnectedEventHandler::new();
        {
            let entries = entries.clone();
            disconnected_handler.on(move |_| {
                let mut entries = entries.borrow_mut();
                entries.clear();
            });
        }

        let mut s = Self {
            entries,
            added_callbacks,
            added_handler,
            changed_callbacks,
            changed_handler,
            removed_callbacks,
            removed_handler,
            disconnected_handler,
        };

        s.update_to_real_entries();

        s
    }

    fn update_to_real_entries(&mut self) {
        let mut entries = self.entries.borrow_mut();
        entries.clear();

        for id in 0..TABLIST_MAX_NAMES {
            unsafe {
                if TabList.NameOffsets[id as usize] != 0 {
                    if let Some(entry) = TabListEntry::from_id(id as u8) {
                        entries.insert(id as u8, Rc::new(entry));
                    }
                }
            }
        }
    }

    pub fn on_added<F>(&mut self, callback: F)
    where
        F: FnMut(&(u8, Weak<TabListEntry>)),
        F: 'static,
    {
        let mut added_callbacks = self.added_callbacks.borrow_mut();
        added_callbacks.on(callback)
    }

    pub fn on_changed<F>(&mut self, callback: F)
    where
        F: FnMut(&(u8, Weak<TabListEntry>)),
        F: 'static,
    {
        let mut changed_callbacks = self.changed_callbacks.borrow_mut();
        changed_callbacks.on(callback)
    }

    pub fn on_removed<F>(&mut self, callback: F)
    where
        F: FnMut(&u8),
        F: 'static,
    {
        let mut removed_callbacks = self.removed_callbacks.borrow_mut();
        removed_callbacks.on(callback)
    }

    pub fn on_disconnected<F>(&mut self, callback: F)
    where
        F: FnMut(&net::DisconnectedEvent),
        F: 'static,
    {
        self.disconnected_handler.on(callback)
    }

    fn best_match(&self, search: &str) -> Option<Weak<TabListEntry>> {
        // tablist doesn't include <Local map chat> or [xtitles], so match from right to left
        let entries = self.entries.borrow();
        let mut positions: Vec<_> = entries
            .values()
            .filter_map(|entry| {
                let nick_name = entry.get_nick_name().replace(" &7(AFK)", "");

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
                let search = remove_color(search);
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

        positions
            .first()
            .map(|(entry, _name, _pos)| Rc::downgrade(*entry))
    }

    #[must_use]
    pub fn find_entry_by_nick_name(&self, search: &str) -> Option<Weak<TabListEntry>> {
        let entries = self.entries.borrow();
        let option = entries.values().find(|entry| {
            // try exact nick_name match first
            // this should match if there are no <Local> or tags on the front
            let nick_name = entry.get_nick_name().replace(" &7(AFK)", "");
            nick_name == search ||
                // compare with colors removed
                remove_color(&nick_name) == remove_color(search)
        });

        if let Some(a) = option {
            Some(Rc::downgrade(a))
        } else {
            let option = entries.values().find(|entry| {
                // try exact real_name match first
                // this should match if there are no <Local> or tags on the front
                let real_name = entry.get_real_name().replace(" &7(AFK)", "");
                real_name == search ||
                    // compare with colors removed
                    remove_color(&real_name) == remove_color(search)
            });

            if let Some(a) = option {
                Some(Rc::downgrade(a))
            } else {
                // exact match failed,
                // match from the right, choose the one with most chars matched

                self.best_match(search)
            }
        }
    }

    #[must_use]
    pub fn get(&self, id: u8) -> Option<Weak<TabListEntry>> {
        let entries = self.entries.borrow();
        let entry = entries.get(&id)?;
        Some(Rc::downgrade(entry))
    }

    #[must_use]
    pub fn get_all(&self) -> Vec<(u8, Weak<TabListEntry>)> {
        let entries = self.entries.borrow();
        entries
            .values()
            .map(|entry| (entry.get_id(), Rc::downgrade(entry)))
            .collect::<Vec<_>>()
    }
}

#[cfg(all(windows, not(feature = "ci")))]
#[ignore]
#[test]
fn test_find_entry_by_nick_name() {
    use classicube_sys::*;

    let tab_list = TabList::new();

    let pairs = [
        ("goodlyay", "&a&f�&a Goodly"),
        ("SpiralP", "&u&bs&fp&6i&fr&ba"),
        ("", "&9&9\u{b}&9 &rSp&9&3a&9c&1e"),
        ("SpiralP2", "&7SpiralP2    &f0"),
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

#[cfg(all(windows, not(feature = "ci")))]
#[ignore]
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

            let search = remove_beginning_color(search);
            let real_nick = remove_beginning_color(nick_name);

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
