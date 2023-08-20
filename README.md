# retrocompressor

![unit tests](https://github.com/dfgordon/retrocompressor/actions/workflows/rust.yml/badge.svg)

The starting motivation for this project is to provide a library that aids in the handling of TD0 files (Teledisk-compatible disk images).  It is envisioned that the scope will expand over time.

At present this performs compression and expansion using LZSS with adaptive Huffman coding.  There are two variants:
* `direct_ports::lzhuf` - nearly a direct port of the classic `LZHUF` of Okumura et al.
* `lzss_huff` - signficant rewrite of `LZHUF` with flexible parameters

## Size Limits

This is not optimized for large files.  Some 32-bit integers used to describe file sizes have been retained since they are part of the format.  As of this writing, there are no status indicators available during processing, status only becomes available upon completion or failure.

## Executable

The executable can be used to compress or expand files from the command line.  For example, to compress or expand a file using LZSS with adaptive Huffman coding:

`retrocompressor compress -m lzss_huff -i <big.txt> -o <small.lzh>`

`retrocompressor expand -m lzss_huff -i <small.lzh> -o <big.txt>`

To get the general help

`retrocompressor --help`

## Library

This crate can be used as a library.  For an example of how to use the library see `main.rs` (which calls into `lib.rs` per the usual rust arrangement).  Also see the [crate documentation](https://docs.rs/retrocompressor/latest/retrocompressor).

## Teledisk

Teledisk images come in an "advanced" variety that uses compression equivalent to Okumura's `LZHUF` (as far as can be established), which in turn can be emulated using module `lzss_huff`.  However, the Teledisk header and the `LZHUF` header are different, and the Teledisk header needs to be modified whenever advanced compression is added or subtracted.  As a convenience there is a module `td0` that handles this transparently.  This can also be accessed from the command line:

`retrocompressor compress -m td0 -i <normal.td0> -o <advanced.td0>`

`retrocompressor expand -m td0 -i <advanced.td0> -o <normal.td0>`

As of this writing there is a small caveat.  When expanding a TD0, the compressed data is consumed as a bitstream, and since TD0 does not encode the size of the expanded data, it is possible to have an extra byte tacked onto the end of the expanded data.  Downstream can easily eliminate (or ignore) this extra byte during processing of the TD0 records.

Unfortunately testing TD0 is problematic since the original software is no longer available, and the format remains closed.  If you discover any errors please file an issue.