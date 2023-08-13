//! LZSS Compression with Adaptive Huffman Encoding
//! 
//! This performs compression equivalent to the C program `LZHUF.C` by
//! Haruyasu Yoshizaki, Haruhiko Okumura, and Kenji Rikitake.  This is not a direct
//! port, but it will produce the same bit-for-bit output as `LZHUF.C`.
//! 
//! * This transforms buffers, not files (we expect files that are easily buffered)
//! * The 4 byte header is always little endian
//! 
//! This program appears to work more reliably than `LZHUF.C`.
//! I found that `LZHUF.C` will hang on large files when compiled with `clang 16`,
//! among other problems.  One theory is this happens when it gets to the stage
//! where the Huffman tree has to be rebuilt, and something goes amiss with the
//! C integer types as interpreted by clang (compared to whatever old compiler).
//! Neither this program nor the direct port exhibit such problems.

use bit_vec::BitVec;
use crate::tools::node_pool::*;
use crate::tools::ring_buffer::*;
use crate::tools::adaptive_huff::*;

// LZSS coding constants

const WIN_SIZE: usize = 4096; // sliding buffer
const LOOKAHEAD: usize = 60; // lookahead buffer size
const THRESHOLD: usize = 2; // minimum string length that will be tokenized

/// Structure to perform the LZSS stage of  compression.
/// This maintains two components.  First a sliding window containing
/// the symbols in the order encountered ("dictionary"), and second a
/// tree structure whose nodes point at dictionary locations where matches
/// have been previously found ("index")
struct LZSS {
    dictionary: RingBuffer,
    index: Tree,
    match_offset: i32,
    match_length: usize
}

impl LZSS {
    fn new() -> Self {
        Self {
            dictionary: RingBuffer::create(WIN_SIZE),
            index: Tree::create(WIN_SIZE, 256),
            match_offset: 0,
            match_length: 0
        }
    }
    /// This finds a match to the symbol run starting at position `pos`.
    /// It always exits by inserting a node: either for a match that was found,
    /// or for a prospective match to come.
    fn insert_node(&mut self) -> Result<(),Error> {
        let pos = self.dictionary.get_pos(0);
        self.match_length = 0;
        // Whatever is attached at this position can only index things that are ahead of us.
        // Therefore throw it all away. (but see note below)
        self.index.set_cursor(pos)?;
        self.index.drop_branch(Side::Left)?;
        self.index.drop_branch(Side::Right)?;
        // self.index.cut_downward(Side::Left)?;
        // self.index.cut_downward(Side::Right)?;
        // find or create root for this symbol
        let symbol = self.dictionary.get(0);
        let mut curs = match self.index.set_cursor_to_root(symbol as usize) {
            Ok(()) => self.index.get_cursor().unwrap(),
            Err(_) => {
                // Symbol has not been indexed yet, save position and go out.
                self.index.spawn_root(symbol as usize, pos)?;
                return Ok(());
            }
        };
        self.index.set_cursor(curs)?;
        loop {
            let mut cmp = 0;
            let mut i: usize = 1;
            // upon exiting this loop, `i` will have the number of matched symbols,
            // and `cmp` will have the difference in first mismatched symbol values.
            while i < LOOKAHEAD {
                cmp = self.dictionary.get(i as i64) as i16 - self.dictionary.get_abs(curs+i) as i16;
                if cmp != 0 {
                    break;
                }
                i += 1;
            }
            if i > THRESHOLD {
                if i > self.match_length {
                    // we found a better match, take it
                    self.match_offset = self.dictionary.distance_behind(curs) as i32 - 1;
                    self.match_length = i;
                    if self.match_length >= LOOKAHEAD {
                        // cannot get a better match than this, so remove the prior position from the index,
                        // and index this position in its place. TODO: this seems to break the assumption
                        // that farther from root means later in buffer.
                        self.index.change_value(pos)?;
                        return Ok(());
                    }
                }
                if i==self.match_length {
                    // if a match has the same length, but occurs with smaller offset, take it
                    let c = self.dictionary.distance_behind(curs) as i32 - 1;
                    if c < self.match_offset {
                        self.match_offset = c;
                    }
                }
            }
            // try next match on one of two branches, determined by the symbol ordering associated
            // with the last mismatch.
            let side = match cmp >= 0 {
                true => Side::Right,
                false => Side::Left
            };
            curs = match self.index.down(side) {
                Ok(c) => c,
                Err(Error::NodeMissing) => {
                    // no match, make this position a new node, go out
                    self.index.spawn(pos, side)?;
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            };
        }
    }
    fn delete_node(&mut self,offset: i64) -> Result<(),Error> {
        // The big idea here is to delete the node without having to cut a whole branch.
        // If p has only one branch, this is easy, the next node down replaces p.
        // If p has two branches, and the left branch has no right branch, then p's right branch
        // moves down to become the left branch's right branch.  The left branch moves up to replace p.
        // If p has two branches, and the left branch branches right, we go down on the right as deep
        // as possible.  The deepest node is brought up to replace p, see below.
        let p = self.dictionary.get_pos(offset);
        if self.index.is_free(p)? {
            return Ok(());
        }
        self.index.set_cursor(p)?;
        // first assemble the branch that will replace p
        let replacement = match self.index.get_down()? {
            [None,None] => {
                return self.index.drop();
            },
            [Some(repl),None] => repl, // only 1 branch, it moves up to replace p
            [None,Some(repl)] => repl, // only 1 branch, it moves up to replace p
            [Some(left),Some(right)] => {
                // There are 2 branches, we have to rearrange things to avoid losing data.
                self.index.set_cursor(left)?;
                match self.index.get_down()? {
                    [_,None] => {
                        // Left branch does not branch right.
                        // Therefore we can simply attach the right branch to left branch's right branch.
                        // The updated left branch will be the replacement.
                        self.index.set_cursor(right)?;
                        self.index.move_node(left, Side::Right)?;
                        left
                    },
                    [_,Some(_)] => {
                        // The left branch branches right, find the terminus on the right.
                        // A right-terminus is not necessarily a leaf, i.e., it can have a left branch.
                        let terminus: usize = self.index.terminus(Side::Right)?;
                        let (terminus_dad,_) = self.index.get_parent_and_side()?;
                        self.index.cut_upward()?;
                        // possible left branch of the terminus takes the former spot of the terminus
                        match self.index.get_down()? {
                            [Some(_),None] => {
                                self.index.down(Side::Left)?;
                                self.index.move_node(terminus_dad,Side::Right)?;
                            },
                            [None,None] => {},
                            _ => panic!("unexpected children")
                        }
                        // The 2 branches of p can now be attached to what was the terminus,
                        // whereas the terminus will be the replacement.
                        self.index.set_cursor(left)?;
                        self.index.move_node(terminus,Side::Left)?;
                        self.index.set_cursor(right)?;
                        self.index.move_node(terminus,Side::Right)?;
                        terminus
                    }
                }
            }
        };
        // Replace `p` with `replacement`
        self.index.set_cursor(p)?;
        if self.index.is_root()? {
            let symbol = self.index.get_symbol()?;
            self.index.set_cursor(replacement)?;
            self.index.move_node_and_replace_root(symbol)

        } else {
            let (parent,side) = self.index.get_parent_and_side()?;
            self.index.set_cursor(replacement)?;
            self.index.move_node_and_replace(parent,side)
        }
    }
}

/// Main compression function
pub fn compress(ibuf: &[u8]) -> Result<Vec<u8>,Error> {
    let mut byte_ptr: usize = 0;
    let mut ans = BitVec::new();
    let mut lzss = LZSS::new();
    let mut huff = AdaptiveHuffman::create(ibuf.to_vec(),256 + LOOKAHEAD - THRESHOLD);
    huff.start_huff();
    // 32 bit header with length of expanded data
    let mut textsize = BitVec::from_bytes(&u32::to_le_bytes(ibuf.len() as u32));
    ans.append(&mut textsize);
    // setup dictionary
    let start_pos = WIN_SIZE - LOOKAHEAD;
    for i in 0..start_pos {
        lzss.dictionary.set(i as i64,b' ');
    }
    let mut len = 0;
    lzss.dictionary.set_pos(start_pos);
    while len < LOOKAHEAD {
        if ibuf.len() <= len {
            break;
        }
        let c = ibuf[len];
        lzss.dictionary.set(len as i64,c);
        len += 1;
        byte_ptr += 1;
    }
    for _i in 1..=LOOKAHEAD {
        lzss.dictionary.retreat();
        lzss.insert_node()?;
    }
    lzss.dictionary.set_pos(start_pos);
    lzss.insert_node()?;
    // main compression loop
    loop {
        if lzss.match_length > len {
            lzss.match_length = len;
        }
        if lzss.match_length <= THRESHOLD {
            lzss.match_length = 1;
            huff.encode_char(lzss.dictionary.get(0) as u16,&mut ans);
        } else {
            huff.encode_char((255-THRESHOLD+lzss.match_length) as u16,&mut ans);
            huff.encode_position(lzss.match_offset as u16,&mut ans);
        }
        let last_match_length = lzss.match_length;
        let mut i = 0;
        while i < last_match_length {
            let c: u8;
            if byte_ptr < ibuf.len() {
                c = ibuf[byte_ptr];
                byte_ptr += 1;
            } else {
                break;
            }
            lzss.delete_node(LOOKAHEAD as i64)?;
            lzss.dictionary.set(LOOKAHEAD as i64,c);
            lzss.dictionary.advance();
            lzss.insert_node()?;
            i += 1;
        }
        while i < last_match_length {
            lzss.delete_node(LOOKAHEAD as i64)?;
            lzss.dictionary.advance();
            len -= 1;
            if len > 0 {
                lzss.insert_node()?;
            }
            i += 1;
        }
        if len <= 0 {
            break;
        }
    }
    Ok(ans.to_bytes())
}

/// Main decompression function
pub fn expand(ibuf: &[u8]) -> Vec<u8>
{
    let mut ans = Vec::new();
    let mut huff = AdaptiveHuffman::create(ibuf.to_vec(),256 + LOOKAHEAD - THRESHOLD);
    let mut lzss= LZSS::new();
	if ibuf.len() == 0 {
		return ans;
    }
	huff.start_huff();
    let start_pos = WIN_SIZE - LOOKAHEAD;
	for i in 0..start_pos {
		lzss.dictionary.set(i as i64,b' ');
    }
    lzss.dictionary.set_pos(start_pos);
    // get size of expanded data from header
    let textsize = u32::from_le_bytes([ibuf[0],ibuf[1],ibuf[2],ibuf[3]]);
    huff.advance(32);
    // start expanding
	while ans.len() < textsize as usize {
    //while huff.ptr < huff.bits.len() {
		let c = huff.decode_char();
		if c < 256 {
            ans.push(c as u8);
			lzss.dictionary.set(0,c as u8);
            lzss.dictionary.advance();
		} else {
			let offset = - (huff.decode_position() as i64 + 1);
			let strlen = c as i64 + THRESHOLD as i64 - 255;
			for _k in 0..strlen {
				let c8 = lzss.dictionary.get(offset);
                ans.push(c8);
                lzss.dictionary.set(0,c8 as u8);
                lzss.dictionary.advance();
            }
		}
	}
    ans
}

#[test]
fn compression_works() {
    let test_data = "12345123456789123456789\n".as_bytes();
    let lzhuf_str = "18 00 00 00 DE EF B7 FC 0E 0C 70 13 85 C3 E2 71 64 81 19 60";
    let compressed = compress(test_data).expect("compression failed");
    assert_eq!(compressed,hex::decode(lzhuf_str.replace(" ","")).unwrap());

    let test_data = "I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let lzhuf_str = "31 00 00 00 EA EB 3D BF 9C 4E FE 1E 16 EA 34 09 1C 0D C0 8C 02 FC 3F 77 3F 57 20 17 7F 1F 5F BF C6 AB 7F A5 AF FE 4C 39 96";
    let compressed = compress(test_data).expect("compression failed");
    assert_eq!(compressed,hex::decode(lzhuf_str.replace(" ","")).unwrap());
}

#[test]
fn invertibility() {
    let test_data = "I am Sam. Sam I am. I do not like this Sam I am.\n".as_bytes();
    let compressed = compress(test_data).expect("compression failed");
    let expanded = expand(&compressed);
    assert_eq!(test_data.to_vec(),expanded);

    let test_data = "1234567".as_bytes();
    let compressed = compress(test_data).expect("compression failed");
    let expanded = expand(&compressed);
    assert_eq!(test_data.to_vec(),expanded[0..7]);
}