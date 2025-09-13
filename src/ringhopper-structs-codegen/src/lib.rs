use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_tag_group_enum(_: TokenStream) -> TokenStream {
    let definitions = ringhopper_definitions::load_all_definitions();

    let mut q = String::with_capacity(1024 * 1024 * 1);

    q += "/// Defines a type of tag.\n";
    q += "///\n";
    q += "/// Internally, this is represented as a 32-bit `u32` (a FourCC).\n";
    q += "#[derive(Copy, Clone, PartialEq, Debug)]\n";
    q += "#[repr(u32)]\n";

    // the enum
    q += "pub enum TagGroup {\n";
    for group in definitions.groups.values() {
        std::fmt::write(&mut q, format_args!("{} = 0x{:08X},\n", group.struct_name, group.fourcc_binary)).unwrap();
    }
    q += "}\n";

    q += "impl TagGroup {\n";


    // str impl
    q += "/// Get the string equivalent of the tag group.\n";
    q += "///\n";
    q += "/// This is what is used for file extensions, and it is displayable to the user.\n";
    q += "pub const fn as_str(self) -> &'static str {\n";
    q += "match self {\n";
    for group in definitions.groups.values() {
        std::fmt::write(&mut q, format_args!("Self::{struct_name}=>\"{name}\",\n", struct_name = group.struct_name, name = group.name)).unwrap();
    }
    q += "}\n";
    q += "}\n";


    // from_str impl
    q += "/// Instantiate a TagGroup from a `str`.\n";
    q += "pub fn from_str(s: &str) -> Option<TagGroup> {\n";
    q += "match s {\n";
    for group in definitions.groups.values() {
        std::fmt::write(&mut q, format_args!("\"{name}\"=>Some(Self::{struct_name}),\n", struct_name = group.struct_name, name = group.name)).unwrap();
    }
    q += "_ => None\n";
    q += "}\n";
    q += "}\n";


    // as_u32 impl
    q += "/// Get the integer equivalent of this value.\n";
    q += "pub const fn as_u32(self) -> u32 {\n";
    q += "self as u32\n";
    q += "}\n";


    // from_u32 impl
    q += "/// Instantiate a TagGroup from an integer.\n";
    q += "pub const fn from_u32(u: u32) -> Option<TagGroup> {\n";
    q += "match u {\n";
    for group in definitions.groups.values() {
        std::fmt::write(&mut q, format_args!("0x{fourcc:08X}=>Some(Self::{struct_name}),\n", struct_name = group.struct_name, fourcc = group.fourcc_binary)).unwrap();
    }
    q += "_ => None\n";
    q += "}\n";
    q += "}\n";

    q += "}\n";

    // Display impl
    q += "impl core::fmt::Display for TagGroup {\n";
    q += "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n";
    q += "f.write_str(self.as_str())\n";
    q += "}\n";
    q += "}\n";

    q.parse().expect("failed to parse generate_tag_group_enum result")
}


#[proc_macro]
pub fn generate_tag_structs(_: TokenStream) -> TokenStream {
    todo!()
}
