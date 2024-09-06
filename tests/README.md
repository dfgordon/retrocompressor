The reference data is in the form of pairs of expanded and compressed data, such as (hamlet_act_1.txt, hamlet_act_1.lzh).

The test code will create a compressed file in a temporary directory, and compare it byte-wise with the reference file.  A subtle point is that text files cannot be counted on to have the same newlines as the tests are moved between platforms.  Therefore the test will copy any text to the temporary directory and fix its newlines there.

In some cases the reference file comes from a legacy code whose products we are duplicating:

reference file | legacy code | compiler
------|------|------
hamlet_act_1.lzh | LZHUF.C | clang 16.0.6
tempest_act_5.lzh | LZHUF.C | clang 16.0.6
td105.adv.td0 | Teledisk 1.05 | n/a
td105.norm.td0 | Teledisk 1.05 | n/a
td215.adv.td0 | Teledisk 2.15 | n/a
td215.norm.td0 | Teledisk 2.15 | n/a

The larger files cannot be compressed by `LZHUF.C`, at least not when compiled with `clang`.  As a result we can only do invertibility tests on the large files.  The file `shkspr.dsk` is a disk image containing `hamlet_full.txt` in compressed form, which can in turn be compressed (because of the empty sectors).  This provides a test of a binary file.