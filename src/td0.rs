//! Teledisk compression
//! 
//! This module allows enabling or disabling of advanced compression in
//! TD0 image files.  This does not do any analysis of the TD0 image, it
//! only verifies and updates the 2-byte signature, and compresses or
//! expands everything following the 12-byte header.
//! 
//! Because TD0 does not store the size of the expanded image, there can be
//! an extra byte at the end of the file after expanding.  It is easy for downstream
//! to eliminate this possible extra byte during processing of the image data.

use std::io::{Cursor,Read,Write,Seek};
use crate::DYNERR;
use crate::lzss_huff;

/// Convert a TD0 image from advanced compression to normal.
/// The heavy lifting is done by the `lzss_huff` module.
pub fn expand<R,W>(compressed_in: &mut R, expanded_out: &mut W) -> Result<(u64,u64),DYNERR>
where R: Read + Seek, W: Write + Seek {
    let mut td_header: [u8;12] = [0;12];
    compressed_in.read_exact(&mut td_header)?;
    if td_header[0]!=b't' || td_header[1]!=b'd' {
        return Err(Box::new(crate::Error::FileFormatMismatch))
    }
    td_header[0] = b'T';
    td_header[1] = b'D';
    expanded_out.write_all(&td_header)?;
    let opt = crate::Options {
        header: false,
        in_offset: 12,
        out_offset: 12
    };
    let (in_size,out_size) = lzss_huff::expand(compressed_in,expanded_out,&opt)?;
    Ok((in_size,out_size))
}

/// Convert a TD0 image from normal to advanced compression.
/// The heavy lifting is done by the `lzss_huff` module.
pub fn compress<R,W>(expanded_in: &mut R, compressed_out: &mut W) -> Result<(u64,u64),DYNERR>
where R: Read + Seek, W: Write + Seek {
    let mut td_header: [u8;12] = [0;12];
    expanded_in.read_exact(&mut td_header)?;
    if td_header[0]!=b'T' || td_header[1]!=b'D' {
        return Err(Box::new(crate::Error::FileFormatMismatch))
    }
    td_header[0] = b't';
    td_header[1] = b'd';
    compressed_out.write_all(&td_header)?;
    let opt = crate::Options {
        header: false,
        in_offset: 12,
        out_offset: 12
    };
    let (in_size,out_size) = lzss_huff::compress(expanded_in,compressed_out,&opt)?;
    Ok((in_size,out_size))
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
    let test_data = "TD0123456789I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let lzhuf_str = "EA EB 3D BF 9C 4E FE 1E 16 EA 34 09 1C 0D C0 8C 02 FC 3F 77 3F 57 20 17 7F 1F 5F BF C6 AB 7F A5 AF FE 4C 39 96";
    let compressed = compress_slice(test_data).expect("compression failed");
    let expected = [
        "td0123456789".as_bytes().to_vec(),
        hex::decode(lzhuf_str.replace(" ","")).unwrap()
    ].concat();
    assert_eq!(compressed,expected);
}

#[test]
fn invertibility() {
    let test_data = "TD0123456789I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let compressed = compress_slice(test_data).expect("compression failed");
    let expanded = expand_slice(&compressed).expect("expansion failed");
    assert_eq!(test_data.to_vec(),expanded);
}