/// Describes a bounds field
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Bounds<T> {
    /// Lower bound
    pub from: T,

    /// Upper bound
    pub to: T
}
