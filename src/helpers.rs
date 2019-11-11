use crate::entities::Entity;
use classicube_sys::Vec3;

pub fn entity_get_eye_position(e: &mut Entity) -> Vec3 {
  let mut pos = e.get_position();
  pos.Y += entity_get_eye_height(e);
  pos
}

pub fn entity_get_eye_height(e: &mut Entity) -> f32 {
  e.get_model_eye_y() * e.get_model_scale().Y
}

pub fn vec3_get_dir_vector(yaw_rad: f32, pitch_rad: f32) -> Vec3 {
  let x = -(pitch_rad.cos()) * -(yaw_rad.sin());
  let y = -(pitch_rad.sin());
  let z = -(pitch_rad.cos()) * (yaw_rad.cos());
  vec3_create(x, y, z)
}

pub fn vec3_create(x: f32, y: f32, z: f32) -> Vec3 {
  Vec3 { X: x, Y: y, Z: z }
}
