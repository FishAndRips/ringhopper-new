use core::fmt::{Debug, Formatter};

#[derive(Copy, Clone, PartialEq, Default)]
#[repr(transparent)]
pub struct ScenarioScriptNodeValue {
    pub data: u32
}

impl Debug for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{}??", self.data))
    }
}
