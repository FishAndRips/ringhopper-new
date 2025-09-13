use core::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, PartialEq, Default)]
#[repr(transparent)]
pub struct Address(pub u32);

impl Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{:08X}", self.0))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{:08X}", self.0))
    }
}
