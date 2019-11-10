use classicube_sys::Entities;
use std::ffi::CStr;

/// 255 is self entity
pub const ENTITY_SELF_ID: usize = 255;

#[derive(Debug)]
pub struct Entity {
  pub id: usize,
}

impl Entity {
  unsafe fn get_entity(&self) -> &classicube_sys::Entity {
    &*Entities.List[self.id]
  }

  pub fn get_id(&self) -> usize {
    self.id
  }

  pub fn get_pos(&self) -> [f32; 3] {
    let entity = unsafe { self.get_entity() };
    [entity.Position.X, entity.Position.Y, entity.Position.Z]
  }

  /// 0-360
  pub fn get_rot(&self) -> [f32; 3] {
    let entity = unsafe { self.get_entity() };
    [entity.RotX, entity.RotY, entity.RotZ]
  }

  pub fn get_real_name(&self) -> String {
    let entity = unsafe { self.get_entity() };
    let c_str = unsafe { CStr::from_ptr(&entity.DisplayNameRaw as *const i8) };
    c_str.to_string_lossy().to_string()
  }
}
