use assert_cmd::prelude::*; // Add methods on commands
use std::path::{PathBuf,Path};
use std::process::Command; // Run programs
use std::io::{BufReader, BufWriter, Read, ErrorKind, Write};
use tempfile;
type DYNERR = Box<dyn std::error::Error>;
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

/// When comparing to a reference sometimes we don't want to compare the whole slice.
/// This compares selected slices based on the the compression `method`, details within.
fn compare_slices(method: &str,ref_them: &Vec<u8>,out_us: &Vec<u8>) {
    match method {
        "td0" => {
            assert_eq!(ref_them[0..3],out_us[0..3]);
            // skip the signature byte
            assert_eq!(ref_them[4..10],out_us[4..10]);
            // skip the CRC (because signature byte could be different)
            if ref_them[4] < 20 {
                // for v1.x we can compare everything after the header
                assert_eq!(ref_them[12..],out_us[12..]);
            } else {
                // For v2.x there can be a few trailing bytes that Teledisk puts in for unknown reasons,
                // it may have to do with how the end of the LZHUF bitstream is handled.
                // We are choosing to panic if this difference exceeds a threshold.
                if i64::abs(ref_them.len() as i64 - out_us.len() as i64) > 10 {
                    panic!("TD0 sizes differ too much");
                };
                // Compare the rest of the bytes, ignoring extra bytes in one or the other,
                // we also back up one byte because of the complexities of what goes on at the end.
                let cmp_end = usize::min(ref_them.len(),out_us.len()) - 1;
                assert_eq!(ref_them[12..cmp_end],out_us[12..cmp_end]);
            }
        },
        _ => {
            assert_eq!(ref_them,out_us);
        }
    }
}

// Make a copy in temporary directory with the specified newline token.
// This insulates us against newline substitutions inserted by git or other layers.
// The starting newline must either be LF or CRLF.
fn copy_and_fix_newlines(in_file: PathBuf,temp_dir: &tempfile::TempDir,tok: &[u8]) -> Result<PathBuf,DYNERR> {
    let in_file = std::fs::File::open(in_file)?;
    let out_path = temp_dir.path().join("converted.txt");
    let out_file = std::fs::File::create(&out_path)?;
    let mut reader = BufReader::new(in_file);
    let mut writer = BufWriter::new(out_file);
    let mut prev: u8 = 255;
    let mut curr: [u8;1] = [0];
    loop {
        match reader.read_exact(&mut curr) {
            Ok(_) => {
                if curr[0]==13 || curr[0]==10 && prev!=13 {
                    writer.write_all(tok)?;
                }
                else if curr[0]!=10 {
                    writer.write_all(&curr)?;
                }
                prev = curr[0]
            }
            Err(e) if e.kind()==ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(Box::new(e))
        }
    }
    Ok(out_path)
}

fn compress_test(base_name: &str,xext: &str,cext: &str,method: &str) -> STDRESULT {
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    let temp_dir = tempfile::tempdir()?;
    let in_path_any_newline = Path::new("tests").join([base_name,".",xext].concat());
    let in_path = match method {
        "td0" => in_path_any_newline,
        _ => copy_and_fix_newlines(in_path_any_newline,&temp_dir, &[13,10])?
    };
    let ref_path_them = Path::new("tests").join([base_name,".",cext].concat());
    let out_path_us = temp_dir.path().join([base_name,".",cext].concat());
    cmd.arg("compress")
        .arg("-m").arg(method)
        .arg("-i").arg(&in_path)
        .arg("-o").arg(&out_path_us)
        .assert()
        .success();
    match (std::fs::read(ref_path_them),std::fs::read(out_path_us)) {
        (Ok(v1),Ok(v2)) => {
            compare_slices(method, &v1, &v2);
        },
        _ => panic!("unable to compare output with reference")
    }
    Ok(())
}

fn expand_test(base_name: &str,xext: &str,cext: &str,method: &str) -> STDRESULT {
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    let temp_dir = tempfile::tempdir()?;
    let in_path = Path::new("tests").join([base_name,".",cext].concat());
    let ref_path_any_newline = Path::new("tests").join([base_name,".",xext].concat());
    let ref_path_them = match method {
        "td0" => ref_path_any_newline,
        _ => copy_and_fix_newlines(ref_path_any_newline, &temp_dir, &[13,10])?
    };
    let out_path_us = temp_dir.path().join([base_name,".",xext].concat());
    cmd.arg("expand")
        .arg("-m").arg(method)
        .arg("-i").arg(&in_path)
        .arg("-o").arg(&out_path_us)
        .assert()
        .success();
    match (std::fs::read(ref_path_them),std::fs::read(out_path_us)) {
        (Ok(v1),Ok(v2)) => {
            compare_slices(method, &v1, &v2);
        },
        _ => panic!("unable to compare output with reference")
    }
    Ok(())
}

fn invertibility_test(file_name: &str,method: &str) -> STDRESULT {
    // newlines don't matter, no reference in this case
    let temp_dir = tempfile::tempdir()?;
    let in_path = Path::new("tests").join(file_name);
    let intermediate = temp_dir.path().join([file_name,"1"].concat());
    let out_path = temp_dir.path().join([file_name,"2"].concat());
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    cmd.arg("compress")
        .arg("-m").arg(method)
        .arg("-i").arg(&in_path)
        .arg("-o").arg(&intermediate)
        .assert()
        .success();
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    cmd.arg("expand")
        .arg("-m").arg(method)
        .arg("-i").arg(&intermediate)
        .arg("-o").arg(&out_path)
        .assert()
        .success();
    match (std::fs::read(in_path),std::fs::read(out_path)) {
        (Ok(v1),Ok(v2)) => {
            assert_eq!(v1,v2);
        },
        _ => panic!("unable to compare files")
    }
    Ok(())
}

#[test]
fn lzhuf_port_compression() -> STDRESULT {
    compress_test("hamlet_act_1","txt","lzh","lzhuf-port")?;
    compress_test("tempest_act_5","txt","lzh","lzhuf-port")
}

#[test]
fn lzss_huff_compression() -> STDRESULT {
    compress_test("hamlet_act_1","txt","lzh","lzss_huff")?;
    compress_test("tempest_act_5","txt","lzh","lzss_huff")
}

#[test]
fn lzhuf_port_expansion() -> STDRESULT {
    expand_test("hamlet_act_1","txt","lzh","lzhuf-port")?;
    expand_test("tempest_act_5","txt","lzh","lzhuf-port")
}

#[test]
fn lzss_huff_expansion() -> STDRESULT {
    expand_test("hamlet_act_1","txt","lzh","lzss_huff")?;
    expand_test("tempest_act_5","txt","lzh","lzss_huff")
}

#[test]
fn lzhuf_port_invertibility() -> STDRESULT {
    invertibility_test("hamlet_full.txt", "lzhuf-port")?;
    invertibility_test("shkspr.dsk", "lzhuf-port")
}

#[test]
fn lzss_huff_invertibility() -> STDRESULT {
    invertibility_test("hamlet_full.txt", "lzss_huff")?;
    invertibility_test("shkspr.dsk", "lzss_huff")
}

#[test]
fn teledisk_compression() -> STDRESULT {
    compress_test("td105","norm.td0","adv.td0","td0")?;
    compress_test("td215","norm.td0","adv.td0","td0")
}

#[test]
fn teledisk_expansion() -> STDRESULT {
    expand_test("td105","norm.td0","adv.td0","td0")?;
    expand_test("td215","norm.td0","adv.td0","td0")
}