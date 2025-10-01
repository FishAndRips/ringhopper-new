use proc_macro::TokenStream;
use std::collections::HashSet;
use ringhopper_definitions::{load_all_definitions, Bitfield, EngineCompressionType, Enum, FieldCount, FieldObject, NamedObject, ParsedDefinitions, SizeableObject, Struct, StructField, StructFieldType};
use std::fmt::write;
use std::fmt::Write;

#[proc_macro]
pub fn generate_tag_group_enum(_: TokenStream) -> TokenStream {
    let definitions = ringhopper_definitions::load_all_definitions();

    let mut q = String::with_capacity(1024 * 1024 * 2);

    q += "/// Defines a type of tag.\n";
    q += "///\n";
    q += "/// Internally, this is represented as a 32-bit `u32` (a FourCC).\n";
    q += "#[derive(Copy, Clone, PartialEq, Debug, Default, PartialOrd, Ord, Eq)]\n";
    q += "#[repr(u32)]\n";

    // the enum
    q += "pub enum TagGroup {\n";
    q += "#[default] None = 0xFFFFFFFF,\n";
    for group in definitions.groups.values() {
        write(&mut q, format_args!("{} = 0x{:08X},\n", group.name_rust_enum, group.fourcc_binary)).unwrap();
    }
    q += "}\n";

    q += "impl TagGroup {\n";


    // str impl
    q += "/// Get the string equivalent of the tag group.\n";
    q += "///\n";
    q += "/// This is what is used for file extensions, and it is displayable to the user.\n";
    q += "pub const fn as_str(self) -> &'static str {\n";
    q += "match self {\n";
    q += "Self::None => \"none\",\n";
    for group in definitions.groups.values() {
        write(&mut q, format_args!("Self::{name_enum}=>\"{name}\",\n", name_enum = group.name_rust_enum, name = group.name)).unwrap();
    }
    q += "}\n";
    q += "}\n";


    // from_str impl
    q += "/// Instantiate a TagGroup from a `str`.\n";
    q += "pub fn from_str(s: &str) -> Option<TagGroup> {\n";
    q += "match s {\n";
    q += "\"none\"=>Some(Self::None),\n";
    for group in definitions.groups.values() {
        write(&mut q, format_args!("\"{name}\"=>Some(Self::{name_enum}),\n", name_enum = group.name_rust_enum, name = group.name)).unwrap();
    }
    q += "_ => None\n";
    q += "}\n";
    q += "}\n";


    // version impl
    q += "/// Get the version of the tag group.\n";
    q += "pub const fn version(self) -> u16 {\n";
    q += "match self {\n";
    q += "Self::None=>0,\n";
    for group in definitions.groups.values() {
        write(&mut q, format_args!("Self::{name_enum}=>{version},\n", name_enum = group.name_rust_enum, version = group.version)).unwrap();
    }
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
        write(&mut q, format_args!("0x{fourcc:08X}=>Some(Self::{name_enum}),\n", name_enum = group.name_rust_enum, fourcc = group.fourcc_binary)).unwrap();
    }
    q += "0xFFFFFFFF | 0x00000000 => Some(TagGroup::None),\n";
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


    // str impl
    q += "/// Load the tag data.\n";
    q += "pub fn read_editable_tag(data: &[u8], parameters: Parameters) -> Result<alloc::boxed::Box<dyn EditableTag>, WriteableDataError> {\n";
    q += "let header = tag::TagFileHeader::read_tag_data::<byteorder::BigEndian>(data, 0x0, &mut TagFileHeader::base_length(), parameters)?;\n";
    q += "match header.tag_group {\n";
    for group in definitions.groups.values() {
        write(&mut q, format_args!("TagGroup::{name_enum}=>read_tag_file::<{struct_name}>(data, parameters).map(|t| alloc::boxed::Box::new(t) as alloc::boxed::Box<dyn EditableTag>),\n", name_enum = group.name_rust_enum, struct_name = group.struct_name)).unwrap();
    }
    q += "_ => Err(WriteableDataError::Other { description: alloc::borrow::Cow::Borrowed(\"no tag group\") }),\n";
    q += "}\n";
    q += "}\n";

    q.parse().expect("failed to parse generate_tag_group_enum result")
}

#[proc_macro]
pub fn generate_tag_data_defs(_: TokenStream) -> TokenStream {
    let definitions = ringhopper_definitions::load_all_definitions();

    let all_modules_needed_set: HashSet<&str> = definitions
        .objects
        .values()
        .map(|o| o.definition_file())
        .collect();

    let mut all_modules_needed_vec: Vec<&str> = Vec::with_capacity(all_modules_needed_set.len());
    all_modules_needed_vec.extend(all_modules_needed_set);
    all_modules_needed_vec.dedup();

    let mut q = String::with_capacity(1024 * 1024 * 32);
    for file in &all_modules_needed_vec {
        let mut safe_name = file[file.rfind("/").unwrap() + 1..file.len() - 5].to_string(); // omit ".json"
        if safe_name == "enum" {
            safe_name += "s";
        }

        write(&mut q, format_args!("pub mod {safe_name} {{\n")).unwrap();
        write(&mut q, format_args!("use super::*;\n")).unwrap();

        for i in definitions.objects.values() {
            if i.definition_file() != *file {
                continue;
            }

            match i {
                NamedObject::Struct(s) => generate_struct(&mut q, s, definitions),
                NamedObject::Bitfield(b) => generate_bitfield(&mut q, b),
                NamedObject::Enum(e) => generate_enum(&mut q, e)
            }
        }

        q += "}\n";

        write(&mut q, format_args!("use {safe_name}::*;\n")).unwrap();
    }

    q.parse().expect("failed to parse generate_tag_structs result")
}



fn generate_enum(q: &mut String, e: &Enum) {
    let name = &e.name;

    *q += "#[derive(Copy, Clone, PartialEq, Debug, Default)]\n";
    *q += "#[repr(u16)]\n";
    write(q, format_args!("pub enum {name} {{\n")).unwrap();
    let mut default_defined = false;
    for field in &e.options {
        if field.flags.exclude {
            continue
        }
        let name = &field.name_rust_enum;
        let value = field.value;
        if !default_defined {
            default_defined = true;
            *q += "#[default]\n";
        }
        write(q, format_args!("{name} = {value},\n")).unwrap();
    }
    *q += "}\n";

    write(q, format_args!("impl SimpleWriteableData for {name} {{\n")).unwrap();
    *q += "#[inline]\n";
    *q += "fn length() -> usize { 2 }\n";

    *q += "fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {\n";
    *q += "match u16::read_tag_data_simple::<B>(from, parameters)? {\n";
    for field in &e.options {
        if field.flags.exclude {
            continue
        }

        let field_name = &field.name_rust_enum;
        write(q, format_args!("0x{:04X} ", field.value)).unwrap();
        if field.flags.cache_only {
            *q += "if parameters.cache_only_fields ";
        }
        else if field.flags.non_cached {
            *q += "if parameters.tag_only_fields ";
        }
        *q += "=> ";
        write(q, format_args!("Ok(Self::{field_name}),\n")).unwrap();
    }
    write(q, format_args!("_ => Err(\"invalid enum value for {name}\"),\n")).unwrap();
    *q += "}\n";
    *q += "}\n";

    *q += "#[inline]\n";
    *q += "fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {\n";
    *q += "(*self as u16).write_tag_data_simple::<B>(to, parameters);\n";
    *q += "}\n";
    *q += "}\n";

    write(q, format_args!("impl EditableTagField for {name} {{\n")).unwrap();
    *q += "#[inline] fn get_enum(&self) -> Option<&dyn EditableEnumTagField> { Some(self) }\n";
    *q += "#[inline] fn get_enum_mut(&mut self) -> Option<&mut dyn EditableEnumTagField> { Some(self) }\n";
    *q += "}\n";

    write(q, format_args!("impl EditableEnumTagField for {name} {{\n")).unwrap();

    let mut get_value = String::with_capacity(1024 * 64);
    let mut set_value = String::with_capacity(1024 * 64);

    *q += "#[inline]\n";
    *q += "fn values(&self) -> &'static [&'static str] { &[\n";

    for i in &e.options {
        if i.flags.exclude {
            continue
        }

        let name = &i.name_rust_enum;
        let mut name_without_underscore = i.name_rust_field.clone();
        while name_without_underscore.starts_with("_") {
            name_without_underscore.remove(0);
        }

        write(q, format_args!("\"{name_without_underscore}\",\n")).unwrap();
        write(&mut get_value, format_args!("Self::{name} => \"{name_without_underscore}\",")).unwrap();
        write(&mut set_value, format_args!("\"{name_without_underscore}\" => {{ *self = Self::{name} }},")).unwrap();
    }

    *q += "] }\n";

    *q += "fn get_value(&self) -> &'static str {\n";
    *q += "match self {\n";
    *q += &get_value;
    *q += "}\n";
    *q += "}\n";

    *q += "fn set_value(&mut self, value: &str) -> Result<(), &'static str> {\n";
    *q += "match value {\n";
    *q += &set_value;
    *q += "_ => return Err(\"unknown value\")\n";
    *q += "}\n";
    *q += "Ok(())\n";
    *q += "}\n";
    *q += "}\n";
}

fn generate_bitfield(q: &mut String, b: &Bitfield) {
    let name = &b.name;

    write(q, format_args!("#[derive(Copy, Clone, Debug, PartialEq, Default)]\n")).unwrap();
    write(q, format_args!("pub struct {name} {{\n")).unwrap();
    for i in 0..b.width {
        let f = 1u32 << i;
        match b.fields.iter().find(|p| p.value == f) {
            Some(field) if !field.flags.exclude => {
                write(q, format_args!("pub {}: bool,", field.name_rust_field)).unwrap()
            },
            _ => (),
        }
    }
    *q += "}\n";

    let width = b.width;

    write(q, format_args!("impl SimpleWriteableData for {name} {{\n")).unwrap();
    *q += "#[inline]\n";
    write(q, format_args!("fn length() -> usize {{ {width} / 8 }}\n")).unwrap();

    *q += "fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {\n";
    write(q, format_args!("let raw_data = u{width}::read_tag_data_simple::<B>(from, parameters)?;\n")).unwrap();
    *q += "Ok(Self {\n";

    for field in &b.fields {
        if field.flags.exclude {
            continue
        }
        write(q, format_args!("{}: \n", field.name_rust_field)).unwrap();
        if field.flags.cache_only {
            *q += "parameters.cache_only_fields && "
        }
        else if field.flags.non_cached {
            *q += "parameters.tag_only_fields && "
        }
        write(q, format_args!("(raw_data & {}) != 0", field.value)).unwrap();
        *q += ",\n";
    }

    *q += "})\n";
    *q += "}\n";

    *q += "fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {\n";
    write(q, format_args!("let mut raw_data = 0u{width};\n")).unwrap();

    for field in &b.fields {
        if field.flags.exclude {
            continue
        }
        *q += "if ";
        if field.flags.cache_only {
            *q += "parameters.cache_only_fields && "
        }
        else if field.flags.non_cached {
            *q += "parameters.tag_only_fields && "
        }
        write(q, format_args!("self.{} {{ raw_data |= {} }}\n", field.name_rust_field, field.value)).unwrap();
    }

    *q += "raw_data.write_tag_data_simple::<B>(to, parameters)\n";
    *q += "}\n";
    *q += "}\n";
    write(q, format_args!("impl EditableTagField for {name} {{")).unwrap();
    *q += "#[inline] fn get_composite(&self) -> Option<&dyn EditableCompositeTagField> { Some(self) }\n";
    *q += "#[inline] fn get_composite_mut(&mut self) -> Option<&mut dyn EditableCompositeTagField> { Some(self) }\n";
    *q += "}\n";

    let mut get_field = String::with_capacity(1024 * 64);
    let mut get_field_mut = String::with_capacity(1024 * 64);
    let mut get_field_flags = String::with_capacity(1024 * 1024);

    write(q, format_args!("impl EditableCompositeTagField for {name} {{")).unwrap();
    *q += "#[inline]\n";
    *q += "fn fields(&self) -> &'static [&'static str] { &[\n";
    for f in &b.fields {
        if f.flags.exclude {
            continue
        }

        let name = &f.name_rust_field;
        let mut name_without_preceding_underscore = f.name_rust_field.clone();
        while name_without_preceding_underscore.starts_with("_") {
            name_without_preceding_underscore.remove(0);
        }

        write(q, format_args!("\"{name_without_preceding_underscore}\",")).unwrap();
        write(&mut get_field, format_args!("\"{name_without_preceding_underscore}\" => Some(&self.{name}),\n")).unwrap();
        write(&mut get_field_mut, format_args!("\"{name_without_preceding_underscore}\" => Some(&mut self.{name}),\n")).unwrap();

        let display_name = &f.name;
        let read_only = f.flags.uneditable_in_editor;
        write(&mut get_field_flags, format_args!("\"{name_without_preceding_underscore}\" => Some(EditableTagSubfieldFlags {{ display_name: \"{display_name}\", read_only: {read_only}, allowed_references: &[] }}),\n")).unwrap();
    }
    *q += "] }\n";

    *q += "fn get_field(&self, field: &str) -> Option<&dyn EditableTagField> {\n";
    *q += "match field {\n";
    *q += &get_field;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn EditableTagField> {\n";
    *q += "match field {\n";
    *q += &get_field_mut;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "fn get_field_flags(&self, field: &str) -> Option<EditableTagSubfieldFlags> {\n";
    *q += "match field {\n";
    *q += &get_field_flags;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "}\n";
}

fn generate_struct(q: &mut String, s: &Struct, definitions: &ParsedDefinitions) {
    let name = &s.name;

    *q += "#[derive(Clone, PartialEq, Debug, Default)]\n";
    if s.is_const {
        *q += "#[derive(Copy)]\n";
    }
    write(q, format_args!("pub struct {name} {{\n")).unwrap();
    for field in &s.fields {
        if field.flags.exclude {
            continue
        }

        match &field.field_type {
            StructFieldType::Object(object) => {
                let buffer;

                let value_name = match object {
                    FieldObject::NamedObject(n) => n.as_str(),
                    FieldObject::Reflexive(r) => {
                        buffer = format!("Reflexive<{r}>");
                        buffer.as_str()
                    },
                    FieldObject::TagReference { .. } => "TagReference",
                    FieldObject::TagGroup => "TagGroup",
                    FieldObject::Data => "Vec<u8>",
                    FieldObject::BSPVertexData => "Vec<u8>",
                    FieldObject::UTF16String => "String",
                    FieldObject::FileData => "Vec<u8>",
                    FieldObject::F32 => "f32",
                    FieldObject::U8 => "u8",
                    FieldObject::U16 => "u16",
                    FieldObject::U32 => "u32",
                    FieldObject::I8 => "i8",
                    FieldObject::I16 => "i16",
                    FieldObject::I32 => "i32",
                    FieldObject::TagID => "TagID",
                    FieldObject::ID => "u32",
                    FieldObject::Index => "Index",
                    FieldObject::Angle => "Angle",
                    FieldObject::Address => "Address",
                    FieldObject::Vector2D => "Vector2D",
                    FieldObject::Vector3D => "Vector3D",
                    FieldObject::CompressedVector2D => "CompressedVector2D",
                    FieldObject::CompressedVector3D => "CompressedVector3D",
                    FieldObject::CompressedFloat => "CompressedFloat",
                    FieldObject::Vector2DInt => "Vector2DInt",
                    FieldObject::Plane2D => "Plane2D",
                    FieldObject::Plane3D => "Plane3D",
                    FieldObject::Euler2D => "Euler2D",
                    FieldObject::Euler3D => "Euler3D",
                    FieldObject::Rectangle => "Rectangle",
                    FieldObject::Quaternion => "Quaternion",
                    FieldObject::Matrix2x3 => "Matrix2x3",
                    FieldObject::Matrix3x3 => "Matrix3x3",
                    FieldObject::ColorRGB => "ColorRGB",
                    FieldObject::ColorARGB => "ColorARGB",
                    FieldObject::Pixel32 => "Pixel32",
                    FieldObject::String32 => "String32",
                    FieldObject::ScenarioScriptNodeValue => "ScenarioScriptNodeValue",
                };

                write(q, format_args!("pub {}: ", field.name_rust_field)).unwrap();
                match field.count {
                    FieldCount::Bounds => write(q, format_args!("Bounds<{value_name}>")).unwrap(),
                    FieldCount::One => *q += value_name,
                    FieldCount::Array(l) => write(q, format_args!("[{value_name};{l}]")).unwrap()
                }

                *q += ",";
            },
            _ => continue
        }
    }
    *q += "}\n";

    if s.is_const {
        write(q, format_args!("impl SimpleWriteableData for {} {{\n", s.name)).unwrap();
        *q += "#[inline]\n";
        write(q, format_args!("fn length() -> usize {{ {} }}", s.size)).unwrap();

        let rw= generate_field_data(
            s,
            definitions,
            &[
                |read_data, field, offset_start, offset_end, endianness| {
                    let field_name = &field.name_rust_field;
                    write(read_data, format_args!("{field_name}: ")).unwrap();

                    let mut conditionally_read = false;
                    if field.flags.cache_only {
                        *read_data += "if parameters.cache_only_fields { ";
                        conditionally_read = true;
                    }
                    else if field.flags.non_cached {
                        *read_data += "if parameters.tag_only_fields { ";
                        conditionally_read = true;
                    }

                    write(read_data, format_args!("SimpleWriteableData::read_tag_data_simple::<{endianness}>(&from[{offset_start}..{offset_end}], parameters)?")).unwrap();

                    if conditionally_read {
                        *read_data += "} else { Default::default() }";
                    }
                    *read_data += ",";
                },
                |write_data, field, offset_start, offset_end, endianness| {
                    let field_name = &field.name_rust_field;

                    let mut conditionally_written = false;
                    if field.flags.cache_only {
                        *write_data += "if parameters.cache_only_fields { ";
                        conditionally_written = true;
                    }
                    else if field.flags.non_cached {
                        *write_data += "if parameters.tag_only_fields { ";
                        conditionally_written = true;
                    }

                    write(write_data, format_args!("self.{field_name}.write_tag_data_simple::<{endianness}>(&mut to[{offset_start}..{offset_end}], parameters);")).unwrap();

                    if conditionally_written {
                        *write_data += " }";
                    }
                }
            ]
        );

        let [read_data, write_data] = rw.as_slice() else { panic!("generate_struct passed/expected the wrong number of fns - is const") };

        *q += "fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {\n";
        *q += "Ok(Self {\n";
        *q += read_data;
        *q += "})\n";
        *q += "}\n";

        *q += "fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {\n";
        *q += write_data;
        *q += "}\n";

        *q += "}\n";
    }
    else {
        write(q, format_args!("impl WriteableData for {} {{\n", s.name)).unwrap();
        *q += "#[inline]\n";
        write(q, format_args!("fn base_length() -> usize {{ {} }}", s.size)).unwrap();

        let rw = generate_field_data(
            s,
            definitions,
            &[
                |read_data, field, offset_start, _offset_end, endianness| {
                    let field_name = &field.name_rust_field;
                    write(read_data, format_args!("{field_name}: ")).unwrap();

                    let conditionally_read = field.flags.cache_only || field.flags.non_cached;

                    if conditionally_read {
                        *read_data += "{ let value = ";
                    }

                    // for non-const, we still have to try to read the data (to advance the cursor) but then we discard the result
                    write(read_data, format_args!("WriteableData::read_tag_data::<{endianness}>(tag_data, offset + {offset_start}, cursor, parameters)?")).unwrap();

                    if conditionally_read {
                        *read_data += "; ";
                    }

                    if conditionally_read {
                        *read_data += "if ";
                        if field.flags.cache_only {
                            *read_data += "parameters.cache_only_fields";
                        }
                        else if field.flags.non_cached {
                            *read_data += "parameters.tag_only_fields";
                        }
                        *read_data += " { value } else { Default::default() } }";
                    }
                    *read_data += ",\n";
                },
                |write_data, field, offset_start, _offset_end, endianness| {
                    let field_name = &field.name_rust_field;

                    let mut conditionally_written = false;
                    if field.flags.cache_only {
                        *write_data += "if parameters.cache_only_fields { ";
                        conditionally_written = true;
                    }
                    else if field.flags.non_cached {
                        *write_data += "if parameters.tag_only_fields { ";
                        conditionally_written = true;
                    }

                    write(write_data, format_args!("self.{field_name}.write_tag_data::<{endianness}>(tag_data, offset + {offset_start}, parameters)?;")).unwrap();

                    if conditionally_written {
                        *write_data += " }";
                    }
                    *write_data += "\n";
                }
            ]
        );

        let [read_data, write_data] = rw.as_slice() else { panic!("generate_struct passed/expected the wrong number of fns - not const") };

        *q += "fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {\n";
        *q += "Ok(Self {\n";
        *q += read_data;
        *q += "})\n";
        *q += "}\n";

        *q += "fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {\n";
        *q += write_data;
        *q += "Ok(())\n";
        *q += "}\n";

        *q += "}\n";
    }

    write(q, format_args!("impl EditableTagField for {name} {{")).unwrap();
    *q += "#[inline] fn get_composite(&self) -> Option<&dyn EditableCompositeTagField> { Some(self) }\n";
    *q += "#[inline] fn get_composite_mut(&mut self) -> Option<&mut dyn EditableCompositeTagField> { Some(self) }\n";
    *q += "}\n";

    let mut get_field = String::with_capacity(1024 * 1024);
    let mut get_field_mut = String::with_capacity(1024 * 1024);
    let mut get_field_flags = String::with_capacity(1024 * 1024);

    write(q, format_args!("impl EditableCompositeTagField for {name} {{")).unwrap();
    *q += "#[inline]\n";
    *q += "fn fields(&self) -> &'static [&'static str] { &[\n";
    for f in &s.fields {
        if f.flags.exclude  {
            continue
        }

        let StructFieldType::Object(field_object) = &f.field_type else {
            continue
        };

        let name = &f.name_rust_field;
        let mut name_without_preceding_underscore = f.name_rust_field.clone();
        while name_without_preceding_underscore.starts_with("_") {
            name_without_preceding_underscore.remove(0);
        }

        let mut allowed_references = String::with_capacity(1024 * 1024);
        if let FieldObject::TagReference { allowed_groups } = &field_object {
            for i in allowed_groups {
                allowed_references += "TagGroup::";
                allowed_references += &definitions.groups.get(i).unwrap().name_rust_enum;
                allowed_references += ",";
            }
        }

        write(q, format_args!("\"{name_without_preceding_underscore}\",")).unwrap();
        write(&mut get_field, format_args!("\"{name_without_preceding_underscore}\" => Some(&self.{name}),\n")).unwrap();
        write(&mut get_field_mut, format_args!("\"{name_without_preceding_underscore}\" => Some(&mut self.{name}),\n")).unwrap();

        let display_name = &f.name;
        let read_only = f.flags.uneditable_in_editor;
        write(&mut get_field_flags, format_args!("\"{name_without_preceding_underscore}\" => Some(EditableTagSubfieldFlags {{ display_name: \"{display_name}\", read_only: {read_only}, allowed_references: &[{allowed_references}] }}),\n")).unwrap();
    }
    *q += "] }\n";

    *q += "fn get_field(&self, field: &str) -> Option<&dyn EditableTagField> {\n";
    *q += "match field {\n";
    *q += &get_field;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn EditableTagField> {\n";
    *q += "match field {\n";
    *q += &get_field_mut;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "fn get_field_flags(&self, field: &str) -> Option<EditableTagSubfieldFlags> {\n";
    *q += "match field {\n";
    *q += &get_field_flags;
    *q += "_ => None\n";
    *q += "}\n";
    *q += "}\n";

    *q += "}\n";

    for i in definitions.groups.values() {
        if &i.struct_name == name {
            write(q, format_args!("impl MainTagStruct for {name} {{")).unwrap();
            *q += "#[inline]\n";
            write(q, format_args!("fn tag_group() -> TagGroup {{ TagGroup::{} }}", i.name_rust_enum)).unwrap();
            *q += "}\n";
            write(q, format_args!("impl EditableTag for {name} {{")).unwrap();
            write(q, format_args!("#[inline] fn tag_group(&self) -> TagGroup {{ TagGroup::{} }}", i.name_rust_enum)).unwrap();
            *q += "#[inline] fn clone_to_boxed_tag(&self) -> Box<dyn EditableTag> { Box::new(self.clone()) }\n";
            *q += "#[inline] fn write_tag_to_vec(&self, parameters: Parameters) -> Result<Vec<u8>, WriteableDataError> { write_tag_file::<Self>(self, parameters) }\n";
            *q += "}\n";
            break
        }
    }


}

/// Returns (read_data, write_data)
fn generate_field_data(
    s: &Struct,
    definitions: &ParsedDefinitions,

    fns: &[IOFn],
) -> Vec<String> {
    let mut buffers = vec![String::with_capacity(1024 * 1024); fns.len()];

    for field in &s.fields {
        if field.flags.exclude {
            continue
        }
        if !matches!(field.field_type, StructFieldType::Object(_)) {
            continue
        }

        let size = field.size(definitions);
        let offset = field.relative_offset;
        let offset_end = field.relative_offset.checked_add(size).expect("relative_offset overflow");

        let endianness = if field.flags.little_endian_in_tags {
            "byteorder::LittleEndian"
        }
        else {
            "B"
        };

        for i in buffers.iter_mut().enumerate() {
            fns[i.0](i.1, field, offset, offset_end, endianness);
        }
    }

    buffers
}

type IOFn = fn(read_data: &mut String, field: &StructField, offset_start: usize, offset_end: usize, endianness: &str);

#[proc_macro]
pub fn generate_engine_defs(_: TokenStream) -> TokenStream {
    let definitions = load_all_definitions();

    let mut q = String::with_capacity(1024 * 1024 * 2);

    q += "pub const ALL_ENGINES: &'static [Engine] = &[\n";

    for i in definitions.engines.values() {
        q += "Engine {\n";
        writeln!(&mut q, "name: \"{}\",", i.name).unwrap();
        writeln!(&mut q, "display_name: \"{}\",", i.display_name).unwrap();
        writeln!(&mut q, "version: \"{}\",", i.version.as_ref().map(|i| i.as_str()).unwrap_or("")).unwrap();
        writeln!(&mut q, "build: {},", i.build.as_ref().map(|i| {
            let mut r = String::with_capacity(65536);
            r += "Some(EngineBuild {\n";

            writeln!(&mut r, "main: \"{}\",", i.string).unwrap();
            writeln!(&mut r, "enforced: {},", i.enforced).unwrap();

            r += "aliases: &[\n";
            for alias in &i.aliases {
                writeln!(&mut r, "\"{alias}\",").unwrap();
            }
            r += "],\n";

            r += "})";
            r
        }).unwrap_or("None".to_owned())).unwrap();
        writeln!(&mut q, "is_build_target: {},", i.build_target).unwrap();
        writeln!(&mut q, "is_fallback: {},", i.fallback).unwrap();
        writeln!(&mut q, "is_cache_default: {},", i.cache_default).unwrap();
        writeln!(&mut q, "is_custom: {},", i.custom).unwrap();
        writeln!(&mut q, "cache_file_version: {},", i.cache_file_version).unwrap();
        writeln!(&mut q, "data_alignment: {},", i.data_alignment).unwrap();
        writeln!(&mut q, "max_tag_space: {},", i.max_tag_space).unwrap();
        writeln!(&mut q, "max_script_nodes: {},", i.max_script_nodes).unwrap();
        writeln!(&mut q, "has_external_models: {},", i.external_models).unwrap();
        writeln!(&mut q, "allows_external_bsps: {},", i.external_bsps).unwrap();
        writeln!(&mut q, "uses_compressed_models: {},", i.compressed_models).unwrap();
        writeln!(&mut q, "has_obfuscated_header_layout: {},", i.obfuscated_header_layout).unwrap();

        q += "bitmaps: EngineBitmap {\n";
        writeln!(&mut q, "swizzled: {}\n,", i.bitmap_options.swizzled).unwrap();
        writeln!(&mut q, "texture_dimension_must_modulo_block_size: {}\n,", i.bitmap_options.texture_dimension_must_modulo_block_size).unwrap();
        writeln!(&mut q, "cubemap_faces_stored_separately: {}\n,", i.bitmap_options.cubemap_faces_stored_separately).unwrap();
        writeln!(&mut q, "alignment: {}\n,", i.bitmap_options.alignment).unwrap();
        q += "}\n,";

        q += "cache_file_size_limits: EngineCacheFileSizeLimits {\n";
        writeln!(&mut q, "user_interface: {}\n,", i.max_cache_file_size.user_interface).unwrap();
        writeln!(&mut q, "singleplayer: {}\n,", i.max_cache_file_size.singleplayer).unwrap();
        writeln!(&mut q, "multiplayer: {}\n,", i.max_cache_file_size.multiplayer).unwrap();
        q += "}\n,";

        q += "base_memory_address: EngineBaseMemoryAddress {\n";
        writeln!(&mut q, "address: {}\n,", i.base_memory_address.address).unwrap();
        writeln!(&mut q, "inferred: {}\n,", i.base_memory_address.inferred).unwrap();
        q += "}\n,";

        writeln!(&mut q, "compression_type: EngineCompressionType::{},\n", match i.compression_type {
            EngineCompressionType::Deflate => "Deflate",
            EngineCompressionType::Uncompressed => "Uncompressed"
        }).unwrap();

        writeln!(&mut q, "resource_maps: {},\n", i.resource_maps.as_ref().map(|i| {
            let mut r = String::with_capacity(65536);

            r += "Some(EngineSupportedResourceMaps {\n";
            writeln!(&mut r, "externally_indexed_tags: {},\n", i.externally_indexed_tags).unwrap();
            r += "})";

            r
        }).unwrap_or("None".to_owned())).unwrap();

        q += "required_tags: EngineRequiredTags {\n";

        fn do_the_thing(q: &mut String, name: &str, things: &[String]) {
            writeln!(q, "{name}: &[").unwrap();
            for i in things {
                writeln!(q, "\"{}\",", i.replace("\\", "\\\\")).unwrap();
            }
            *q += "],\n";
        }

        do_the_thing(&mut q, "all", &i.required_tags.all);
        do_the_thing(&mut q, "user_interface", &i.required_tags.user_interface);
        do_the_thing(&mut q, "multiplayer", &i.required_tags.multiplayer);
        do_the_thing(&mut q, "singleplayer", &i.required_tags.singleplayer);

        q += "},\n";

        q += "},\n";
    }

    q += "];\n";

    q.parse().expect("generate_engines parse fail")
}
