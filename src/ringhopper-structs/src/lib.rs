#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub use funnel_web;

ringhopper_structs_codegen::generate_tag_group_enum!();

mod tag_path;
mod util;
mod io;
mod address;

pub use tag_path::*;
pub use io::*;
pub use address::*;
pub mod tag_structs;
