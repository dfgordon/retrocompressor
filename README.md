# retrocompressor

The starting motivation for this project is to enable full R/W access of teledisks (TD0 images) in `a2kit`.  It is envisioned that the scope will expand over time.

At present this performs compression and expansion of files (CLI) or buffers (crate) using an algorithm equivalent to `LZHUF.C` by Okumura et al..
There are two variants, a significant rust rewrite, and a near-direct port.  The latter is likely under Okumura's license, see source files for more.

