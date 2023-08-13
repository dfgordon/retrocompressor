//! Module to perform the adaptive Huffman coding.
//! This is used by the `lzss_huff` module.
//! This is supposed to perform the coding the same way as `LZHUF.C`,
//! see the `direct_ports` module for more on the legacy.

use bit_vec::BitVec;

/// Components for the Huffman stage of compression.
/// The tree is constantly updated as new data is decoded.
pub struct AdaptiveHuffman {
    max_freq: usize,
    num_symb: usize,
    node_count: usize,
    root: usize,
    bits: BitVec,
    ptr: usize,
    /// node frequency and sorting key, extra is the frequency backstop
    freq: Vec<usize>,
    /// index of parent node of the node in this slot
    parent: Vec<usize>,
    /// index of the left son of the node in this slot, right son is found by incrementing by 1
    son: Vec<usize>,
    /// map from symbols (index) to leaves (value)
    symb_map: Vec<usize>
}

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

impl AdaptiveHuffman {
    /// The `dat` argument is always the input, whether we are compressing or expanding.
    pub fn create(dat: Vec<u8>,num_symbols: usize) -> Self {
        Self {
            max_freq: 0x8000,
            num_symb: num_symbols,
            node_count: 2*num_symbols - 1,
            root: 2*num_symbols - 2,
            bits: BitVec::from_bytes(&dat),
            ptr: 0,
            freq: vec![0;2*num_symbols],
            parent: vec![0;2*num_symbols-1],
            son: vec![0;2*num_symbols-1],
            symb_map: vec![0;num_symbols]
        }
    }
    pub fn advance(&mut self,bits: usize) {
        self.ptr += bits;
    }
    /// initialize the Huffman tree
    pub fn start_huff(&mut self) {
        // Leaves are stored first, one for each symbol (character)
        // leaves are signaled by son[i] >= node_count
        for i in 0..self.num_symb {
            self.freq[i] = 1;
            self.son[i] = i + self.node_count;
            self.symb_map[i] = i;
        }
        // Next construct the branches and root, there are num_symb-1 non-leaf nodes.
        // The sons will be 0,2,4,...,node_count-3, these are left sons, the right sons
        // are not explicitly stored, because we always have rson[i] = lson[i] + 1
        // prnt will be n,n,n+1,n+1,n+2,n+2,...,n+node_count-1,n+node_count-1
        // Frequency (freq) of a parent node is the sum of the frequencies attached to it.
        // Note the frequencies will be in ascending order.
        let mut i = 0;
        let mut j = self.num_symb;
        while j <= self.root {
            self.freq[j] = self.freq[i] + self.freq[i+1];
            self.son[j] = i;
            self.parent[i] = j;
            self.parent[i+1] = j;
            i += 2;
            j += 1;
        }
        // last frequency entry is a backstop that prevents any frequency from moving
        // beyond the end of the array (must be larger than any possible frequency)
        self.freq[self.node_count] = 0xffff;
        self.parent[self.root] = 0;
    }
    /// Rebuild the adaptive Huffman tree, triggered by frequency hitting the maximum.
    fn rebuild_huff(&mut self) {
        // Collect leaf nodes from anywhere and pack them on the left.
        // Replace the freq of every leaf by (freq+1)/2.
        let mut j = 0;
        for i in 0..self.node_count {
            if self.son[i] >= self.node_count {
                self.freq[j] = (self.freq[i] + 1)/2;
                self.son[j] = self.son[i];
                j += 1;
            }
        }
        // Connect sons, old connections are not used in any way.
        // LZHUF has i,j,k as signed, seems to be no reason.
        let mut i: usize = 0; // left son
        j = self.num_symb; // parent node - should already be num_symb
        let mut k: usize; // right son or sorting reference
        let mut f: usize; // sum of lson and rson frequencies
        let mut l: usize; // offset from sorting reference to parent node
        while j < self.node_count {
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
        for i in 0..self.node_count {
            k = self.son[i];
            if k >= self.node_count {
                // k is a leaf, connect to symbol table
                self.symb_map[k-self.node_count] = i;
            } else {
                // k=left son, k+1=right son
                self.parent[k] = i;
                self.parent[k+1] = i;
            }
        }
    }
    /// increment frequency of given code by one, and update tree
    fn update(&mut self,c0: i16) {
        let mut i: usize;
        let mut j: usize;
        let mut k: usize;
        let mut l: usize;
        if self.freq[self.root] == self.max_freq {
            self.rebuild_huff()
        }
        // the leaf node corresponding to this character, "extra" part of prnt
        let mut c = self.symb_map[c0 as usize];
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
                if i<self.node_count {
                    self.parent[i] = l;
                    self.parent[i+1] = l;
                } else {
                    self.symb_map[i-self.node_count] = l;
                }
                
                j = self.son[l];
                self.son[l] = i;
                
                if j<self.node_count {
                    self.parent[j] = c;
                    self.parent[j+1] = c;
                } else {
                    self.symb_map[j-self.node_count] = c;
                }
                self.son[c] = j;

                c = l;
            }
            c = self.parent[c];
            if c==0 {
                break; // root was reached
            }
        }
    }
    /// get the next bit based on the internal bit pointer
    fn get_bit(&mut self) -> u8 {
        match self.bits.get(self.ptr) {
            Some(bit) => {
                self.ptr += 1;
                bit as u8
            },
            None => 0
        }
    }
    /// get the next 8 bits into a u16, used exlusively to decode the position
    fn get_byte(&mut self) -> u8 {
        let mut ans: u8 = 0;
        for _i in 0..8 {
            ans <<= 1;
            ans |= self.get_bit();
        }
        ans
    }
    /// output `num_bits` of `code` starting from the MSB
    fn put_code(&mut self,num_bits: u16,mut code: u16,obuf: &mut BitVec) {
        for _i in 0..num_bits {
            obuf.push(code & 0x8000 > 0);
            code <<= 1;
        }
    }
    pub fn encode_char(&mut self,c: u16,obuf: &mut BitVec) {
        let mut i: u16 = 0;
        let mut j: u16 = 0;
        let mut k: usize = self.symb_map[c as usize];
        // travel from leaf to root
        loop {
            i >>= 1;
            // if node's address is odd-numbered, choose bigger brother node
            if k & 1 > 0 {
                i += 0x8000;
            }
            j += 1;
            k = self.parent[k];
            if k==self.root {
                break;
            }
        }
        self.put_code(j,i,obuf);
        self.update(c as i16); // TODO: why is input to update signed
    }
    pub fn encode_position(&mut self,c: u16,obuf: &mut BitVec) {
        // upper 6 bits come from table
        let i = (c >> 6) as usize;
        self.put_code(P_LEN[i] as u16,(P_CODE[i] as u16) << 8,obuf);
        // lower 6 bits verbatim
        self.put_code(6,(c & 0x3f) << 10,obuf);
    }
    pub fn decode_char(&mut self) -> i16 {
        let mut c: usize = self.son[self.root];
        // travel from root to leaf, choosing the smaller child node (son[])
        // if the read bit is 0, the bigger (son[]+1) if read bit is 1
        while c < self.node_count {
            c += self.get_bit() as usize;
            c = self.son[c];
        }
        c -= self.node_count;
        self.update(c as i16); // TODO: why is input to update signed
        c as i16
    }
    pub fn decode_position(&mut self) -> u16 {
        // get upper 6 bits from table
        let mut first8 = self.get_byte() as u16;
        let upper6 = (D_CODE[first8 as usize] as u16) << 6;
        let coded_bits = D_LEN[first8 as usize] as u16;
        // read lower 6 bits verbatim
        // we already got 8 bits, we need another 6 - (8-coded_bits) = coded_bits - 2
        for _i in 0..coded_bits-2 {
            first8 <<= 1;
            first8 += self.get_bit() as u16;
        }
        upper6 | (first8 & 0x3f)
    }
}

