use assert_cmd::prelude::*; // Add methods on commands
use std::path::{PathBuf,Path};
use std::process::Command; // Run programs
use std::io::{BufReader, BufWriter, Read, ErrorKind, Write};
use tempfile;
type DYNERR = Box<dyn std::error::Error>;
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

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
                    writer.write_all(&mut tok.clone())?;
                }
                else if curr[0]!=10 {
                    writer.write_all(&mut curr.clone())?;
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
    let in_path = copy_and_fix_newlines(in_path_any_newline,&temp_dir, &[13,10])?;
    let cmp_path = Path::new("tests").join([base_name,".",cext].concat());
    let out_path = temp_dir.path().join([base_name,".",cext].concat());
    cmd.arg("compress")
        .arg("-m").arg(method)
        .arg("-i").arg(&in_path)
        .arg("-o").arg(&out_path)
        .assert()
        .success();
    match (std::fs::read(cmp_path),std::fs::read(out_path)) {
        (Ok(v1),Ok(v2)) => {
            assert_eq!(v1,v2);
        },
        _ => panic!("unable to compare output with reference")
    }
    Ok(())
}

fn expand_test(base_name: &str,xext: &str,cext: &str,method: &str) -> STDRESULT {
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    let temp_dir = tempfile::tempdir()?;
    let in_path = Path::new("tests").join([base_name,".",cext].concat());
    let cmp_path_any_newline = Path::new("tests").join([base_name,".",xext].concat());
    let cmp_path = copy_and_fix_newlines(cmp_path_any_newline, &temp_dir, &[13,10])?;
    let out_path = temp_dir.path().join([base_name,".",xext].concat());
    cmd.arg("expand")
        .arg("-m").arg(method)
        .arg("-i").arg(&in_path)
        .arg("-o").arg(&out_path)
        .assert()
        .success();
    match (std::fs::read(cmp_path),std::fs::read(out_path)) {
        (Ok(v1),Ok(v2)) => {
            assert_eq!(v1,v2);
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