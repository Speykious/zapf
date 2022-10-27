use std::fs::File;
use std::io::{Write, self};
use std::path::Path;

// Ideally, it should be given a vector of (fs::Files) and it
// Reads all the given files and stores their parent agnostic
// (aka no prefix) paths, size and position in the combined file
// in its header.
// The writing of said files to the combined file shall be async in nature

pub fn combine_files<W: Write>(
    root_path: impl AsRef<Path>,
    paths: &[impl AsRef<Path>],
    writer: &mut W,
) -> io::Result<()> {
    for path in paths {
        let path = path.as_ref();
        let file = File::open(&path)?;

        let path = path.strip_prefix(&root_path).unwrap();
    }

    Ok(())
}
