#![cfg_attr(miri, feature(alloc_layout_extra))]

mod imposter;
mod memory;

pub mod collections;

pub use crate::imposter::*;
pub use memory::*;
