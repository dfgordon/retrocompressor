//! # Retrocompressor Library
//! 
//! Compress or expand retro file formats
//! * `direct_ports::lzhuf` is a nearly direct port of `LZHUF` by Okumura et al.
//! * `lzss_huff` produces output compatible with `LZHUF` using a different implementation
//! * `td0` converts between advanced (compressed) and normal (expanded) TD0 disk image formats
//! 
//! The compression/expansion functions are generics that operate on trait objects
//! with bounds `Read + Seek` or `Write + Seek`.  There are convenience functions for working
//! directly with buffers.
//! 
//! ## File Example
//! 
//! ```rs
//! use retrocompressor::*;
//! let mut in_file = std::fs::File::open("some_input_path").expect("open failed");
//! let mut out_file = std::fs::File::create("some_output_path").expect("create failed");
//! let (in_size,out_size) = lzss_huff::expand(&mut in_file,&mut out_file,&STD_OPTIONS)
//!     .expect("expansion failed");
//! eprintln!("expanded {} into {}",in_size,out_size);
//! ```
//! 
//! ## Buffer Example
//! 
//! ```rs
//! use retrocompressor::*;
//! let test_data = "This is the chaunt of the priests.  The chaunt of the priests of Mung.".as_bytes();
//! let compressed = lzss_huff::compress_slice(test_data,&STD_OPTIONS).expect("compression failed");
//! ```

mod tools;
pub mod lzss_huff;
pub mod td0;
pub mod direct_ports;

type DYNERR = Box<dyn std::error::Error>;
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

/// Tree Errors
#[derive(thiserror::Error,Debug)]
pub enum Error {
    #[error("file format mismatch")]
    FileFormatMismatch,
    #[error("file too large")]
    FileTooLarge,
    #[error("checksum failed")]
    BadChecksum
}

/// Options controlling compression
#[derive(Clone)]
pub struct Options {
    /// whether to include an optional header
    header: bool,
    /// starting position in the input file
    in_offset: u64,
    /// starting position in the output file
    out_offset: u64,
    /// size of window, e.g., for LZSS dictionary
    window_size: usize,
    /// threshold, e.g. minimum length of match to encode
    threshold: usize,
    /// lookahead, e.g. for LZSS matches
    lookahead: usize,
    /// precursor symbol, e.g. backfill symbol for LZSS dictionary
    precursor: u8
}

pub const STD_OPTIONS: Options = Options {
    header: true,
    in_offset: 0,
    out_offset: 0,
    window_size: 4096,
    threshold: 2,
    lookahead: 60,
    precursor: b' '
};
