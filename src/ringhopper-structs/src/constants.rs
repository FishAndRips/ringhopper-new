/// DVD sector size
pub const DVD_SECTOR_SIZE: usize = 2048;

/// zlib header table
///
/// All valid compressed zlib streams will start with these bytes.
// Compressed data for a valid Xbox map can only ever start with the following zlib headers
// as for why see the answer by mwfearnley at https://stackoverflow.com/questions/9050260/what-does-a-zlib-header-look-like
pub const ZLIB_HEADER_TABLE: [u16; 32] = [
    0x1D08, 0x5B08, 0x9908, 0xD708,
    0x1918, 0x5718, 0x9518, 0xD318,
    0x1528, 0x5328, 0x9128, 0xCF28,
    0x1138, 0x4F38, 0x8D38, 0xCB38,
    0x0D48, 0x4B48, 0x8948, 0xC748,
    0x0958, 0x4758, 0x8558, 0xC358,
    0x0568, 0x4368, 0x8168, 0xDE68,
    0x0178, 0x5E78, 0x9C78, 0xDA78
];

/// All HUD message text types
pub const HUD_MESSAGE_TEXT_TYPES: &[&str] = &[
    "a-button",
    "b-button",
    "x-button",
    "y-button",
    "black-button",
    "white-button",
    "left-trigger",
    "right-trigger",
    "dpad-up",
    "dpad-down",
    "dpad-left",
    "dpad-right",
    "start-button",
    "back-button",
    "left-thumb",
    "right-thumb",
    "left-stick",
    "right-stick",
    "action",
    "throw-grenade",
    "primary-trigger",
    "integrated-light",
    "jump",
    "use-equipment",
    "rotate-weapons",
    "rotate-grenades",
    "zoom",
    "crouch",
    "accept",
    "back",
    "move",
    "look",
    "custom-1",
    "custom-2",
    "custom-3",
    "custom-4",
    "custom-5",
    "custom-6",
    "custom-7",
    "custom-8"
];

/// The default maximum number of players for a multiplayer game.
pub const DEFAULT_MAX_NUMBER_PLAYERS: usize = 16;


/// The default maximum number of teams for a teamplay multiplayer game.
pub const DEFAULT_MAX_TEAM_COUNT: usize = 2;
