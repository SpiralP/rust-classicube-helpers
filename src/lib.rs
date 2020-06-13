#![feature(coerce_unsized)]
#![feature(unsize)]
#![allow(clippy::redundant_closure_call)]

mod callback_handler;
pub mod color;
pub mod entities;
pub mod events;
mod macros;
pub mod shared;
pub mod tab_list;
pub mod tick;
mod traits;

pub use crate::{macros::*, traits::*};
