The reference data is in the form of pairs of expanded and compressed data, such as (hamlet_act_1.txt, hamlet_act_1.lzh).

The test code will create a compressed file in a temporary directory, and compare it byte-wise with the reference file.

In some cases the reference file comes from a legacy code whose products we are duplicating:

reference file | legacy code | compiler
------|------|------
hamlet_act_1.lzh | LZHUF.C | clang 16.0.6
tempest_act_5.lzh | LZHUF.C | clang 16.0.6

Invertibility tests are straightforward.