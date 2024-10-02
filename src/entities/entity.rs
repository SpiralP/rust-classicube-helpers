use std::{
    ffi::CStr,
    ptr::{addr_of, NonNull},
};

use classicube_sys::{Entities, Vec3};

/// 255 is self entity
pub const ENTITY_SELF_ID: u8 = 255;

#[derive(Debug)]
pub struct Entity {
    id: u8,
    inner: &'static mut classicube_sys::Entity,
}

impl Entity {
    /// # Safety
    ///
    /// `id` must exist.
    ///
    /// `Entity` cannot outlive the entity in-game.
    ///
    /// `Entities` will use `Weak` to make sure this dies when the entity is removed.
    pub unsafe fn from_id(id: u8) -> Option<Self> {
        let mut ptr = NonNull::new(Entities.List[id as usize])?;
        let inner = ptr.as_mut();
        Some(Self { id, inner })
    }

    #[must_use]
    pub fn get_inner(&self) -> &classicube_sys::Entity {
        self.inner
    }

    /// # Safety
    ///
    /// `Entity` cannot outlive the entity in-game.
    ///
    /// `Entities` will use `Weak` to make sure this dies when the entity is removed.
    pub fn get_inner_mut(&mut self) -> &mut classicube_sys::Entity {
        self.inner
    }

    #[must_use]
    pub fn get_id(&self) -> u8 {
        self.id
    }

    #[must_use]
    pub fn get_position(&self) -> Vec3 {
        self.inner.Position
    }

    #[must_use]
    pub fn get_head(&self) -> [f32; 2] {
        [self.inner.Pitch, self.inner.Yaw]
    }

    /// [x, y, z], numbers are 0-360
    #[must_use]
    pub fn get_rot(&self) -> [f32; 3] {
        [self.inner.RotX, self.inner.RotY, self.inner.RotZ]
    }

    #[must_use]
    pub fn get_velocity(&self) -> Vec3 {
        self.inner.Velocity
    }

    #[must_use]
    pub unsafe fn get_model(&self) -> Option<&classicube_sys::Model> {
        let mut model = NonNull::new(self.inner.Model)?;
        Some(model.as_mut())
    }

    #[must_use]
    pub fn get_model_eye_y(&self) -> f32 {
        let model = unsafe { self.get_model().expect("Entity::get_model") };
        let get_eye_y = model.GetEyeY.expect("GetEyeY");

        // it most likely doesn't mutate the Entity
        let inner_ptr = (self.inner as *const classicube_sys::Entity).cast_mut();
        unsafe { get_eye_y(inner_ptr) }
    }

    #[must_use]
    pub fn get_model_name_y(&self) -> f32 {
        let model = unsafe { self.get_model().expect("Entity::get_model") };
        let get_name_y = model.GetNameY.expect("GetNameY");

        // it most likely doesn't mutate the Entity
        let inner_ptr = (self.inner as *const classicube_sys::Entity).cast_mut();
        unsafe { get_name_y(inner_ptr) }
    }

    #[must_use]
    pub fn get_model_scale(&self) -> Vec3 {
        self.inner.ModelScale
    }

    #[must_use]
    pub fn get_display_name(&self) -> String {
        let c_str = unsafe { CStr::from_ptr(addr_of!(self.inner.NameRaw).cast()) };
        c_str.to_string_lossy().to_string()
    }

    #[must_use]
    pub fn get_eye_height(&self) -> f32 {
        self.get_model_eye_y() * self.get_model_scale().y
    }

    #[must_use]
    pub fn get_eye_position(&self) -> Vec3 {
        let mut pos = self.get_position();
        pos.y += self.get_eye_height();
        pos
    }
}
