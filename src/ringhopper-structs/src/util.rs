use core::cmp::Ordering;

/// Compare string `a` with string `b`.
///
/// Useful in const contexts.
#[inline]
pub(crate) const fn strcmp_const(a: &str, b: &str) -> Ordering {
    let a = a.as_bytes();
    let b = b.as_bytes();
    memcmp_const(a, b)
}

/// Compare slice `a` with slice `b`.
///
/// Useful in const contexts.
#[inline]
pub(crate) const fn memcmp_const(a: &[u8], b: &[u8]) -> Ordering {
    let mut z = 0usize;
    loop {
        if z == b.len() {
            if z == a.len() {
                return Ordering::Equal
            }
            return Ordering::Greater
        }
        let a = a[z];
        let b = b[z];
        if a > b {
            return Ordering::Greater
        }
        if a < b {
            return Ordering::Less
        }
        z += 1;
    }
}
