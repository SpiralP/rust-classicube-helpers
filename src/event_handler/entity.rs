use crate::{entities::Entity, make_event_handler};
use std::os::raw::c_int;

make_event_handler!(
  Entity,
  Added,
  Int,
  (
      {
          name: entity,
          rust_type: Entity,
          c_type: c_int,
          to_rust: |id| Entity::from_id(id as u8),
      },
  )
);

make_event_handler!(
  Entity,
  Removed,
  Int,
  (
      {
          name: entity,
          rust_type: Entity,
          c_type: c_int,
          to_rust: |id| Entity::from_id(id as u8),
      },
  )
);
