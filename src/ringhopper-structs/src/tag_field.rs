//! TODO: This is not complete!

use core::fmt::{Display, Formatter};
use funnel_web::color::{ColorARGB, ColorRGB, Pixel32};
use funnel_web::id::{Index, ID};
use funnel_web::rectangle::Rectangle;
use funnel_web::string::ASCIIString;
use funnel_web::vector::*;
use crate::{Address, Reflexive, TagReference, WriteableData};

pub trait EditableTagField: 'static + core::any::Any {
    fn get_composite(&self) -> Option<&dyn EditableCompositeTagField> {
        None
    }
    fn get_composite_mut(&mut self) -> Option<&mut dyn EditableCompositeTagField> {
        None
    }

    fn get_indexed(&self) -> Option<&dyn EditableIndexedTagField> {
        None
    }
    fn get_indexed_mut(&mut self) -> Option<&mut dyn EditableIndexedTagField> {
        None
    }

    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        None
    }
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        None
    }
}

pub trait EditableTagFieldData: EditableTagField {
    fn get_value(&self) -> TagFieldDataValue<'_>;
    fn set_value(&mut self, value: &str) -> Result<(), &'static str>;
}

pub trait EditableCompositeTagField: EditableTagField {
    fn fields(&self) -> &'static [&'static str];
    fn get_field(&self, field: &str) -> Option<&dyn EditableTagField>;
    fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn EditableTagField>;
}

pub trait EditableIndexedTagField: EditableTagField {
    fn item_count(&self) -> usize;
    fn get_item(&self, item: usize) -> Option<&dyn EditableTagField>;
    fn get_item_mut(&mut self, item: usize) -> Option<&mut dyn EditableTagField>;
    fn remove_item(&mut self, item: usize) -> Result<(), &'static str>;
    fn swap_items(&mut self, a: usize, b: usize) -> Result<(), &'static str>;
    fn add_item(&mut self, at: usize) -> Result<(), &'static str>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TagFieldDataValue<'a> {
    Bool(bool),
    Float(f32),
    String(&'a str),
    Integer(i64)
}

impl<'a> Display for TagFieldDataValue<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bool(q) => Display::fmt(q, f),
            Self::Float(q) => Display::fmt(q, f),
            Self::String(q) => Display::fmt(q, f),
            Self::Integer(q) => Display::fmt(q, f),
        }
    }
}

impl EditableTagField for bool {
    #[inline]
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
    #[inline]
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        Some(self)
    }
}

impl EditableTagFieldData for bool {
    #[inline]
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::Bool(*self)
    }
    #[inline]
    fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
        *self = match value {
            "0" | "false" | "FALSE" | "no" | "NO" => false,
            "1" | "true" | "TRUE" | "yes" | "YES" => true,
            _ => return Err("cannot parse boolean")
        };
        Ok(())
    }
}

macro_rules! define_editable_tag_field_parseable {
    ($t:ty, $err:literal) => {
        impl EditableTagField for $t {
            #[inline]
            fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
                Some(self)
            }
            #[inline]
            fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
                Some(self)
            }
        }
        impl EditableTagFieldData for $t {
            #[inline]
            fn get_value(&self) -> TagFieldDataValue<'_> {
                TagFieldDataValue::Integer((*self).into())
            }
            #[inline]
            fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
                value.parse::<$t>().map_err(|_| $err)?;
                Ok(())
            }
        }
    };
}

define_editable_tag_field_parseable!(u8, "can't parse into u8");
define_editable_tag_field_parseable!(u16, "can't parse into u16");
define_editable_tag_field_parseable!(u32, "can't parse into u32");
define_editable_tag_field_parseable!(i8, "can't parse into i8");
define_editable_tag_field_parseable!(i16, "can't parse into i16");
define_editable_tag_field_parseable!(i32, "can't parse into i32");


impl EditableTagField for f32 {
    #[inline]
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
    #[inline]
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        Some(self)
    }
}
impl EditableTagFieldData for f32 {
    #[inline]
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::Float(*self)
    }
    #[inline]
    fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
        let float = value.parse::<f32>()
            .map_err(|_| "invalid float (can't be parsed)")?;

        if float.is_nan() {
            return Err("invalid float (NaN)")
        }

        *self = float;

        Ok(())
    }
}

impl<const LEN: usize> EditableTagField for ASCIIString<LEN> {
    #[inline]
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
    #[inline]
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        Some(self)
    }
}

impl<const LEN: usize> EditableTagFieldData for ASCIIString<LEN> {
    #[inline]
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::String(self.as_str())
    }
    #[inline]
    fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
        *self = Self::from_str(value).ok_or("invalid string")?;
        Ok(())
    }
}

macro_rules! composite_tag_fields {
    ($type:ty, $($vals:tt), *) => {
        impl EditableTagField for $type {
            #[inline]
            fn get_composite(&self) -> Option<&dyn EditableCompositeTagField> {
                Some(self)
            }
            #[inline]
            fn get_composite_mut(&mut self) -> Option<&mut dyn EditableCompositeTagField> {
                Some(self)
            }
        }
        impl EditableCompositeTagField for $type {
            fn fields(&self) -> &'static [&'static str] {
                &[$(stringify!($vals)),*]
            }
            fn get_field(&self, field: &str) -> Option<&dyn EditableTagField> {
                match field {
                    $(stringify!($vals) => Some(&self.$vals),)*
                    _ => None
                }
            }
            fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn EditableTagField> {
                match field {
                    $(stringify!($vals) => Some(&mut self.$vals),)*
                    _ => None
                }
            }
        }
    };
}

composite_tag_fields!(Vector2D, x, y);
composite_tag_fields!(Vector3D, x, y, z);
composite_tag_fields!(Vector4D, x, y, z, w);
composite_tag_fields!(Quaternion, x, y, z, w);
composite_tag_fields!(Euler2D, yaw, pitch);
composite_tag_fields!(Euler3D, yaw, pitch, roll);
composite_tag_fields!(Plane2D, vector, offset);
composite_tag_fields!(Plane3D, vector, offset);
composite_tag_fields!(Matrix2x3, forward, up);
composite_tag_fields!(Matrix3x3, forward, left, up);
composite_tag_fields!(Matrix4x3, scale, rotation, position);
composite_tag_fields!(Rectangle, top, left, bottom, right);
composite_tag_fields!(ColorRGB, r, g, b);
composite_tag_fields!(ColorARGB, a, color);
composite_tag_fields!(Vector2DInt, x, y);

impl<const SALT: u16> EditableTagField for ID<SALT> {}

impl EditableTagField for TagReference {}
impl<T: EditableTagField + WriteableData + Default> EditableTagField for Reflexive<T> {
    fn get_indexed(&self) -> Option<&dyn EditableIndexedTagField> {
        Some(self)
    }
    fn get_indexed_mut(&mut self) -> Option<&mut dyn EditableIndexedTagField> {
        Some(self)
    }
}
impl<T: EditableTagField + WriteableData + Default> EditableIndexedTagField for Reflexive<T> {
    fn item_count(&self) -> usize {
        self.len()
    }
    fn get_item(&self, item: usize) -> Option<&dyn EditableTagField> {
        match self.get(item) {
            Some(i) => Some(i),
            None => None
        }
    }
    fn get_item_mut(&mut self, item: usize) -> Option<&mut dyn EditableTagField> {
        match self.get_mut(item) {
            Some(i) => Some(i),
            None => None
        }
    }
    fn remove_item(&mut self, item: usize) -> Result<(), &'static str> {
        if item >= self.len() {
            return Err("out of bounds index")
        }
        self.remove(item);
        Ok(())
    }
    fn swap_items(&mut self, a: usize, b: usize) -> Result<(), &'static str> {
        if a.max(b) >= self.len() {
            return Err("out of bounds index(s)")
        }
        self.swap(a, b);
        Ok(())
    }
    fn add_item(&mut self, at: usize) -> Result<(), &'static str> {
        if at > self.len() {
            return Err("out of bounds index")
        };
        self.insert(at, T::default());
        Ok(())
    }
}

impl EditableTagField for CompressedFloat {}
impl EditableTagField for CompressedVector2D {}
impl EditableTagField for CompressedVector3D {}

impl EditableTagField for Pixel32 {}
impl EditableTagField for Angle {
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        Some(self)
    }
}
impl EditableTagFieldData for Angle {
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::Float(self.0.to_degrees())
    }
    fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
        let mut degrees = false;

        let float = if let Some(r) = value.strip_suffix("r") {
            r
        }
        else if let Some(d) = value.strip_suffix("d") {
            degrees = true;
            d
        }
        else {
            value
        }.parse::<f32>().map_err(|_| "invalid angle float (can't parse)")?;

        if float.is_nan() {
            return Err("invalid angle float (NaN)")
        }

        if degrees {
            *self = Self::from_degrees(float)
        }
        else {
            *self = Self::from_radians(float)
        }

        Ok(())
    }
}

impl EditableTagField for Index {
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
    fn get_field_data_mut(&mut self) -> Option<&mut dyn EditableTagFieldData> {
        Some(self)
    }
}
impl EditableTagFieldData for Index {
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::Integer(self.0 as i64)
    }
    fn set_value(&mut self, value: &str) -> Result<(), &'static str> {
        *self = match value {
            "null" | "NULL" | "-1" | "none" | "NONE" => Self::new(),
            n => Index(n.parse::<u16>().map_err(|_| "invalid index value")?)
        };
        Ok(())
    }
}

impl EditableTagField for Address {
    fn get_field_data(&self) -> Option<&dyn EditableTagFieldData> {
        Some(self)
    }
}
impl EditableTagFieldData for Address {
    fn get_value(&self) -> TagFieldDataValue<'_> {
        TagFieldDataValue::Integer(self.0 as i64)
    }
    fn set_value(&mut self, _value: &str) -> Result<(), &'static str> {
        unreachable!("addresses are read-only with this interface")
    }
}

#[cfg(test)]
mod test {
    use funnel_web::string::String32;
    use crate::{EditableTagField, Reflexive};
    use alloc::string::ToString;
    use funnel_web::vector::Vector3D;

    #[test]
    fn editable_string() {
        let mut string = String32::from_str("test").unwrap();
        let string_field_data = string.get_field_data_mut().unwrap();
        assert_eq!(string_field_data.get_value().to_string(), "test");
        string_field_data.set_value("this is a test string").unwrap();
        assert_eq!(string_field_data.get_value().to_string(), "this is a test string");
        assert!(string_field_data.set_value("this value is way way way way way way way way way way way way way too long").is_err());
        assert_eq!(string_field_data.get_value().to_string(), "this is a test string");
    }

    #[test]
    fn editable_bool() {
        let mut bool = false;
        let bool_field_data = bool.get_field_data_mut().unwrap();
        assert_eq!(bool_field_data.get_value().to_string(), "false");
        bool_field_data.set_value("1").unwrap();
        assert_eq!(bool_field_data.get_value().to_string(), "true");
        bool_field_data.set_value("false").unwrap();
        assert_eq!(bool_field_data.get_value().to_string(), "false");
        bool_field_data.set_value("true").unwrap();
        assert_eq!(bool_field_data.get_value().to_string(), "true");
        assert!(bool);
    }

    #[test]
    fn editable_composite() {
        let mut v: Vector3D = Vector3D {
            x: 0.0,
            y: 1.0,
            z: 2.0
        };

        let v_c = v.get_composite_mut().unwrap();
        v_c.get_field_mut("x").unwrap().get_field_data_mut().unwrap().set_value("1337").unwrap();
        v_c.get_field_mut("y").unwrap().get_field_data_mut().unwrap().set_value("1234").unwrap();
        v_c.get_field_mut("z").unwrap().get_field_data_mut().unwrap().set_value("9001").unwrap();

        // NaN is banned and does not change the final result
        assert!(v_c.get_field_mut("z").unwrap().get_field_data_mut().unwrap().set_value("NaN").is_err());

        assert_eq!(v, Vector3D { x: 1337.0, y: 1234.0, z: 9001.0 });
    }

    #[test]
    fn editable_indexed() {
        let mut r: Reflexive<Vector3D> = Reflexive::new();

        let r_c = r.get_indexed_mut().unwrap();
        for i in 0..64 {
            r_c.add_item(i).unwrap();
        }

        for i in 0..r_c.item_count() {
            let offset = i * 3;
            let field = r_c.get_item_mut(i).unwrap().get_composite_mut().unwrap();
            for j in field.fields().iter().enumerate() {
                let offset = offset + j.0;
                field.get_field_mut(j.1).unwrap().get_field_data_mut().unwrap().set_value(&offset.to_string()).unwrap();
            }
        }

        r_c.swap_items(22, 63).unwrap();

        assert_eq!(r[63].x, 66.0);
        assert_eq!(r[63].y, 67.0);
        assert_eq!(r[63].z, 68.0);

        assert_eq!(r[22].x, 189.0);
        assert_eq!(r[22].y, 190.0);
        assert_eq!(r[22].z, 191.0);

        assert_eq!(r.get(64), None);
    }
}
