# retrocompressor

![unit tests](https://github.com/dfgordon/retrocompressor/actions/workflows/rust.yml/badge.svg)

The starting motivation for this project is to provide a library that aids in the handling of TD0 images (teledisk format).  It is envisioned that the scope will expand over time.

At present this performs compression and expansion using an algorithm equivalent to `LZHUF.C` by Okumura et al..
There are two variants, a significant rust rewrite (module `lzss_huff`), and a near-direct port (module `direct_ports::lzhuf`).  The latter is likely under Okumura's license, see source files for more.

## Executable

The executable can be used to compress or expand files from the command line.  For example, to compress or expand a file using the MIT licensed `LZHUF` algorithm

`retrocompressor compress -m lzhuf -i big.txt -o small.lzh`

`retrocompressor expand -m lzhuf -i small.lzh -o big.txt`

To get the general help

`retrocompressor --help`

## Library

This crate can be used as a library.  For an example of how to use the library see `main.rs` (which calls into `lib.rs` per the usual rust arrangement).  Also see the [crate documentation](https://docs.rs/retrocompressor/latest/retrocompressor).

## Teledisk

Teledisk images come in an "advanced" variety that uses compression equivalent to Okumura's `LZHUF` (as far as can be established), which is in turn equivalent to the module `lzss_huff`.  However, the Teledisk header and the `LZHUF` header are different.  To aid in handling this, `lzss_huff::expand` and `lzss_huff::compress` accept options that allow one to omit the usual `LZHUF` header, while skipping over the Teledisk header.  As a convenience there is a module `td0` that handles all this transparently.  This can also be accessed from the command line:

`retrocompressor compress -m td0 -i normal.td0 -o advanced.td0`

`retrocompressor expand -m td0 -i advanced.td0 -o normal.td0`

As of this writing there is a small caveat.  When expanding a TD0, the compressed data is consumed as a bitstream, and since TD0 does not encode the size of the expanded data, it is possible to have an extra byte tacked onto the end of the expanded data.  This extra byte will usually be eliminated downstream when the TD0 is fully processed record by record.

