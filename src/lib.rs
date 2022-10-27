use log::*;
use walkdir::WalkDir;

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};

// Ideally, it should be given a vector of (fs::Files) and it
// Reads all the given files and stores their parent agnostic
// (aka no prefix) paths, size and position in the combined file
// in its header.
// The writing of said files to the combined file shall be async in nature

const HEAD: u32 = (2 * u32::BITS + usize::BITS) / 8;
const META: u32 = (3 * u64::BITS) / 8;
const MAGIC: &[u8] = "ZAPF".as_bytes();
const VERSION_MAJOR_STR: &str = env!("CARGO_PKG_VERSION_MAJOR");

#[derive(Clone, Debug, PartialEq, Eq)]
struct ZapFileMeta {
    pub path_index: u64,
    pub size: u64,
    pub content_index: u64,
}

pub fn pack_files<W: Write>(root_path: impl AsRef<Path>, writer: &mut W) -> io::Result<()> {
    let root_path = root_path.as_ref();
    let paths: Vec<PathBuf> = {
        let mut paths = Vec::new();
        for entry in WalkDir::new(&root_path) {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            paths.push(path.to_owned());
        }
        paths
    };

    // magic number
    let mut writer = BufWriter::new(writer);
    writer.write_all(MAGIC)?;

    // version
    let zapf_version = VERSION_MAJOR_STR.parse::<u32>().unwrap();
    writer.write_all(&zapf_version.to_le_bytes())?;

    // number of files
    writer.write_all(&paths.len().to_le_bytes())?;

    // get list of paths but stripped from root path
    let stripped_paths = {
        let mut stripped_paths = Vec::with_capacity(paths.len());
        for path in &paths {
            let path = path.strip_prefix(&root_path).unwrap();
            stripped_paths.push(path.to_owned());
        }
        stripped_paths
    };

    let len_header = HEAD as u64 + paths.len() as u64 * META as u64;

    // get path index and content index from sum of all path lengths
    let mut path_idx = len_header;
    let mut content_idx = {
        let mut content_idx = len_header as u64;
        for path in &stripped_paths {
            content_idx += (u16::BITS / 8) as u64 + path.as_os_str().len() as u64;
        }
        content_idx
    };

    // write list of file metadata
    info!("Writing list of file metadata");
    for (path, stripped) in paths.iter().zip(&stripped_paths) {
        let size = File::open(&path)?.metadata()?.len();
        debug!("- {}", stripped.display());

        // file metadata
        writer.write_all(&path_idx.to_le_bytes())?;
        writer.write_all(&size.to_le_bytes())?;
        writer.write_all(&content_idx.to_le_bytes())?;

        path_idx += (u16::BITS / 8) as u64 + stripped.as_os_str().len() as u64;
        content_idx += size;
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
    for (path, stripped) in paths.iter().zip(&stripped_paths) {
        debug!("- {}", stripped.display());

        // TODO: Multithread this.
        // dump all content of the file into the writer
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        io::copy(&mut reader, &mut writer)?;
    }

    Ok(())
}

pub fn unpack_files(
    packed_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let packed_path = packed_path.as_ref();
    let output_path = output_path.as_ref();

    // get file
    let mut packed_file: File = File::open(&packed_path)?;
    debug!("ZAP file size: {:#?}", packed_file.metadata()?.len());

    // read the header to the buf
    let mut buf: [u8; HEAD as usize] = [0; HEAD as usize];
    packed_file.read_exact(&mut buf)?;
    
    // verify magic number
    let magic_number = &buf[..4];
    assert_eq!(magic_number, MAGIC);
    // panic!("{} is not a zap file.", packed_path.as_ref().display());

    // verify zap file format version
    let zapf_version = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let expected_zapf_version = VERSION_MAJOR_STR.parse::<u32>().unwrap();
    assert_eq!(zapf_version, expected_zapf_version);

    // get number of files
    let num_files = usize::from_le_bytes(buf[8..16].try_into().unwrap());

    // get zap metadata
    let zap_metas = {
        let mut zap_metas = Vec::new();
        for _ in 0..num_files {
            // read metadata from file (3 u64s)
            let mut metadata: [u8; META as usize] = [0; META as usize];
            packed_file.read_exact(&mut metadata)?;

            zap_metas.push(ZapFileMeta {
                path_index: u64::from_le_bytes(metadata[0..8].try_into().unwrap()),
                size: u64::from_le_bytes(metadata[8..16].try_into().unwrap()),
                content_index: u64::from_le_bytes(metadata[16..24].try_into().unwrap()),
            })
        }
        zap_metas
    };
    dbg!(&zap_metas);

    // get the path using the path index
    for meta in &zap_metas {
        let mut file_reader = BufReader::new(&packed_file);

        // go to the path index and read it
        file_reader.seek(io::SeekFrom::Start(meta.path_index))?;

        // get size of path string
        const SIZELEN: usize = (u16::BITS / 8) as usize;
        let mut path_size_buf: [u8; SIZELEN] = [0; SIZELEN];
        file_reader.read_exact(&mut path_size_buf)?;

        let path_size: usize = u16::from_le_bytes(path_size_buf) as usize;
        let mut path_vec: Vec<u8> = vec![0; path_size];
        file_reader.read_exact(&mut path_vec)?;

        let path_string = String::from_utf8(path_vec).unwrap();
        dbg!(&path_string);

        // read from the old/write the contents into a new file
        let path = Path::new(&path_string).to_owned();
        let path = output_path.join(path);
        // it's slow af, didn't finish yet
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file_write = File::create(path)?;
        let mut file_writer = BufWriter::new(&file_write);

        file_reader.seek(io::SeekFrom::Start(meta.content_index))?;
        let mut file_reader = file_reader.take(meta.size);
        io::copy(&mut file_reader, &mut file_writer)?;
    }

    // high five o/\o HIGH FIIIIVVVEEE!!!!
    Ok(())
}
