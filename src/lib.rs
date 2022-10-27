use log::*;

use std::fs::{File, self};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

// Ideally, it should be given a vector of (fs::Files) and it
// Reads all the given files and stores their parent agnostic
// (aka no prefix) paths, size and position in the combined file
// in its header.
// The writing of said files to the combined file shall be async in nature

pub fn pack_files<W: Write>(
    root_path: impl AsRef<Path>,
    paths: &[impl AsRef<Path>],
    writer: &mut W,
) -> io::Result<()> {
    let root_path = root_path.as_ref();

    // magic number
    let mut writer = BufWriter::new(writer);
    writer.write_all("ZAPF".as_bytes())?;

    // version
    let zapf_version = env!("CARGO_PKG_VERSION_MAJOR")
        .parse::<u32>()
        .unwrap()
        .to_le_bytes();
    writer.write_all(&zapf_version)?;

    // number of files
    writer.write_all(&paths.len().to_le_bytes())?;

    let mut stripped_paths = Vec::with_capacity(paths.len());
    let len_header = 3 * u32::BITS + paths.len() as u32 * 3 * u32::BITS;

    // get path index and content index from sum of all path lengths
    let mut path_idx = len_header;
    let mut content_idx = {
        let mut content_idx = len_header as u64;
        for path in paths {
            let path = path.as_ref();
            content_idx += u16::BITS as u64 + path.as_os_str().len() as u64;
        }
        content_idx
    };

    // write list of file metadata
    info!("Writing list of file metadata");
    for path in paths {
        let path = path.as_ref();
        let size = File::open(&path)?.metadata()?.len();

        let path = fs::canonicalize(&path)?;
        dbg!(&path);
        dbg!(&root_path);
        let path = path.strip_prefix(&root_path).unwrap();
        debug!("- {}", path.display());

        // file metadata
        writer.write_all(&path_idx.to_le_bytes())?;
        writer.write_all(&size.to_le_bytes())?;
        writer.write_all(&content_idx.to_le_bytes())?;

        path_idx += u16::BITS + path.as_os_str().len() as u32;
        content_idx += size;
        stripped_paths.push(path.to_owned());
    }

    // write file paths
    info!("Writing file paths");
    for path in &stripped_paths {
        debug!("- {}", path.display());

        // path length
        let path_len: u16 = path.as_os_str().len() as u16;
        writer.write_all(&path_len.to_le_bytes())?;

        // path string
        let path_string = path.display().to_string();
        writer.write_all(path_string.as_bytes())?;
    }

    // write file contents
    info!("Writing file contents");
    for (path, stripped) in paths.iter().zip(stripped_paths) {
        let path = path.as_ref();
        debug!("- {}", stripped.display());

        // TODO: Multithread this.
        // dump all content of the file into the writer
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        io::copy(&mut reader, &mut writer)?;
    }

    Ok(())
}
