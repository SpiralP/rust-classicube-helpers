use classicube_sys::{StringsBuffer_UNSAFE_Get, TabList};
use std::os::raw::c_int;

#[derive(Debug, Clone, Copy)]
pub struct TabListEntry {
    id: u8,
}

impl TabListEntry {
    pub fn from_id(id: u8) -> Self {
        Self { id }
    }

    pub fn get_id(self) -> u8 {
        self.id
    }

    /// or "Player"
    pub fn get_real_name(self) -> Option<String> {
        let offset = unsafe { TabList.NameOffsets[self.id as usize] };

        unsafe {
            if offset != 0 && TabList._buffer.count != 0 {
                Some(
                    StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(offset - 3))
                        .to_string(),
                )
            } else {
                None
            }
        }
    }

    /// or "Text" or "list"
    pub fn get_nick_name(self) -> Option<String> {
        let offset = unsafe { TabList.NameOffsets[self.id as usize] };

        unsafe {
            if offset != 0 && TabList._buffer.count != 0 {
                Some(
                    StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(offset - 2))
                        .to_string(),
                )
            } else {
                None
            }
        }
    }

    pub fn get_group(self) -> Option<String> {
        let offset = unsafe { TabList.NameOffsets[self.id as usize] };

        unsafe {
            if offset != 0 && TabList._buffer.count != 0 {
                Some(
                    StringsBuffer_UNSAFE_Get(&mut TabList._buffer, c_int::from(offset - 1))
                        .to_string(),
                )
            } else {
                None
            }
        }
    }

    pub fn get_rank(self) -> Option<u8> {
        let rank = unsafe { TabList.GroupRanks[self.id as usize] };
        if rank == 0 {
            None
        } else {
            Some(rank)
        }
    }
}
