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
    FileFormatMismatch
}

/// Options controlling compression
pub struct Options {
    /// whether to include an optional header
    header: bool,
    /// starting position in the input file
    in_offset: u64,
    /// starting position in the output file
    out_offset: u64
}

pub const STD_OPTIONS: Options = Options {
    header: true,
    in_offset: 0,
    out_offset: 0
};
