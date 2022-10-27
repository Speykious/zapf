use clap::Parser;
use walkdir::WalkDir;
use zapf::pack_files;
use std::fs::{File, self};
use std::io::{self, BufWriter};
use std::path::{PathBuf};

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

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&cli.folder) {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        paths.push(path.to_owned());
    }

    let out_path = cli.output.unwrap_or_default().join(folder.file_name().unwrap());
    dbg!(&out_path);
    let out_file = File::create(&out_path)?;
    let mut out_writer = BufWriter::new(out_file);

    pack_files(&folder, &paths, &mut out_writer)?;

    Ok(())
}
