#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use core::fmt::{Debug, Formatter};
pub use funnel_web;

mod tag_path;
mod util;
mod io;
mod address;

#[derive(Copy, Clone)]
pub union ScenarioScriptNodeValue {
    pub real: f32,
    pub short: i16,
    pub long: i32,
    pub id: u32
}

impl Debug for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // SAFETY: This is just viewing it as an int which shouldn't be dangerous
        f.write_fmt(format_args!("0x{}??", unsafe { self.id }))
    }
}

impl PartialEq for ScenarioScriptNodeValue {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: This is just viewing it as an int which shouldn't be dangerous
        unsafe { self.id == other.id }
    }
}

pub use tag_path::*;
pub use io::*;
pub use address::*;

pub mod definitions;
