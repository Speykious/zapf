use clap::Parser;
use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::PathBuf;
use zapf::pack_files;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    pub folder: PathBuf,
    pub output: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let folder = fs::canonicalize(&cli.folder)?;

    println!("Folder: {}", folder.display());

    let out_path = cli
        .output
        .unwrap_or_default()
        .join(folder.file_name().unwrap())
        .with_extension("zap");
    dbg!(&out_path);
    let out_file = File::create(&out_path)?;
    let mut out_writer = BufWriter::new(out_file);

    pack_files(&folder, &mut out_writer)?;

    Ok(())
}
