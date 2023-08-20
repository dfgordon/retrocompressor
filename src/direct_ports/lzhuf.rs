//! LZSS Compression with Adaptive Huffman Encoding
//! 
//! This is mostly a direct port of the C program `LZHUF.C` by
//! Haruyasu Yoshizaki, Haruhiko Okumura, and Kenji Rikitake.
//! As a result this is likely under the same license as `LZHUF`:
//!
//! LZHUF.C (c)1989 by Haruyasu Yoshizaki, Haruhiko Okumura, and Kenji Rikitake.
//! All rights reserved. Permission granted for non-commercial use.
//! 
//! Differences from `LZHUF`:
//! * File and bitstream handling is going to look different
//! * Comments are greatly expanded and some identifiers are given longer names
//! * Some components are gathered into structs
//! * The 4 byte header is always little endian
//! 
//! If you need an equivalent program under MIT license, or need more flexibility in
//! the parameters, use the module `retrocompressor::lzss_huff`.
//! 
//! The rust port works more reliably than `LZHUF.C`, which when compiled with `clang 16`,
//! may hang for files >~ 100K.  A possible explanation is that `LZHUF.C` has trouble when
//! it needs to rebuild the Huffman tree.  This in turn could have to do with C integer types
//! being interpreted by clang differently from the original intent.

use bit_vec::BitVec;
use std::io::{Cursor,Read,Write,Seek,SeekFrom,BufReader,BufWriter,Bytes};
use crate::DYNERR;

// LZSS coding constants

const WIN_SIZE: usize = 4096; // sliding buffer
const LOOKAHEAD: usize = 60; // lookahead buffer size
const THRESHOLD: usize = 2; // minimum string length that will be tokenized
const NIL: usize = WIN_SIZE; // pointer value NIL means we have a leaf

// Huffman coding constants

const N_CHAR: usize = 256 - THRESHOLD + LOOKAHEAD; // kinds of characters (character code = 0..N_CHAR-1)
const TAB_SIZE: usize = N_CHAR * 2 - 1; // size of table
const ROOT: usize = TAB_SIZE - 1; // position of root
const MAX_FREQ: usize = 0x8000; // updates tree when the root frequency comes to this value.

/// encoding table giving number of bits used to encode the
/// upper 6 bits of the position
const P_LEN: [u8;64] = [
	0x03, 0x04, 0x04, 0x04, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
	0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08
];

/// codes for the upper 6 bits of position, the P_LEN
/// most significant bits are the code, remaining bits should
/// not be written.
const P_CODE: [u8;64] = [
	0x00, 0x20, 0x30, 0x40, 0x50, 0x58, 0x60, 0x68,
	0x70, 0x78, 0x80, 0x88, 0x90, 0x94, 0x98, 0x9C,
	0xA0, 0xA4, 0xA8, 0xAC, 0xB0, 0xB4, 0xB8, 0xBC,
	0xC0, 0xC2, 0xC4, 0xC6, 0xC8, 0xCA, 0xCC, 0xCE,
	0xD0, 0xD2, 0xD4, 0xD6, 0xD8, 0xDA, 0xDC, 0xDE,
	0xE0, 0xE2, 0xE4, 0xE6, 0xE8, 0xEA, 0xEC, 0xEE,
	0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7,
	0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF
];

/// decoding table for number of bits used to encode the
/// upper 6 bits of the position, the index is the code
/// plus some few bits on the right that don't matter
/// (extra bits are the MSB's of the lower 6 bits)
const D_LEN: [u8;256] = [
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
	0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
];

/// values for the upper 6 bits of position, indexing is
/// the same as for D_LEN
const D_CODE: [u8;256] = [
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
	0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
	0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
	0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
	0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
	0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
	0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
	0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
	0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
	0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
	0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A,
	0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
	0x0C, 0x0C, 0x0C, 0x0C, 0x0D, 0x0D, 0x0D, 0x0D,
	0x0E, 0x0E, 0x0E, 0x0E, 0x0F, 0x0F, 0x0F, 0x0F,
	0x10, 0x10, 0x10, 0x10, 0x11, 0x11, 0x11, 0x11,
	0x12, 0x12, 0x12, 0x12, 0x13, 0x13, 0x13, 0x13,
	0x14, 0x14, 0x14, 0x14, 0x15, 0x15, 0x15, 0x15,
	0x16, 0x16, 0x16, 0x16, 0x17, 0x17, 0x17, 0x17,
	0x18, 0x18, 0x19, 0x19, 0x1A, 0x1A, 0x1B, 0x1B,
	0x1C, 0x1C, 0x1D, 0x1D, 0x1E, 0x1E, 0x1F, 0x1F,
	0x20, 0x20, 0x21, 0x21, 0x22, 0x22, 0x23, 0x23,
	0x24, 0x24, 0x25, 0x25, 0x26, 0x26, 0x27, 0x27,
	0x28, 0x28, 0x29, 0x29, 0x2A, 0x2A, 0x2B, 0x2B,
	0x2C, 0x2C, 0x2D, 0x2D, 0x2E, 0x2E, 0x2F, 0x2F,
	0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
];

/// Components for the LZSS stage of compression.
/// The tree pointers `lson`, `rson`, and `dad` are indexed over the length of the ring buffer.
/// We really have 256 trees.  Each root corresponds to a symbol.
/// The last 256 elements of rson are used for the roots.
/// The tree structure is not involved in decompression.
struct LZSS {
    dictionary: Vec<u8>,
    match_position: i32,
    match_length: usize,
    lson: Vec<usize>,
    rson: Vec<usize>,
    dad: Vec<usize>,
}

/// Components for the Huffman stage of compression.
/// The tree is constantly updated during compression/expansion.
struct AdaptiveHuffman {
    bits: BitVec,
    ptr: usize,
    count: usize,
    /// Frequencies, this is used as a sorting key.
    /// Parent node frequencies are the sum of the child node frequencies.
    freq: Vec<usize>,
    /// 0..TAB_SIZE are a map from child nodes to parent nodes
    /// TAB_SIZE..TAB_SIZE+N_CHAR are a map from symbols to leaf nodes
    prnt: Vec<usize>,
    /// Only the left sons are explicitly stored, but the right son
    /// is always adjacent, i.e. lson[i] = son[i], and rson[i] = son[i]+1.
    /// The leaf nodes have "sons" that point into the symbol-leaf
    /// map, see `prnt`.
    son: Vec<usize>
}

impl LZSS {
    fn new() -> Self {
        Self {
            dictionary: vec![0;WIN_SIZE+LOOKAHEAD-1],
            match_position: 0,
            match_length: 0,
            lson: vec![0;WIN_SIZE+1],
            rson: vec![0;WIN_SIZE+257],
            dad: vec![0;WIN_SIZE+1]
        }
    }
    fn init_tree(&mut self) {
        for i in WIN_SIZE+1..=WIN_SIZE+256 {
            self.rson[i] = NIL; // root
        }
        for i in 0..WIN_SIZE {
            self.dad[i] = NIL; // node
        }
    }
    /// This finds a match to the symbol run starting at position `r` by searching
    /// through the index tree represented by `dad`, `lson`, and `rson`.
    /// It always exits by inserting a node: either for a match that was found,
    /// or for a prospective match to come.
    fn insert_node(&mut self,r: usize) {
        let mut cmp: i16 = 1;
        let key = &self.dictionary[r..];
        // At the start, p corresponds to a symbol, not a position in the ring.
        // When we start looping it will become the position of the current match.
        let mut p = WIN_SIZE + 1 + key[0] as usize;
        self.rson[r] = NIL;
        self.lson[r] = NIL;
        self.match_length = 0;
        // Each iteration of the loop corresponds to a particular match location in
        // the dictionary.  The matches are compared in turn and the longest is chosen.
        loop {
            if cmp>=0 {
                if self.rson[p] != NIL {
                    // make p the position in the ring where a match could start
                    p = self.rson[p];
                } else {
                    // no more matches, index this position and go out
                    self.rson[p] = r;
                    self.dad[r] = p;
                    return;
                }
            } else {
                if self.lson[p] != NIL {
                    p = self.lson[p];
                } else {
                    self.lson[p] = r;
                    self.dad[r] = p;
                    return;
                }
            }
            let mut i: usize = 1;
            // upon exiting this loop, `i` will have the number of matched symbols,
            // and `cmp` will have the difference in first non-matching symbol values.
            while i < LOOKAHEAD {
                cmp = key[i] as i16 - self.dictionary[p+i] as i16;
                if cmp != 0 {
                    break;
                }
                i += 1;
            }
            if i > THRESHOLD {
                if i > self.match_length {
                    // we found a better match, take it
                    self.match_position = ((r as i32 - p as i32) & (WIN_SIZE as i32 - 1)) - 1;
                    self.match_length = i;
                    if self.match_length >= LOOKAHEAD {
                        // cannot get a better match than this, go out
                        break;
                    }
                }
                if i==self.match_length {
                    // if a match has the same length, but occurs with smaller offset, take it
                    let c = ((r as i32 - p as i32) & (WIN_SIZE as i32 - 1)) - 1;
                    if c < self.match_position {
                        self.match_position = c;
                    }
                }
            }
        }
        // if we got here, there was a maximal match.
        // we want to replace the old entry with one pointing to the current location.
        self.dad[r] = self.dad[p];
        self.lson[r] = self.lson[p];
        self.rson[r] = self.rson[p];
        self.dad[self.lson[p]] = r;
        self.dad[self.rson[p]] = r;
        if self.rson[self.dad[p]] == p {
            self.rson[self.dad[p]] = r;
        } else {
            self.lson[self.dad[p]] = r;
        }
        self.dad[p] = NIL;  // remove p
    }
    fn delete_node(&mut self,p: usize) {
        // The big idea here is to delete the node without having to cut a whole branch.
        // If p has only one son, this is easy, that son replaces p.
        // If p has two sons, and the left brother has only a left son, then the right brother's branch
        // gets attached to the left brother's branch (as his right son).
        // If p has two sons, and the left brother has a right son already, it gets more complex,
        // see below.
        let mut q;

        if self.dad[p] == NIL {
            return;			// not registered
        }
        if self.rson[p] == NIL {
            q = self.lson[p]; // simple 1 for 1 replacement
        } else if self.lson[p] == NIL {
            q = self.rson[p]; // simple 1 for 1 replacement
        } else {
            // This is the case where p has 2 sons.  We pick the left brother (arbitrarily?)
            // as the one we will attach to his grandad (we are deleting the dad), unless...
            q = self.lson[p];
            if self.rson[q] != NIL {
                // ...the left brother has a right son.
                // in order to not lose information we must go down and find a node with only
                // one son, in this case we look for one with no right son.
                loop {
                    q = self.rson[q];
                    if self.rson[q] == NIL {
                        break;
                    }
                }
                // now q is the youngest right descendent.
                // take q away from his dad, replacing with q's left son (or NIL)
                self.rson[self.dad[q]] = self.lson[q];
                self.dad[self.lson[q]] = self.dad[q];
                // make q's new left son the former left son of p (the q we started with)
                self.lson[q] = self.lson[p];
                self.dad[self.lson[p]] = q;
            }
            // next 2 lines take original q's brother and make him new q's right son
            self.rson[q] = self.rson[p];
            self.dad[self.rson[p]] = q;
        }
        // family tree is q < p < grandad.  we want to make it q < grandad, losing p.
        self.dad[q] = self.dad[p];
        if self.rson[self.dad[p]] == p {
            self.rson[self.dad[p]] = q;
        } else {
            self.lson[self.dad[p]] = q;
        }
        self.dad[p] = NIL;    
    }
}

impl AdaptiveHuffman {
    /// must create a new object for each coding or decoding task
    fn new() -> Self {
        Self {
            bits: BitVec::new(),
            ptr: 0,
            count: 0,
            freq: vec![0;TAB_SIZE+1], // extra element is the frequency backstop
            prnt: vec![0;TAB_SIZE+N_CHAR], // extra N_CHAR elements are the symbol map
            son: vec![0;TAB_SIZE]
        }
    }
    /// keep the bit vector small, we don't need the bits behind us
    fn drop_leading_bits(&mut self) {
        let cpy = self.bits.clone();
        self.bits = BitVec::new();
        for i in self.ptr..cpy.len() {
            self.bits.push(cpy.get(i).unwrap());
        }
        self.ptr = 0;
    }
    /// initialize the Huffman tree (does not reset bitstream)
    fn start_huff(&mut self) {
        // Leaves are stored first, one for each symbol (character)
        // leaves are signaled by son[i] >= TAB_SIZE, which is the region of
        // prnt that is dedicated to finding the leaves.
        for i in 0..N_CHAR {
            self.freq[i] = 1;
            self.son[i] = i + TAB_SIZE;
            self.prnt[i+TAB_SIZE] = i;
        }
        // Next construct the branches and root, there are N_CHAR-1 non-leaf nodes.
        // The sons will be 0,2,4,...,TAB_SIZE-3, these are left sons, the right sons
        // are not explicitly stored, because we always have rson[i] = lson[i] + 1
        // prnt will be n,n,n+1,n+1,n+2,n+2,...,n+TAB_SIZE-1,n+TAB_SIZE-1
        // Frequency (freq) of a parent node is the sum of the frequencies attached to it.
        // Note the frequencies will be in ascending order.
        let mut i = 0;
        let mut j = N_CHAR;
        while j <= ROOT {
            self.freq[j] = self.freq[i] + self.freq[i+1];
            self.son[j] = i;
            self.prnt[i] = j;
            self.prnt[i+1] = j;
            i += 2;
            j += 1;
        }
        // last frequency entry is a backstop that prevents any frequency from moving
        // beyond the end of the array (must be larger than any possible frequency)
        self.freq[TAB_SIZE] = 0xffff;
        self.prnt[ROOT] = 0;
    }
    /// Rebuild the adaptive Huffman tree, triggered by frequency hitting the maximum.
    fn rebuild_huff(&mut self) {
        // Collect leaf nodes from anywhere and pack them on the left.
        // Replace the freq of every leaf by (freq+1)/2.
        let mut j = 0;
        for i in 0..TAB_SIZE {
            if self.son[i] >= TAB_SIZE {
                self.freq[j] = (self.freq[i] + 1)/2;
                self.son[j] = self.son[i];
                j += 1;
            }
        }
        // Connect sons, old connections are not used in any way.
        // LZHUF has i,j,k as signed, seems to be no reason.
        let mut i: usize = 0; // left son
        j = N_CHAR; // parent node - should already be N_CHAR
        let mut k: usize; // right son or sorting reference
        let mut f: usize; // sum of lson and rson frequencies
        let mut l: usize; // offset from sorting reference to parent node
        while j < TAB_SIZE {
            // first set parent frequency, supposing i,k are sons
            k = i + 1;
            f = self.freq[i] + self.freq[k];
            self.freq[j] = f;
            // make k the farthest node with frequency > this frequency
            k = j - 1;
            while f < self.freq[k] {
                k -= 1;
            }
            k += 1;
            // insert parent of i at position k
            l = (j - k)*2;
            for kp in (k..k+l).rev() {
                self.freq[kp+1] = self.freq[kp]
            }
            self.freq[k] = f;
            for kp in (k..k+l).rev() {
                self.son[kp+1] = self.son[kp]
            }
            self.son[k] = i;
            i += 2; // next left son
            j += 1; // next parent
        }
        // Connect parents.
        // In this loop i is the parent, k is the son
        for i in 0..TAB_SIZE {
            k = self.son[i];
            if k >= TAB_SIZE {
                // k is a leaf, connect to symbol table
                self.prnt[k] = i;
            } else {
                // k=left son, k+1=right son
                self.prnt[k] = i;
                self.prnt[k+1] = i;
            }
        }
    }
    /// increment frequency of given code by one, and update tree
    fn update(&mut self,c0: i16) {
        let mut i: usize;
        let mut j: usize;
        let mut k: usize;
        let mut l: usize;
        if self.freq[ROOT] == MAX_FREQ {
            self.rebuild_huff()
        }
        // the leaf node corresponding to this character, "extra" part of prnt
        let mut c = self.prnt[(c0 as i32 + TAB_SIZE as i32) as usize];
        // sorting loop, node pool is arranged in ascending frequency order
        loop {
            self.freq[c] += 1;
            k = self.freq[c];
            // if order is disturbed, exchange nodes
            l = c + 1;
            if k > self.freq[l] {
                while k > self.freq[l] {
                    l += 1;
                }
                l -= 1;
                // swap the node being checked with the farthest one that is smaller than it
                self.freq[c] = self.freq[l];
                self.freq[l] = k;
                
                i = self.son[c];
                self.prnt[i] = l;
                if i<TAB_SIZE {
                    self.prnt[i+1] = l;
                }
                
                j = self.son[l];
                self.son[l] = i;
                
                self.prnt[j] = c;
                if j<TAB_SIZE {
                    self.prnt[j+1] = c;
                }
                self.son[c] = j;

                c = l;
            }
            c = self.prnt[c];
            if c==0 {
                break; // root was reached
            }
        }
    }
    /// Get the next bit reading from the `bytes` iterator as needed.
    /// When EOF is reached 0 is returned, consistent with original C code.
    /// Byte iterator should not be advanced outside this function.
    fn get_bit<R: Read>(&mut self,bytes: &mut Bytes<R>) -> u8 {
        match self.bits.get(self.ptr) {
            Some(bit) => {
                self.ptr += 1;
                bit as u8
            },
            None => {
                match bytes.next() {
                    Some(Ok(by)) => {
                        if self.bits.len()>512 {
                            self.drop_leading_bits();
                        }
                        self.bits.append(&mut BitVec::from_bytes(&[by]));
                        self.count += 1;
                        self.get_bit(bytes)
                    }
                    Some(Err(e)) => {
                        panic!("error reading file {}",e)
                    },
                    None => 0
                }
            }
        }
    }
    /// get the next 8 bits into a u8, used exlusively to decode the position
    fn get_byte<R: Read>(&mut self,bytes: &mut Bytes<R>) -> u8 {
        let mut ans: u8 = 0;
        for _i in 0..8 {
            ans <<= 1;
            ans |= self.get_bit(bytes);
        }
        ans
    }
    /// output `num_bits` of `code` starting from the MSB, unlike LZHUF.C the bits are always
    /// written to the output stream (sometimes backing up and rewriting)
    fn put_code<W: Write + Seek>(&mut self,num_bits: u16,mut code: u16,writer: &mut BufWriter<W>) {
        for _i in 0..num_bits {
            self.bits.push(code & 0x8000 > 0);
            code <<= 1;
            self.ptr += 1;
        }
        let bytes = self.bits.to_bytes();
        writer.write(&bytes.as_slice()).expect("write err");
        if self.bits.len() % 8 > 0 {
            writer.seek(SeekFrom::Current(-1)).expect("seek err");
            self.ptr = 8 * (self.bits.len() / 8);
            self.drop_leading_bits();
        } else {
            self.bits = BitVec::new();
            self.ptr = 0;
        }
    }
    fn encode_char<W: Write + Seek>(&mut self,c: u16,writer: &mut BufWriter<W>) {
        let mut i: u16 = 0;
        let mut j: u16 = 0;
        let mut k: usize = self.prnt[c as usize + TAB_SIZE];
        // travel from leaf to root
        loop {
            i >>= 1;
            // if node's address is odd-numbered, choose bigger brother node
            if k & 1 > 0 {
                i += 0x8000;
            }
            j += 1;
            k = self.prnt[k];
            if k==ROOT {
                break;
            }
        }
        self.put_code(j,i,writer);
        self.update(c as i16); // TODO: why is input to update signed
    }
    fn encode_position<W: Write + Seek>(&mut self,c: u16,writer: &mut BufWriter<W>) {
        // upper 6 bits come from table
        let i = (c >> 6) as usize;
        self.put_code(P_LEN[i] as u16,(P_CODE[i] as u16) << 8,writer);
        // lower 6 bits verbatim
        self.put_code(6,(c & 0x3f) << 10,writer);
    }
    fn decode_char<R: Read>(&mut self,bytes: &mut Bytes<R>) -> i16 {
        let mut c: usize = self.son[ROOT];
        // travel from root to leaf, choosing the smaller child node (son[])
        // if the read bit is 0, the bigger (son[]+1) if read bit is 1
        while c < TAB_SIZE {
            c += self.get_bit(bytes) as usize;
            c = self.son[c];
        }
        c -= TAB_SIZE;
        self.update(c as i16); // TODO: why is input to update signed
        c as i16
    }
    fn decode_position<R: Read>(&mut self,bytes: &mut Bytes<R>) -> u16 {
        // get upper 6 bits from table
        let mut first8 = self.get_byte(bytes) as u16;
        let upper6 = (D_CODE[first8 as usize] as u16) << 6;
        let coded_bits = D_LEN[first8 as usize] as u16;
        // read lower 6 bits verbatim
        // we already got 8 bits, we need another 6 - (8-coded_bits) = coded_bits - 2
        for _i in 0..coded_bits-2 {
            first8 <<= 1;
            first8 += self.get_bit(bytes) as u16;
        }
        upper6 | (first8 & 0x3f)
    }
}

/// Main compression function
pub fn encode<R: Read + Seek, W: Write + Seek>(expanded_in: &mut R, compressed_out: &mut W) -> Result<(u64,u64),DYNERR> {
    let mut reader = BufReader::new(expanded_in);
    let mut writer = BufWriter::new(compressed_out);
    // write the 32-bit header with length of expanded data
    let expanded_length = reader.seek(SeekFrom::End(0))?;
    if expanded_length >= u32::MAX as u64 {
        return Err(Box::new(crate::Error::FileTooLarge));
    }
    let header = u32::to_le_bytes(expanded_length as u32);
    writer.write(&header)?;
    reader.seek(SeekFrom::Start(0))?;
    // init
    let mut bytes = reader.bytes();
    let mut lzss = LZSS::new();
    let mut huff = AdaptiveHuffman::new();
    huff.start_huff();
    lzss.init_tree();
    // initialize LZSS dictionary
    let mut s = 0;
    let mut r = WIN_SIZE - LOOKAHEAD;
    for i in s..r {
        lzss.dictionary[i] = b' ';
    }
    let mut len = 0;
    while len < LOOKAHEAD {
        match bytes.next() {
            Some(Ok(c)) => {
                lzss.dictionary[r+len] = c;
                len += 1;
            },
            None => {
                break;
            },
            Some(Err(e)) => {
                return Err(Box::new(e));
            }
        }
    }
    for i in 1..=LOOKAHEAD {
        lzss.insert_node(r-i);
    }
    lzss.insert_node(r);
    // start compressing
    loop {
        if lzss.match_length > len {
            lzss.match_length = len;
        }
        if lzss.match_length <= THRESHOLD {
            lzss.match_length = 1;
            huff.encode_char(lzss.dictionary[r] as u16,&mut writer);
        } else {
            huff.encode_char((255-THRESHOLD+lzss.match_length) as u16,&mut writer);
            huff.encode_position(lzss.match_position as u16,&mut writer);
        }
        let last_match_length = lzss.match_length;
        let mut i = 0;
        while i < last_match_length {
            let c = match bytes.next() {
                Some(Ok(c)) => c,
                None => break,
                Some(Err(e)) => return Err(Box::new(e))
            };
            lzss.delete_node(s);
            lzss.dictionary[s] = c;
            if s < LOOKAHEAD - 1 {
                // Mirror into padding after the ring buffer proper,
                // this could be eliminated by better ring-handling in `insert_node`.
                lzss.dictionary[s+WIN_SIZE] = c;
            }
            s = (s+1) & (WIN_SIZE-1);
            r = (r+1) & (WIN_SIZE-1);
            lzss.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            lzss.delete_node(s);
            s = (s+1) & (WIN_SIZE-1);
            r = (r+1) & (WIN_SIZE-1);
            len -= 1;
            if len > 0 {
                lzss.insert_node(r);
            }
            i += 1;
        }
        if len <= 0 {
            break;
        }
    }
    writer.seek(SeekFrom::End(0))?; // coder could be rewound
    writer.flush()?;
    Ok((expanded_length,writer.stream_position()?))
}

/// Main decompression function.
/// Returns (compressed size, expanded size) or error.
pub fn decode<R: Read + Seek , W: Write + Seek>(compressed_in: &mut R, expanded_out: &mut W) -> Result<(u64,u64),DYNERR>
{
    let mut reader = BufReader::new(compressed_in);
    let mut writer = BufWriter::new(expanded_out);
    // get size of expanded data from 32 bit header
    let mut header: [u8;4] = [0;4];
    reader.read_exact(&mut header)?;
    let textsize = u32::from_le_bytes(header);
    // init
    let mut bytes = reader.bytes();
    let mut huff = AdaptiveHuffman::new();
    let mut lzss= LZSS::new();
	huff.start_huff();
	for i in 0..WIN_SIZE - LOOKAHEAD {
		lzss.dictionary[i] = b' ';
    }
	let mut r = WIN_SIZE - LOOKAHEAD;
    // start expanding
	while writer.stream_position()? < textsize as u64 {
		let c = huff.decode_char(&mut bytes);
		if c < 256 {
            writer.write(&[c as u8])?;
			lzss.dictionary[r] = c as u8;
            r += 1;
			r &= WIN_SIZE - 1;
		} else {
			let strpos = ((r as i32 - huff.decode_position(&mut bytes) as i32 - 1) & (WIN_SIZE as i32 - 1)) as usize;
			let strlen = c as usize + THRESHOLD - 255;
			for k in 0..strlen {
				let c8 = lzss.dictionary[(strpos + k) & (WIN_SIZE - 1)];
                writer.write(&[c8])?;
				lzss.dictionary[r] = c8;
                r += 1;
				r &= WIN_SIZE - 1;
			}
		}
	}
    writer.flush()?;
    Ok((huff.count as u64,writer.stream_position()?))
}

/// Convenience function, calls `decode` with a slice returning a Vec
pub fn decode_slice(slice: &[u8]) -> Result<Vec<u8>,DYNERR> {
    let mut src = Cursor::new(slice);
    let mut ans: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    decode(&mut src,&mut ans)?;
    Ok(ans.into_inner())
}

/// Convenience function, calls `encode` with a slice returning a Vec
pub fn encode_slice(slice: &[u8]) -> Result<Vec<u8>,DYNERR> {
    let mut src = Cursor::new(slice);
    let mut ans: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    encode(&mut src,&mut ans)?;
    Ok(ans.into_inner())
}

#[test]
fn compression_works() {
    let test_data = "12345123456789123456789\n".as_bytes();
    let lzhuf_str = "18 00 00 00 DE EF B7 FC 0E 0C 70 13 85 C3 E2 71 64 81 19 60";
    let compressed = encode_slice(test_data).expect("encoding failed");
    assert_eq!(compressed,hex::decode(lzhuf_str.replace(" ","")).unwrap());

    let test_data = "I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let lzhuf_str = "31 00 00 00 EA EB 3D BF 9C 4E FE 1E 16 EA 34 09 1C 0D C0 8C 02 FC 3F 77 3F 57 20 17 7F 1F 5F BF C6 AB 7F A5 AF FE 4C 39 96";
    let compressed = encode_slice(test_data).expect("encoding failed");
    assert_eq!(compressed,hex::decode(lzhuf_str.replace(" ","")).unwrap());
}

#[test]
fn invertibility() {
    let test_data = "I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let compressed = encode_slice(test_data).expect("encoding failed");
    let expanded = decode_slice(&compressed).expect("decoding failed");
    assert_eq!(test_data.to_vec(),expanded);

    let test_data = "1234567".as_bytes();
    let compressed = encode_slice(test_data).expect("encoding failed");
    let expanded = decode_slice(&compressed).expect("decoding failed");
    assert_eq!(test_data.to_vec(),expanded[0..7]);
}