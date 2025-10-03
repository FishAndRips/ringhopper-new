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
