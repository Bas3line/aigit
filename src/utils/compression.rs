use flate2::{Compression, write::ZlibEncoder, read::ZlibDecoder};
use std::io::{Write, Read};

pub fn compress(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

pub fn compress_with_level(data: &[u8], level: Compression) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), level);
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn get_compression_ratio(original_size: usize, compressed_size: usize) -> f32 {
    if original_size == 0 {
        return 0.0;
    }
    (original_size - compressed_size) as f32 / original_size as f32 * 100.0
}
