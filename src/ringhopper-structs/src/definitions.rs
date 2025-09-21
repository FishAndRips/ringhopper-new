//! Contains all definition-derived data.

ringhopper_structs_codegen::generate_tag_group_enum!();

use crate::*;
use alloc::vec::Vec;
use funnel_web::vector::*;
use funnel_web::color::*;
use funnel_web::string::*;
use funnel_web::id::*;
use funnel_web::rectangle::*;
use alloc::string::String;
use byteorder::ByteOrder;

ringhopper_structs_codegen::generate_tag_data_defs!();
