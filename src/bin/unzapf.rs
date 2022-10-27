use clap::Parser;
use std::fs;
use std::io;
use std::path::PathBuf;
use zapf::unpack_files;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    pub file: PathBuf,
    pub output: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let out_path = cli
        .output
        .unwrap_or_else(|| PathBuf::default().join(cli.file.file_name().unwrap()));
    dbg!(&out_path);

    fs::create_dir_all(&out_path)?;
    unpack_files(&cli.file, &out_path)?;

    Ok(())
}
