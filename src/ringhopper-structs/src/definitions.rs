//! Contains all definition-derived data.

ringhopper_structs_codegen::generate_tag_group_enum!();

/// Contains all enums from the definitions.
pub mod enums {
    use crate::*;
    use byteorder::ByteOrder;
    ringhopper_structs_codegen::generate_tag_enums!();
}

/// Contains all bitfields from the definitions.
pub mod bitfields {
    use crate::*;
    use byteorder::ByteOrder;
    ringhopper_structs_codegen::generate_tag_bitfields!();
}

/// Contains all structs from the definitions.
pub mod structs {
    use crate::*;
    use alloc::vec::Vec;
    use funnel_web::vector::*;
    use funnel_web::color::*;
    use funnel_web::string::*;
    use funnel_web::id::*;
    use funnel_web::rectangle::*;
    use super::bitfields::*;
    use super::enums::*;
    use super::TagGroup;
    use alloc::string::String;
    use byteorder::ByteOrder;
    ringhopper_structs_codegen::generate_tag_structs!();
}
