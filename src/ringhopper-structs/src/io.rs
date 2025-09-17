use byteorder::ByteOrder;
use funnel_web::color::{ColorARGB, ColorRGB, Pixel32};
use funnel_web::id::{Index, ID};
use funnel_web::rectangle::Rectangle;
use funnel_web::string::ASCIIString;
use funnel_web::vector::*;
use crate::{Address, ScenarioScriptNodeValue};
use crate::definitions::TagGroup;

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

    /// Length of the data.
    ///
    /// This is implemented as `size_of::<Self>()`. If overridden, the value should never change
    /// between calls and should always be the same for this type.
    fn length() -> usize {
        size_of::<Self>()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Strictness {
    Relaxed,
    Strict
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Parameters {
    pub strictness: Strictness,
    pub cache_only_fields: bool,
    pub tag_only_fields: bool
}

macro_rules! byte_io {
    ($t:ty) => {
        impl SimpleWriteableData for $t {
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], _parameters: Parameters) -> Result<Self, &'static str> {
                Ok(from[0] as $t)
            }
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], _parameters: Parameters) {
                to[0] = *self as u8;
            }
        }

    };
}

byte_io!(u8);
byte_io!(i8);

macro_rules! long_io {
    ($t:ty, $read:tt, $write:tt) => {
        impl SimpleWriteableData for $t {
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], _parameters: Parameters) -> Result<Self, &'static str> {
                Ok(B::$read(from))
            }
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], _parameters: Parameters) {
                B::$write(to, *self)
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
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        let potential_fourcc = u32::read_tag_data_simple::<B>(from, parameters)?;
        Self::from_u32(potential_fourcc)
            .ok_or("unrecognized tag group fourcc")
    }
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.as_u32().write_tag_data_simple::<B>(to, parameters);
    }
}

macro_rules! io_ordered_primitive_write {
    ($self:expr, $parameters:expr, $to:expr, $c:tt, $q:tt) => {
        let start = $c;
        let end = $c + size_of_val(&$self.$q);
        $self.$q.write_tag_data_simple::<B>(&mut $to[start..end], $parameters);
        $c = end;
    };

    ($self:expr, $parameters:expr, $to:expr, $c:tt, $q:tt, $($vals:tt), +) => {
        io_ordered_primitive_write!($self, $parameters, $to, $c, $q);
        io_ordered_primitive_write!($self, $parameters, $to, $c, $($vals), +);
    };
}

macro_rules! io_ordered_primitive_read {
    ($self:expr, $parameters:expr, $from:expr, $c:tt, $q:tt) => {
        let start = $c;
        let end = $c + size_of_val(&$self.$q);
        $self.$q = SimpleWriteableData::read_tag_data_simple::<B>(&$from[start..end], $parameters)?;
        $c = end;
    };

    ($self:expr, $parameters:expr, $from:expr, $c:tt, $q:tt, $($vals:tt), +) => {
        io_ordered_primitive_read!($self, $parameters, $from, $c, $q);
        io_ordered_primitive_read!($self, $parameters, $from, $c, $($vals), +);
    };
}

macro_rules! io_ordered_primitive {
    ($type:ty, $($vals:tt), *) => {
        impl SimpleWriteableData for $type {
            #[allow(unused)]
            fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
                let mut current_primitive_offset = 0usize;
                // SAFETY: Everything that uses io_ordered_primitive!() is valid to be zeroed out
                let mut result: Self = unsafe { core::mem::zeroed() };
                io_ordered_primitive_read!(result, parameters, from, current_primitive_offset, $($vals), *);
                Ok(result)
            }
            #[allow(unused)]
            fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
                let mut current_primitive_offset = 0usize;
                io_ordered_primitive_write!(self, parameters, to, current_primitive_offset, $($vals), *);
            }
        }
    };
}

io_ordered_primitive!(Vector2D, x, y);
io_ordered_primitive!(Vector3D, x, y, z);
io_ordered_primitive!(Vector4D, x, y, z, w);
io_ordered_primitive!(Quaternion, x, y, z, w);
io_ordered_primitive!(Euler2D, yaw, pitch);
io_ordered_primitive!(Euler3D, yaw, pitch, roll);
io_ordered_primitive!(Plane2D, vector, offset);
io_ordered_primitive!(Plane3D, vector, offset);
io_ordered_primitive!(Angle, 0);
io_ordered_primitive!(CompressedFloat, 0);
io_ordered_primitive!(CompressedVector2D, 0);
io_ordered_primitive!(CompressedVector3D, 0);
io_ordered_primitive!(Matrix2x3, forward, up);
io_ordered_primitive!(Matrix3x3, forward, left, up);
io_ordered_primitive!(Matrix4x3, scale, rotation, position);
io_ordered_primitive!(Rectangle, top, left, bottom, right);
io_ordered_primitive!(ColorRGB, r, g, b);
io_ordered_primitive!(ColorARGB, a, color);
io_ordered_primitive!(Vector2DInt, x, y);
io_ordered_primitive!(Pixel32, 0);
io_ordered_primitive!(Index, 0);
io_ordered_primitive!(Address, 0);

impl SimpleWriteableData for ScenarioScriptNodeValue {
    fn length() -> usize {
        4
    }
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        Ok(Self {
            data: u32::read_tag_data_simple::<B>(from, parameters)?
        })
    }
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.data.write_tag_data_simple::<B>(to, parameters)
    }
}

impl<const SALT: u16> SimpleWriteableData for ID<SALT> {
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        Self::from_u32(u32::read_tag_data_simple::<B>(from, parameters)?).ok_or("invalid id data")
    }
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.as_u32().write_tag_data_simple::<B>(to, parameters)
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

    fn length() -> usize {
        LEN
    }
}
