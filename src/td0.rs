//! Teledisk-compatible compression
//! 
//! This module allows enabling or disabling of advanced compression in
//! TD0 image files.  This does not do any analysis of the TD0 image, it
//! only verifies and updates the 2-byte signature, CRC, and compresses or
//! expands everything following the 12-byte header.
//! 
//! Because TD0 does not store the size of the expanded image, there can be
//! an extra byte at the end of the file after expanding.  It appears Teledisk
//! would usually (maybe always) pad the expanded data by several bytes.
//! Some decoders count on this padding to correctly decode the last symbol.
//! The aforementioned issue does not apply to v1.x (LZW) compression.

use std::io::{Cursor,Read,Write,Seek};
use crate::DYNERR;
use crate::lzss_huff;
use crate::lzw;

/// Calculate the checksum for the TD0 data in `buf`.
/// This is only used for the image header, the several other CRC in the TD0 require
/// no explicit handling at the level of this module.
pub fn crc16(crc_seed: u16, buf: &[u8]) -> u16
{
    let mut crc: u16 = crc_seed;
    for i in 0..buf.len() {
        crc ^= (buf[i] as u16) << 8;
        for _bit in 0..8 {
            crc = (crc << 1) ^ match crc & 0x8000 { 0 => 0, _ => 0xa097 };
        }
    }
    crc
}

pub const TD_V1_OPTIONS: lzw::Options = lzw::Options {
    header_bits: 16,
    header_divisor: 4,
    in_offset: 12,
    out_offset: 12,
    chunk_size: 4096,
    min_symbol: 0,
    max_symbol: 255,
    clear_code: None,
    stop_code: None,
    min_code_width: 12,
    max_code_width: 12,
    ord: crate::BitOrder::Lsb0,
    max_file_size: 3_000_000
};

pub const TD_V2_OPTIONS: lzss_huff::Options = lzss_huff::Options {
    header: false,
    in_offset: 12,
    out_offset: 12,
    window_size: 4096,
    threshold: 2,
    lookahead: 60,
    precursor: b' ',
    max_file_size: 3_000_000
};

/// Convert a TD0 image from advanced compression to normal.
/// For Teledisk 2.x, the heavy lifting is done by the `lzss_huff` module.
/// For Teledisk 1.x, the heavy lifting is done by the `lzw` module.
pub fn expand<R,W>(compressed_in: &mut R, expanded_out: &mut W) -> Result<(u64,u64),DYNERR>
where R: Read + Seek, W: Write + Seek {
    let mut td_header: [u8;12] = [0;12];
    compressed_in.read_exact(&mut td_header)?;
    if &td_header[0..2] != "td".as_bytes() {
        return Err(Box::new(crate::Error::FileFormatMismatch))
    }
    let crc = u16::to_le_bytes(crc16(0,&td_header[0..10]));
    if crc!=td_header[10..12] {
        return Err(Box::new(crate::Error::BadChecksum))
    }
    td_header[0..2].copy_from_slice("TD".as_bytes());
    let crc = u16::to_le_bytes(crc16(0,&td_header[0..10]));
    td_header[10..12].copy_from_slice(&crc);
    expanded_out.write_all(&td_header)?;
    // Dunfield's notes suggest looking at nibble values, but we find it is the decimal digits that count
    if td_header[4] < 20 {
        let (in_size,out_size) = lzw::expand(compressed_in,expanded_out,&TD_V1_OPTIONS)?;
        Ok((in_size+td_header.len() as u64,out_size+td_header.len() as u64))
    } else {
        let (in_size,out_size) = lzss_huff::expand(compressed_in,expanded_out,&TD_V2_OPTIONS)?;
        Ok((in_size+td_header.len() as u64,out_size+td_header.len() as u64))
    }
}

/// Convert a TD0 image from normal to advanced compression.
/// For Teledisk 2.x, the heavy lifting is done by the `lzss_huff` module.
/// For Teledisk 1.x, the heavy lifting is done by the `lzw` module.
pub fn compress<R,W>(expanded_in: &mut R, compressed_out: &mut W) -> Result<(u64,u64),DYNERR>
where R: Read + Seek, W: Write + Seek {
    let mut td_header: [u8;12] = [0;12];
    expanded_in.read_exact(&mut td_header)?;
    if &td_header[0..2] != "TD".as_bytes() {
        return Err(Box::new(crate::Error::FileFormatMismatch))
    }
    let crc = u16::to_le_bytes(crc16(0,&td_header[0..10]));
    if crc!=td_header[10..12] {
        return Err(Box::new(crate::Error::BadChecksum))
    }
    td_header[0..2].copy_from_slice("td".as_bytes());
    let crc = u16::to_le_bytes(crc16(0,&td_header[0..10]));
    td_header[10..12].copy_from_slice(&crc);
    compressed_out.write_all(&td_header)?;
    // Dunfield's notes suggest looking at nibble values, but we find it is the decimal digits that count
    if td_header[4] < 20 {
        let (in_size,out_size) = lzw::compress(expanded_in,compressed_out,&TD_V1_OPTIONS)?;
        Ok((in_size+td_header.len() as u64,out_size+td_header.len() as u64))
    } else {
        let (in_size,out_size) = lzss_huff::compress(expanded_in,compressed_out,&TD_V2_OPTIONS)?;
        Ok((in_size+td_header.len() as u64,out_size+td_header.len() as u64))
    }
}

/// Convenience function, calls `compress` with a slice returning a Vec
pub fn compress_slice(slice: &[u8]) -> Result<Vec<u8>,DYNERR> {
    let mut src = Cursor::new(slice);
    let mut ans: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    compress(&mut src,&mut ans)?;
    Ok(ans.into_inner())
}

/// Convenience function, calls `expand` with a slice returning a Vec
pub fn expand_slice(slice: &[u8]) -> Result<Vec<u8>,DYNERR> {
    let mut src = Cursor::new(slice);
    let mut ans: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    expand(&mut src,&mut ans)?;
    Ok(ans.into_inner())
}

#[test]
fn compression_works() {
    let mut normal_header = "TD0123456789".as_bytes().to_vec();
    let normal_data = "I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes().to_vec();
    let crc = u16::to_le_bytes(crc16(0,&normal_header[0..10]));
    normal_header[10..12].copy_from_slice(&crc);

    let mut advanced_header = "td0123456789".as_bytes().to_vec();
    let advanced_data = "EA EB 3D BF 9C 4E FE 1E 16 EA 34 09 1C 0D C0 8C 02 FC 3F 77 3F 57 20 17 7F 1F 5F BF C6 AB 7F A5 AF FE 4C 39 96";
    let crc = u16::to_le_bytes(crc16(0,&advanced_header[0..10]));
    advanced_header[10..12].copy_from_slice(&crc);

    let test_data = [normal_header,normal_data].concat();
    let compressed = compress_slice(&test_data).expect("compression failed");
    let expected = [advanced_header,hex::decode(advanced_data.replace(" ","")).unwrap()].concat();
    assert_eq!(compressed,expected);
}

#[test]
fn invertibility() {
    let mut test_data = "TD0123456789I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes().to_vec();
    let crc = u16::to_le_bytes(crc16(0,&test_data[0..10]));
    test_data[10..12].copy_from_slice(&crc);
    let compressed = compress_slice(&test_data).expect("compression failed");
    let expanded = expand_slice(&compressed).expect("expansion failed");
    assert_eq!(test_data.to_vec(),expanded);
}