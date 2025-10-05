use core::cmp::Ordering;
use core::iter::once;
use crate::definitions::tag::bitmap::BitmapGroupSequence;
use crate::definitions::tag::scenario::ScenarioType;

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

/// Change the reference's lifetime.
///
/// # Safety
///
/// Using this function will lead to regret.
#[allow(unused)]
pub(crate) unsafe fn launder_reference_lifetime<'a, T: ?Sized>(a: &'a T) -> &'static T {
    // SAFETY: LOL
    unsafe { &*(a as *const T) }
}

/// Change the reference's lifetime.
///
/// # Safety
///
/// Using this function will lead to even more regret than [`launder_reference_lifetime`].
#[allow(unused)]
pub(crate) unsafe fn launder_reference_lifetime_mut<'a, T: ?Sized>(a: &'a mut T) -> &'static mut T {
    // SAFETY: LOL
    unsafe { &mut *(a as *mut T) }
}

#[expect(unused)]
pub(crate) fn bitmap_group_sequence_is_sprite_sheet(sequence: &BitmapGroupSequence) -> bool {
    sequence.bitmap_count == 0 || !sequence.sprites.is_empty()
}

#[derive(Copy, Clone)]
pub(crate) struct MiniString {
    buffer: [u8; 64],
    len: usize
}

impl MiniString {
    #[expect(unused)]
    pub fn from_fmt(format_args: core::fmt::Arguments) -> Self {
        struct Inner {
            buffer: [u8; 64],
            len: usize
        }
        impl core::fmt::Write for Inner {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for i in s.chars() {
                    let mut bytes = [0u8; 4];
                    let bytes = i.encode_utf8(&mut bytes).as_bytes();
                    let new_len = self.len + bytes.len();

                    if new_len > self.buffer.len() {
                        break
                    }
                    self.buffer[self.len..new_len].copy_from_slice(bytes);
                    self.len = new_len;
                }
                Ok(())
            }
        }

        let mut inner: Inner = Inner {
            buffer: [0u8; 64],
            len: 0
        };

        let _ = core::fmt::write(&mut inner, format_args);

        Self {
            buffer: inner.buffer,
            len: inner.len
        }
    }

    #[expect(unused)]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[0..self.len]).expect("MiniBuffer as_str fail")
    }
}

/// Describes a stock map.
#[derive(Debug)]
pub struct StockMap {
    /// Base name of the scenario tag (excluding file extension).
    ///
    /// Example: `ui`
    pub scenario: &'static str,

    /// Full path of the scenario tag (excluding file extension).
    ///
    /// Example: `levels\ui\ui`
    pub scenario_path: &'static str,

    /// Menu order for the given map.
    ///
    /// This is also the same internal order as the resource maps on Halo PC when building
    /// user interface, then singleplayer, then multiplayer.
    pub menu_order: usize,

    /// Type of map.
    pub scenario_type: ScenarioType,

    /// The map is present on the Xbox version of the game.
    pub xbox: bool
}

/// All known stock maps in lexographical order.
pub const STOCK_MAPS: &[StockMap] = &[
    StockMap { scenario: "a10", scenario_path: "levels\\a10\\a10", menu_order: 0, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "a30", scenario_path: "levels\\a30\\a30", menu_order: 1, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "a50", scenario_path: "levels\\a50\\a50", menu_order: 2, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "b30", scenario_path: "levels\\b30\\b30", menu_order: 3, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "b40", scenario_path: "levels\\b40\\b40", menu_order: 4, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "beavercreek", scenario_path: "levels\\test\\beavercreek\\beavercreek", menu_order: 0, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "bloodgulch", scenario_path: "levels\\test\\bloodgulch\\bloodgulch", menu_order: 9, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "boardingaction", scenario_path: "levels\\test\\boardingaction\\boardingaction", menu_order: 8, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "c10", scenario_path: "levels\\c10\\c10", menu_order: 5, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "c20", scenario_path: "levels\\c20\\c20", menu_order: 6, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "c40", scenario_path: "levels\\c40\\c40", menu_order: 7, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "carousel", scenario_path: "levels\\test\\carousel\\carousel", menu_order: 7, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "chillout", scenario_path: "levels\\test\\chillout\\chillout", menu_order: 6, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "d20", scenario_path: "levels\\d20\\d20", menu_order: 8, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "d40", scenario_path: "levels\\d40\\d40", menu_order: 9, scenario_type: ScenarioType::Singleplayer, xbox: true },
    StockMap { scenario: "damnation", scenario_path: "levels\\test\\damnation\\damnation", menu_order: 2, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "dangercanyon", scenario_path: "levels\\test\\dangercanyon\\dangercanyon", menu_order: 13, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "deathisland", scenario_path: "levels\\test\\deathisland\\deathisland", menu_order: 14, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "gephyrophobia", scenario_path: "levels\\test\\gephyrophobia\\gephyrophobia", menu_order: 15, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "hangemhigh", scenario_path: "levels\\test\\hangemhigh\\hangemhigh", menu_order: 5, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "icefields", scenario_path: "levels\\test\\icefields\\icefields", menu_order: 16, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "infinity", scenario_path: "levels\\test\\infinity\\infinity", menu_order: 17, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "longest", scenario_path: "levels\\test\\longest\\longest", menu_order: 12, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "prisoner", scenario_path: "levels\\test\\prisoner\\prisoner", menu_order: 4, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "putput", scenario_path: "levels\\test\\putput\\putput", menu_order: 11, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "ratrace", scenario_path: "levels\\test\\ratrace\\ratrace", menu_order: 3, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "sidewinder", scenario_path: "levels\\test\\sidewinder\\sidewinder", menu_order: 1, scenario_type: ScenarioType::Multiplayer, xbox: true },
    StockMap { scenario: "timberland", scenario_path: "levels\\test\\timberland\\timberland", menu_order: 18, scenario_type: ScenarioType::Multiplayer, xbox: false },
    StockMap { scenario: "ui", scenario_path: "levels\\ui\\ui", menu_order: 0, scenario_type: ScenarioType::UserInterface, xbox: true },
    StockMap { scenario: "wizard", scenario_path: "levels\\test\\wizard\\wizard", menu_order: 10, scenario_type: ScenarioType::Multiplayer, xbox: true },
];

/// If the given scenario is a stock map, return information about it.
#[inline]
pub fn get_stock_map_info(scenario: &str) -> Option<&'static StockMap> {
    STOCK_MAPS
        .binary_search_by(|what| what.scenario.cmp(scenario))
        .ok()
        .map(|i| &STOCK_MAPS[i])
}

/// Convert the fourcc into a u32.
#[inline]
pub const fn build_fourcc(fourcc_str: &str) -> u32 {
    let &[a,b,c,d] = fourcc_str.as_bytes() else {
        panic!("must be a four character string")
    };
    u32::from_be_bytes([a,b,c,d])
}


#[cfg(test)]
mod test {
    use crate::{get_stock_map_info, STOCK_MAPS};

    #[test]
    pub fn finds_all_stock_maps() {
        for i in STOCK_MAPS {
            assert_eq!(get_stock_map_info(i.scenario).expect("tried to get stock map").scenario, i.scenario);
        }
    }
}
