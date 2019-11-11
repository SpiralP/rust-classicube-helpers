use classicube_sys::{Entities, Vec3};
use std::ffi::CStr;

/// 255 is self entity
pub const ENTITY_SELF_ID: u8 = 255;

#[derive(Debug)]
pub struct Entity {
  id: u8,
}

impl Entity {
  pub fn from_id(id: u8) -> Self {
    Self { id }
  }

  #[inline]
  pub fn get_id(&self) -> u8 {
    self.id
  }

  #[inline]
  unsafe fn get_entity(&self) -> &classicube_sys::Entity {
    &*Entities.List[self.id as usize]
  }

  #[inline]
  pub fn get_position(&self) -> Vec3 {
    let entity = unsafe { self.get_entity() };
    entity.Position
  }

  #[inline]
  pub fn get_head(&self) -> [f32; 2] {
    let entity = unsafe { self.get_entity() };
    [entity.HeadX, entity.HeadY]
  }

  /// [x, y, z], numbers are 0-360
  #[inline]
  pub fn get_rot(&self) -> [f32; 3] {
    let entity = unsafe { self.get_entity() };
    [entity.RotX, entity.RotY, entity.RotZ]
  }

  #[inline]
  pub fn get_velocity(&self) -> Vec3 {
    let entity = unsafe { self.get_entity() };
    entity.Velocity
  }

  #[inline]
  pub fn get_model_eye_y(&self) -> f32 {
    let entity = unsafe { self.get_entity() };
    let model = unsafe { &*entity.Model };
    let get_eye_y = model.GetEyeY.unwrap();

    // it most likely doesn't mutate the Entity
    let entity = entity as *const _ as *mut _;
    unsafe { (get_eye_y)(entity) }
  }

  #[inline]
  pub fn get_model_scale(&self) -> Vec3 {
    let entity = unsafe { self.get_entity() };
    entity.ModelScale
  }

  #[inline]
  pub fn get_display_name(&self) -> String {
    let entity = unsafe { self.get_entity() };
    let c_str = unsafe { CStr::from_ptr(&entity.DisplayNameRaw as *const i8) };
    c_str.to_string_lossy().to_string()
  }

  #[inline]
  pub fn get_eye_position(&self) -> Vec3 {
    let mut pos = self.get_position();
    pos.Y += self.get_eye_height();
    pos
  }

  #[inline]
  pub fn get_eye_height(&self) -> f32 {
    self.get_model_eye_y() * self.get_model_scale().Y
  }
}
