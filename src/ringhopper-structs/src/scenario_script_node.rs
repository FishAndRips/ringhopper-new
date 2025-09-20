use core::fmt::{Debug, Formatter};
use funnel_web::id::ID;

/// Essentially a union.
///
/// Provides a safe interface to access its data as various types.
#[derive(Copy, Clone, PartialEq, Default)]
#[repr(transparent)]
pub struct ScenarioScriptNodeValue(pub u32);

impl ScenarioScriptNodeValue {
    /// Create the value from a float (real).
    pub const fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }

    /// Create the value from a 32-bit signed integer (long).
    pub const fn from_i32(value: i32) -> Self {
        Self(value as u32)
    }

    /// Create the value from a 16-bit signed integer (short).
    pub const fn from_i16(value: i16) -> Self {
        Self((value as u32) | 0xFFFF0000)
    }

    /// Create the value from a boolean.
    pub const fn from_bool(value: bool) -> Self {
        Self((value as u32) | 0xFFFFFF00)
    }

    /// Create the value from an ID.
    pub const fn from_id<const SALT: u16>(id: ID<SALT>) -> Self {
        Self(id.as_u32())
    }

    /// Get the value as a float (real).
    pub const fn as_f32(&self) -> f32 {
        f32::from_bits(self.0)
    }

    /// Get the value as a 32-bit signed integer (long).
    pub const fn as_i32(&self) -> i32 {
        self.0 as i32
    }

    /// Get the value as a 16-bit signed integer (short).
    pub const fn as_i16(&self) -> i16 {
        (self.0 & 0xFFFF) as i16
    }

    /// Get the value as a boolean.
    ///
    /// Returns `None` if the lowest 8 bits do not correspond to `0x00` or `0x01`.
    pub const fn as_bool(&self) -> Option<bool> {
        match self.0 & 0xFF {
            0 => Some(false),
            1 => Some(true),
            _ => None
        }
    }

    /// Get the value as an ID.
    ///
    /// Returns `None` if the ID is not valid.
    pub const fn as_id<const SALT: u16>(&self) -> Option<ID<SALT>> {
        ID::<SALT>::from_u32(self.0)
    }
}

impl Debug for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{}??", self.0))
    }
}

impl PartialEq<f32> for ScenarioScriptNodeValue {
    fn eq(&self, other: &f32) -> bool {
        self.as_f32() == *other
    }
}

impl PartialEq<ScenarioScriptNodeValue> for f32 {
    fn eq(&self, other: &ScenarioScriptNodeValue) -> bool {
        *self == other.as_f32()
    }
}

impl PartialEq<i32> for ScenarioScriptNodeValue {
    fn eq(&self, other: &i32) -> bool {
        self.as_i32() == *other
    }
}

impl PartialEq<ScenarioScriptNodeValue> for i32 {
    fn eq(&self, other: &ScenarioScriptNodeValue) -> bool {
        *self == other.as_i32()
    }
}

impl PartialEq<i16> for ScenarioScriptNodeValue {
    fn eq(&self, other: &i16) -> bool {
        self.as_i16() == *other
    }
}

impl PartialEq<ScenarioScriptNodeValue> for i16 {
    fn eq(&self, other: &ScenarioScriptNodeValue) -> bool {
        *self == other.as_i16()
    }
}

impl PartialEq<bool> for ScenarioScriptNodeValue {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}

impl PartialEq<ScenarioScriptNodeValue> for bool {
    fn eq(&self, other: &ScenarioScriptNodeValue) -> bool {
        Some(*self) == other.as_bool()
    }
}

impl<const SALT: u16> PartialEq<ID<SALT>> for ScenarioScriptNodeValue {
    fn eq(&self, other: &ID<SALT>) -> bool {
        self.as_id() == Some(*other)
    }
}

impl<const SALT: u16> PartialEq<ScenarioScriptNodeValue> for ID<SALT> {
    fn eq(&self, other: &ScenarioScriptNodeValue) -> bool {
        Some(*self) == other.as_id()
    }
}
