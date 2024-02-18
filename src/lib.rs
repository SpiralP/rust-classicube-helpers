#![warn(clippy::pedantic)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

pub mod async_manager;
mod callback_handler;
pub mod chat;
pub mod color;
pub mod entities;
pub mod events;
mod macros;
pub mod shared;
pub mod tab_list;
pub mod tick;
mod traits;

pub use crate::traits::*;
pub use tracing;
