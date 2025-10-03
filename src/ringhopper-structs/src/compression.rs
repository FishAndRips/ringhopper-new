use alloc::format;
use crate::constants::ZLIB_HEADER_TABLE;
use crate::LoadCacheFileError;
use alloc::borrow::Cow;
use core::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub struct CompressionError(pub Cow<'static, str>);

impl From<CompressionError> for LoadCacheFileError {
    fn from(value: CompressionError) -> Self {
        LoadCacheFileError::CorruptMap { description: format!("Compression error: {}", value.0) }
    }
}

impl Display for CompressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

/// Decompress the data into the buffer.
/// 
/// If successful, the actual size of the decompressed data will be returned. Otherwise, a [`CompressionError`] will be returned.
pub fn zlib_decompress(compressed_data: &[u8], output_decompressed_data: &mut [u8]) -> Result<usize, CompressionError> {
    if !ZLIB_HEADER_TABLE
        .iter()
        .rev() // doing reverse search because it is more likely to hit the last two
        .map(|i| i.to_le_bytes())
        .any(|i| compressed_data.starts_with(i.as_slice())) {
        return Err(CompressionError(Cow::Borrowed("Decompression failed; map does not appear to be compressed or has an invalid zlib header")))
    }
    
    let mut inflate_stream = libz_rs_sys::z_stream::default();
    inflate_stream.configure_default_rust_allocator();
    assert!(size_of_val(&inflate_stream.avail_in) >= size_of::<u32>(), "bad sizes");

    let stream_size = size_of_val(&inflate_stream) as i32;
    let version = libz_rs_sys::zlibVersion();
    let err = unsafe { libz_rs_sys::inflateInit_(&mut inflate_stream, version, stream_size) };
    assert_eq!(err, libz_rs_sys::Z_OK, "Failed to init inflate stream");

    inflate_stream.avail_in = compressed_data.len().try_into().expect("compressed data exceeds the size of a u32");
    inflate_stream.next_in = compressed_data.as_ptr();

    inflate_stream.avail_out = output_decompressed_data.len().try_into().expect("decompressed data exceeds the size of a u32");
    inflate_stream.next_out = output_decompressed_data.as_mut_ptr();

    let err = unsafe { libz_rs_sys::inflate(&mut inflate_stream, libz_rs_sys::Z_FINISH) };
    if err != libz_rs_sys::Z_STREAM_END {
        return Err(CompressionError(Cow::Owned(format!("Decompression failed: got a zlib error {err}"))))
    }

    let err = unsafe { libz_rs_sys::inflateEnd(&mut inflate_stream) };
    assert_eq!(err, libz_rs_sys::Z_OK);

    Ok(inflate_stream.total_out as usize)
}
