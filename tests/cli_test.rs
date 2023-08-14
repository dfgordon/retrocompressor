use assert_cmd::prelude::*; // Add methods on commands
use std::path::{PathBuf,Path};
use std::process::Command; // Run programs
use tempfile;
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

// Make a copy in temporary directory with the specified newline token.
// This insulates us against newline substitutions inserted by git or other layers.
// The starting newline must either be LF or CRLF.
fn copy_and_fix_newlines(in_file: PathBuf,temp_dir: &tempfile::TempDir,tok: &[u8]) -> Result<PathBuf,Box<dyn std::error::Error>> {
    let txt = std::fs::read(in_file).expect("could not read input file");
    let mut new_txt: Vec<u8> = Vec::new();
    let mut last_char: u8 = 255;
    for i in 0..txt.len() {
        if txt[i]==13 || txt[i]==10 && last_char!=13 {
            new_txt.append(&mut tok.to_vec());
        }
        else if txt[i]!=10 {
            new_txt.push(txt[i]);
        }
        last_char = txt[i];
    }
    let new_txt_path = temp_dir.path().join("converted.txt");
    match std::fs::write(&new_txt_path,new_txt) {
        Ok(_) => Ok(new_txt_path),
        Err(e) => Err(Box::new(e))
    }
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

#[test]
fn lzhuf_port_compression() -> STDRESULT {
    compress_test("hamlet_act_1","txt","lzh","lzhuf-port")?;
    compress_test("tempest_act_5","txt","lzh","lzhuf-port")
}

#[test]
fn lzhuf_compression() -> STDRESULT {
    compress_test("hamlet_act_1","txt","lzh","lzhuf")?;
    compress_test("tempest_act_5","txt","lzh","lzhuf")
}

#[test]
fn lzhuf_port_expansion() -> STDRESULT {
    expand_test("hamlet_act_1","txt","lzh","lzhuf-port")?;
    expand_test("tempest_act_5","txt","lzh","lzhuf-port")
}

#[test]
fn lzhuf_expansion() -> STDRESULT {
    expand_test("hamlet_act_1","txt","lzh","lzhuf")?;
    expand_test("tempest_act_5","txt","lzh","lzhuf")
}
