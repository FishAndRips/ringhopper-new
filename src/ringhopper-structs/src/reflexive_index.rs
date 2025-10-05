use core::cmp::Ordering;
use byteorder::ByteOrder;
use funnel_web::id::Index;
use crate::{EditableReflexiveIndex, EditableTagField, Parameters, SimpleWriteableData};

/// Defines an index that points to a reflexive.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(transparent)]
pub struct ReflexiveIndex<const TAG: usize>(pub Index);

impl<const TAG: usize> ReflexiveIndex<TAG> {
    /// Get the reflexive name.
    #[inline]
    #[must_use]
    pub const fn get_reflexive_name() -> &'static str {
        super::definitions::tag::REFLEXIVE_INDICES[TAG].0
    }

    /// Get the name of the struct the reflexive is in.
    #[inline]
    #[must_use]
    pub const fn get_reflexive_struct() -> &'static str {
        super::definitions::tag::REFLEXIVE_INDICES[TAG].1
    }

    /// Get the index value.
    #[inline]
    #[must_use]
    pub const fn index(self) -> Option<usize> {
        self.0.index()
    }

    /// Return true if the index is null.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Instantiate an empty index.
    #[inline]
    #[must_use]
    pub const fn new() -> ReflexiveIndex<TAG> {
        Self(Index::new())
    }
}

impl<const TAG: usize> From<Index> for ReflexiveIndex<TAG> {
    fn from(value: Index) -> Self {
        Self(value)
    }
}

impl<const TAG: usize> EditableReflexiveIndex for ReflexiveIndex<TAG> {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.0
    }
    #[inline]
    fn get_index_mut(&mut self) -> &mut Index {
        &mut self.0
    }
    #[inline]
    fn get_reflexive_name(&self) -> &'static str {
        Self::get_reflexive_name()
    }
    #[inline]
    fn get_reflexive_struct(&self) -> &'static str {
        Self::get_reflexive_struct()
    }
}

impl<const TAG: usize> EditableTagField for ReflexiveIndex<TAG> {
    #[inline]
    fn get_field_type_name(&self) -> &'static str {
        "ReflexiveIndex"
    }
    fn get_reflexive_index(&self) -> Option<&dyn EditableReflexiveIndex> {
        Some(self)
    }
    fn get_reflexive_index_mut(&mut self) -> Option<&mut dyn EditableReflexiveIndex> {
        Some(self)
    }
}

impl<const TAG: usize> SimpleWriteableData for ReflexiveIndex<TAG> {
    fn read_tag_data_simple<B: ByteOrder>(from: &[u8], parameters: Parameters) -> Result<Self, &'static str> {
        Index::read_tag_data_simple::<B>(from, parameters).map(Self)
    }
    fn write_tag_data_simple<B: ByteOrder>(&self, to: &mut [u8], parameters: Parameters) {
        self.0.write_tag_data_simple::<B>(to, parameters)
    }
    fn length() -> usize {
        Index::length()
    }
}

impl<const TAG: usize> PartialOrd for ReflexiveIndex<TAG> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl<const TAG: usize> Ord for ReflexiveIndex<TAG> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
