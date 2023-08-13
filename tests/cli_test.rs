use assert_cmd::prelude::*; // Add methods on commands
use std::path::Path;
use std::process::Command; // Run programs
use tempfile;
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

fn compress_test(base_name: &str,xext: &str,cext: &str,method: &str) -> STDRESULT {
    let mut cmd = Command::cargo_bin("retrocompressor")?;
    let in_path = Path::new("tests").join([base_name,".",xext].concat());
    let cmp_path = Path::new("tests").join([base_name,".",cext].concat());
    let temp_dir = tempfile::tempdir()?;
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
    let in_path = Path::new("tests").join([base_name,".",cext].concat());
    let cmp_path = Path::new("tests").join([base_name,".",xext].concat());
    let temp_dir = tempfile::tempdir()?;
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
