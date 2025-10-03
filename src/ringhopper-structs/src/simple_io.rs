use byteorder::ByteOrder;
use funnel_web::color::{ColorARGB, ColorRGB, Pixel32};
use funnel_web::id::{Index, TagID, ID};
use funnel_web::rectangle::Rectangle;
use funnel_web::string::ASCIIString;
use funnel_web::vector::*;
use crate::{Address, Bounds, ScenarioScriptNodeValue};
use crate::definitions::tag::TagGroup;

/// WriteableData for simple primitives.
pub trait SimpleWriteableData: Copy + Clone + Sized {
    /// Read data.
    ///
    /// `from.len()` can be assumed to be equal to `Self::length()`
    ///
    /// Returns `Err()` with an explanation if invalid.
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str>;

    /// Write data.
    ///
    /// `to.len()` can be assumed to be equal to `Self::length()`
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters);

    /// Length of the data in bytes.
    #[must_use]
    fn length() -> usize;
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum Strictness {
    Relaxed,
    Strict
}

/// Trust me, bro.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ForceBaseMemoryAddress {
    ForceInferred,
    ForceFixed
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Parameters {
    pub strictness: Strictness,
    pub base_memory_address: Option<ForceBaseMemoryAddress>,
    pub cache_only_fields: bool,
    pub tag_only_fields: bool
}

impl Parameters {
    /// Default parameters for reading from tag files
    pub const TAG_FILES: Parameters = Parameters {
        strictness: Strictness::Strict,
        tag_only_fields: true,
        cache_only_fields: false,
        base_memory_address: None
    };

    /// Default parameters for reading from cache files
    pub const CACHE_FILES: Parameters = Parameters {
        strictness: Strictness::Strict,
        tag_only_fields: false,
        cache_only_fields: true,
        base_memory_address: None
    };
}

macro_rules! byte_io {
    ($t:ty) => {
        impl SimpleWriteableData for $t {
            #[inline]
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], _parameters: Parameters) -> Result<Self, &'static str> {
                Ok(from[0] as $t)
            }
            #[inline]
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], _parameters: Parameters) {
                to[0] = *self as u8;
            }
            #[inline]
            fn length() -> usize {
                1
            }
        }

    };
}

byte_io!(u8);
byte_io!(i8);

macro_rules! long_io {
    ($t:ty, $read:tt, $write:tt) => {
        impl SimpleWriteableData for $t {
            #[inline]
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], _parameters: Parameters) -> Result<Self, &'static str> {
                Ok(B::$read(from))
            }
            #[inline]
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], _parameters: Parameters) {
                B::$write(to, *self)
            }
            #[inline]
            fn length() -> usize {
                size_of::<Self>()
            }
        }
    };
}

long_io!(u16, read_u16, write_u16);
long_io!(i16, read_i16, write_i16);
long_io!(u32, read_u32, write_u32);
long_io!(i32, read_i32, write_i32);
long_io!(f32, read_f32, write_f32);

impl SimpleWriteableData for TagGroup {
    #[inline]
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        let potential_fourcc = u32::read_tag_data_simple::<B>(from, parameters)?;
        Self::from_u32(potential_fourcc)
            .ok_or("unrecognized tag group fourcc")
    }
    #[inline]
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.as_u32().write_tag_data_simple::<B>(to, parameters);
    }
    #[inline]
    fn length() -> usize {
        4
    }
}

macro_rules! io_ordered_primitive {
    ($type:ty, $len:expr, $($vals:tt), *) => {
        impl SimpleWriteableData for $type {
            #[allow(unused)]
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
                let mut current_primitive_offset = 0usize;

                // SAFETY: Everything that uses io_ordered_primitive!() is valid to be zeroed out,
                // and we need an existing struct for size_of_val to work.
                let mut result: Self = unsafe { core::mem::zeroed() };

                $(
                    let start = current_primitive_offset;
                    let end = current_primitive_offset + size_of_val(&result.$vals);
                    result.$vals = SimpleWriteableData::read_tag_data_simple::<B>(&from[start..end], parameters)?;
                    current_primitive_offset = end;
                )*

                Ok(result)
            }
            #[allow(unused)]
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
                let mut current_primitive_offset = 0usize;

                $(
                    let start = current_primitive_offset;
                    let end = current_primitive_offset + size_of_val(&self.$vals);
                    self.$vals.write_tag_data_simple::<B>(&mut to[start..end], parameters);
                    current_primitive_offset = end;
                )*
            }
            #[inline]
            fn length() -> usize {
                $len
            }
        }
    };
}

io_ordered_primitive!(Vector2D, 4*2, x, y);
io_ordered_primitive!(Vector3D, 4*3, x, y, z);
io_ordered_primitive!(Vector4D, 4*4, x, y, z, w);
io_ordered_primitive!(Quaternion, 4*4, x, y, z, w);
io_ordered_primitive!(Euler2D, 4*2, yaw, pitch);
io_ordered_primitive!(Euler3D, 4*3, yaw, pitch, roll);
io_ordered_primitive!(Plane2D, 4*2+4, vector, offset);
io_ordered_primitive!(Plane3D, 4*3+4, vector, offset);
io_ordered_primitive!(Angle, 4, 0);
io_ordered_primitive!(CompressedFloat, 2, 0);
io_ordered_primitive!(CompressedVector2D, 4, 0);
io_ordered_primitive!(CompressedVector3D, 4, 0);
io_ordered_primitive!(Matrix2x3, Vector3D::length() * 2, forward, up);
io_ordered_primitive!(Matrix3x3, Vector3D::length() * 3, forward, left, up);
io_ordered_primitive!(Matrix4x3, Vector3D::length() * (3 + 1) + 4, scale, rotation, position);
io_ordered_primitive!(Rectangle, 2*4, top, left, bottom, right);
io_ordered_primitive!(ColorRGB, 4*3, r, g, b);
io_ordered_primitive!(ColorARGB, 4*4, a, color);
io_ordered_primitive!(Vector2DInt, 4, x, y);
io_ordered_primitive!(Pixel32, 4, 0);
io_ordered_primitive!(Index, 2, 0);
io_ordered_primitive!(Address, 4, 0);

impl SimpleWriteableData for ScenarioScriptNodeValue {
    #[inline]
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        Ok(Self(u32::read_tag_data_simple::<B>(from, parameters)?))
    }
    #[inline]
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.0.write_tag_data_simple::<B>(to, parameters)
    }
    #[inline]
    fn length() -> usize {
        4
    }
}

impl<const SALT: u16> SimpleWriteableData for ID<SALT> {
    #[inline]
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        let u32_data = u32::read_tag_data_simple::<B>(from, parameters)?;
        Ok(Self::from_u32(u32_data))
    }
    #[inline]
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.as_u32().write_tag_data_simple::<B>(to, parameters)
    }
    #[inline]
    fn length() -> usize {
        4
    }
}

impl<T: SimpleWriteableData + Sized, const LEN: usize> SimpleWriteableData for [T; LEN] {
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        // Using MaybeUninit to prevent needing a Default requirement.
        let mut data: [core::mem::MaybeUninit<T>; LEN] = [const { core::mem::MaybeUninit::uninit() }; LEN];

        for (chunk, data) in from.chunks(T::length()).zip(data.iter_mut()) {
            match T::read_tag_data_simple::<B>(chunk, parameters) {
                Ok(n) => { data.write(n); },
                Err(e) => {
                    // Since SimpleWriteableData requires Copy, we don't need to bother dropping
                    // anything since nothing that implements Copy can implement Drop
                    return Err(e);
                }
            }
        }

        // SAFETY: We've successfully initialized everything above
        //
        // We are using transmute_copy instead of transmute because the compiler cannot, in this
        // context, determine that [MaybeUninit<T>; LEN] is the same size as [T; LEN]. However,
        // MaybeUninit<T> is the same length, alignment, and ABI as T, thus arrays of the same
        // length should be the same exact length.
        let result: [T; LEN] = unsafe {
            core::mem::transmute_copy::<[core::mem::MaybeUninit<T>; LEN], [T; LEN]>(&data)
        };

        Ok(result)
    }

    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        for (chunk, data) in to.chunks_mut(T::length()).zip(self.iter()) {
            data.write_tag_data_simple::<B>(chunk, parameters);
        }
    }

    #[inline]
    fn length() -> usize {
        T::length() * LEN
    }
}

impl<const LEN: usize> SimpleWriteableData for ASCIIString<LEN> {
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], _parameters: Parameters) -> Result<Self, &'static str> {
        let string: [u8; LEN] = from.try_into().unwrap();
        Self::from_bytes(string).ok_or("invalid ASCIIString")
    }

    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], _parameters: Parameters) {
        to.copy_from_slice(self.bytes())
    }

    #[inline]
    fn length() -> usize {
        LEN
    }
}

impl<T: SimpleWriteableData + Sized> SimpleWriteableData for Bounds<T> {
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        let (from, to) = from.split_at(T::length());

        Ok(Self {
            from: T::read_tag_data_simple::<B>(from, parameters)?,
            to: T::read_tag_data_simple::<B>(to, parameters)?
        })
    }
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        let (from, to) = to.split_at_mut(T::length());

        self.from.write_tag_data_simple::<B>(from, parameters);
        self.to.write_tag_data_simple::<B>(to, parameters);
    }
    #[inline]
    fn length() -> usize {
        T::length() * 2
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct TagReferenceC {
    pub group: TagGroup,
    pub path_pointer: Address,
    pub path_size: u32,
    pub tag_id: TagID
}

io_ordered_primitive!(TagReferenceC, 0x10, group, path_pointer, path_size, tag_id);


#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct ReflexiveC {
    pub count: u32,
    pub address: Address,
    pub unused: u32
}

io_ordered_primitive!(ReflexiveC, 0xC, count, address, unused);


#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct TagDataC {
    pub length: u32,
    pub flags: u32,
    pub file_offset: u32,
    pub data: Address,
    pub unused: u32
}

io_ordered_primitive!(TagDataC, 0x14, length, flags, file_offset, data, unused);
