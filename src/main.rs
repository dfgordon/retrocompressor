use clap::{arg,crate_version,Command};
use retrocompressor::{lzw,lzss_huff, td0, direct_ports};
type STDRESULT = Result<(),Box<dyn std::error::Error>>;

const RCH: &str = "unreachable was reached";

fn ok_to_overwrite(path_out: &str) -> bool {
    if let Ok(_f) = std::fs::File::open(path_out) {
        let mut ans = String::new();
        eprint!("{} exists, overwrite? (y/n) ",path_out);
        std::io::stdin().read_line(&mut ans).expect("could not read stdin");
        if ans.trim_end()=="y" || ans.trim_end()=="Y" {
            log::warn!("existing file will not be truncated");
            return true;
        }
        return false;
    }
    true
}

fn main() -> STDRESULT
{
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let long_help =
"Examples:
---------
Compress:      `retrocompressor compress -m lzss_huff -i my_compressed -o my_expanded`
Expand:        `retrocompressor expand -m lzss_huff -i my_expanded -o my_compressed`";

    let methods = ["lzw","lzhuf-port","lzss_huff","td0"];

    let mut main_cmd = Command::new("retrocompressor")
        .about("Compress and expand with retro formats")
        .after_long_help(long_help)
        .version(crate_version!());
    main_cmd = main_cmd.subcommand(Command::new("compress")
        .arg(arg!(-m --method <METHOD> "compression algorithm").value_parser(methods)
            .required(true))
        .arg(arg!(-i --input <PATH> "input path").required(true))
        .arg(arg!(-o --output <PATH> "output path").required(true))
        .about("compress a file"));

        main_cmd = main_cmd.subcommand(Command::new("expand")
        .arg(arg!(-m --method <METHOD> "compression algorithm").required(true))
        .arg(arg!(-i --input <PATH> "input path").required(true))
        .arg(arg!(-o --output <PATH> "output path").required(true))
        .about("expand a file"));

    let matches = main_cmd.get_matches();
    
    if let Some(cmd) = matches.subcommand_matches("compress") {
        let path_in = cmd.get_one::<String>("input").expect(RCH);
        let path_out = cmd.get_one::<String>("output").expect(RCH);
        let method = cmd.get_one::<String>("method").expect(RCH);
        if !ok_to_overwrite(path_out) {
            eprintln!("abort operation");
            return Ok(());
        }
        let mut in_file = std::fs::File::open(path_in)?;
        let mut out_file = std::fs::OpenOptions::new().write(true).truncate(false).create(true).open(path_out)?;
        let (in_size,out_size) = match method.as_str() {
            "lzw" => lzw::compress(&mut in_file,&mut out_file,&lzw::STD_OPTIONS)?,
            "lzhuf-port" => direct_ports::lzhuf::encode(&mut in_file,&mut out_file)?,
            "lzss_huff" => lzss_huff::compress(&mut in_file,&mut out_file,&lzss_huff::STD_OPTIONS)?,
            "td0" => td0::compress(&mut in_file,&mut out_file)?,
            _ => {
                eprintln!("{} not supported",method);
                return Err(Box::new(std::fmt::Error));
            }
        };
        out_file.set_len(out_size)?;
        eprintln!("compressed {} into {}",in_size,out_size);
    }

    if let Some(cmd) = matches.subcommand_matches("expand") {
        let path_in = cmd.get_one::<String>("input").expect(RCH);
        let path_out = cmd.get_one::<String>("output").expect(RCH);
        let method = cmd.get_one::<String>("method").expect(RCH);
        if !ok_to_overwrite(path_out) {
            eprintln!("abort operation");
            return Ok(());
        }
        let mut in_file = std::fs::File::open(path_in)?;
        let mut out_file = std::fs::OpenOptions::new().write(true).truncate(false).create(true).open(path_out)?;
        let (in_size,out_size) = match method.as_str() {
            "lzw" => lzw::expand(&mut in_file,&mut out_file,&lzw::STD_OPTIONS)?,
            "lzhuf-port" => direct_ports::lzhuf::decode(&mut in_file,&mut out_file)?,
            "lzss_huff" => lzss_huff::expand(&mut in_file,&mut out_file,&lzss_huff::STD_OPTIONS)?,
            "td0" => td0::expand(&mut in_file,&mut out_file)?,
            _ => {
                eprintln!("{} not supported",method);
                return Err(Box::new(std::fmt::Error));
            }
        };
        out_file.set_len(out_size)?;
        eprintln!("expanded {} into {}",in_size,out_size);
    }

    Ok(())   
}