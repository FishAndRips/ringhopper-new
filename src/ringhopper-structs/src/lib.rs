#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub use funnel_web;

mod tag_path;
mod util;
mod simple_io;
mod tag_io;
mod address;
mod bounds;
mod scenario_script_node;
mod reflexive;

pub use tag_path::*;
pub use simple_io::*;
pub use address::*;
pub use bounds::*;
pub use scenario_script_node::*;
pub use tag_io::*;
pub use reflexive::*;

pub mod definitions;
