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

#[derive(Copy, Clone, PartialEq, Default)]
#[repr(transparent)]
pub struct ScenarioScriptNodeValue {
    pub data: u32
}

impl Debug for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{}??", self.data))
    }
}

pub use tag_path::*;
pub use io::*;
pub use address::*;

pub mod definitions;
