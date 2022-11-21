use std::os::raw::c_int;

use classicube_sys::{StringsBuffer_UNSAFE_Get, TabList};

#[derive(Debug)]
pub struct TabListEntry {
    id: u8,
    name_offset: &'static u16,
    group_rank: &'static u8,
}

impl TabListEntry {
    /// # Safety
    ///
    /// `id` must exist.
    ///
    /// `Entity` cannot outlive the entity in-game.
    ///
    /// `Entities` will use `Weak` to make sure this dies when the entity is removed.
    pub unsafe fn from_id(id: u8) -> Option<Self> {
        let name_offset = &TabList.NameOffsets[id as usize];
        let group_rank = &TabList.GroupRanks[id as usize];

        if *name_offset == 0 || *group_rank == 0 || TabList._buffer.count == 0 {
            return None;
        }

        Some(Self {
            id,
            name_offset,
            group_rank,
        })
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }

    /// or "Player"
    pub fn get_real_name(&self) -> String {
        unsafe {
            StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(*self.name_offset - 3))
        }
        .to_string()
    }

    /// or "Text" or "list"
    pub fn get_nick_name(&self) -> String {
        unsafe {
            StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(*self.name_offset - 2))
        }
        .to_string()
    }

    pub fn get_group(&self) -> String {
        unsafe {
            StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(*self.name_offset - 1))
        }
        .to_string()
    }

    pub fn get_rank(&self) -> u8 {
        *self.group_rank
    }
}
