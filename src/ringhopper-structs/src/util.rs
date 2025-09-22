use core::cmp::Ordering;
use core::iter::once;

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

/// Encode string into a null-terminated UTF-8 string.
#[inline]
#[expect(unused)]
pub(crate) fn encode_utf8_null_terminated_string(string: &str) -> impl Iterator<Item = char> {
    string.chars().chain(once('\x00'))
}

/// Encode string into a null-terminated UTF-16 string.
#[inline]
pub(crate) fn encode_utf16_null_terminated_string(string: &str) -> impl Iterator<Item = u16> {
    string.encode_utf16().chain(once(0))
}
