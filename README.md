# retrocompressor

![unit tests](https://github.com/dfgordon/retrocompressor/actions/workflows/rust.yml/badge.svg)

The starting motivation for this project is to provide a library that aids in the handling of TD0 files (Teledisk-compatible disk images).  It is envisioned that the scope will expand over time.

* `direct_ports::lzhuf` - nearly a direct port of the classic `LZHUF` of Okumura et al.
* `lzss_huff` - signficant rewrite of `LZHUF` with flexible parameters
* `td0` - convert normal Teledisk to advanced Teledisk, or vice-versa

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

Testing TD0 is problematic since the original software is no longer available, and the format remains closed.  If you discover any errors please file an issue.

### Important

Advanced TD0 images do not record the length of the expanded data. As a result, some decoders have trouble decoding the last symbol (notably [MAME](https://www.mamedev.org/) v0.257 ).  The workaround is to pad the *expanded* TD0 with several disparate-valued bytes *before* compression.  Teledisk evidently did this, so normally there is no problem, but if you are a creator of TD0 images, it is a good idea to include the padding.