use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use thiserror::Error;
use walkdir::WalkDir;

const MAGIC: &[u8; 4] = b"INFS";

#[derive(Error, Debug)]
pub enum ArchiveError {
  #[error("Path too long: {path} ({len} bytes, max {max} bytes)")]
  PathTooLong {
    path: String,
    len: usize,
    max: usize,
  },

  #[error("Too many files: {count} (max {max})")]
  TooManyFiles { count: usize, max: usize },

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Source directory must be a subdirectory of the manifest directory")]
  InvalidSourceDirectory,

  #[error("Failed to collect files: {0}")]
  WalkDir(#[from] walkdir::Error),
}

#[derive(Error, Debug)]
pub enum FsError {
  #[error("File not found")]
  NotFound,

  #[error("Invalid archive")]
  InvalidArchive,
}

#[derive(Debug)]
struct FileEntry {
  pub path: PathBuf,
  pub size: u64,
}

impl FileEntry {
  pub fn new(path: impl Into<PathBuf>, size: u64) -> Self {
    Self {
      path: path.into(),
      size,
    }
  }
}

fn compute_header(files: &[FileEntry]) -> Result<Vec<u8>, ArchiveError> {
  // Validate file count fits in u32
  if files.len() > u32::MAX as usize {
    return Err(ArchiveError::TooManyFiles {
      count: files.len(),
      max: u32::MAX as usize,
    });
  }

  let mut header_size = 4 + 4; // magic + file count
  for file in files {
    let path_str = file.path.to_string_lossy();
    let path_len = path_str.len();

    if path_len > u16::MAX as usize {
      return Err(ArchiveError::PathTooLong {
        path: path_str.to_string(),
        len: path_str.len(),
        max: u16::MAX as usize,
      });
    }

    // path_len + path + size + offset
    header_size += 2 + path_len + 8 + 8;
  }

  let mut header = Vec::with_capacity(header_size);

  header.extend_from_slice(MAGIC);
  header.extend_from_slice(&(files.len() as u32).to_le_bytes());

  let mut data_offset = header_size as u64;
  for file in files {
    let path_str = file.path.to_string_lossy();
    let path_bytes = path_str.as_bytes();

    header.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
    header.extend_from_slice(path_bytes);
    header.extend_from_slice(&file.size.to_le_bytes());
    header.extend_from_slice(&data_offset.to_le_bytes());

    data_offset += file.size;
  }

  Ok(header)
}

fn write_archive(files: &[FileEntry], output_path: &Path) -> Result<(), ArchiveError> {
  let mut file = File::create(output_path)?;

  // Write header
  let header = compute_header(files)?;
  file.write_all(&header)?;

  // Write file data
  for file_entry in files {
    let mut f = File::open(&file_entry.path)?;
    io::copy(&mut f, &mut file)?;
  }

  Ok(())
}

pub fn embed_fs(source_dir: &str, name: &str) -> Result<(), ArchiveError> {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("no CARGO_MANIFEST_DIR");
  let source_dir = Path::new(&manifest_dir).join(source_dir).canonicalize()?;

  // Ensure the source directory is a subdirectory of the manifest directory
  if !source_dir.starts_with(&manifest_dir) {
    return Err(ArchiveError::InvalidSourceDirectory);
  }

  let relative_source_dir = source_dir.strip_prefix(&manifest_dir).unwrap();
  println!("cargo:rerun-if-changed={}", relative_source_dir.display());

  let mut files = Vec::new();
  let walk = WalkDir::new(&source_dir).follow_links(false);
  for entry in walk {
    let entry = entry?;
    let meta = entry.metadata()?;
    if !meta.is_file() {
      continue;
    }

    let path = entry.path().strip_prefix(&manifest_dir).unwrap();
    files.push(FileEntry::new(path, meta.len()));
  }

  let out_dir = env::var("OUT_DIR").expect("no OUT_DIR");
  let output_file = format!("{}.embed_fs", name);
  let output_path = Path::new(&out_dir).join(output_file);

  write_archive(&files, &output_path)
}

pub struct FsEntry {
  pub path: String,
  pub size: u64,
  data_offset: u64,
}

impl FsEntry {
  pub fn new(path: String, size: u64, data_offset: u64) -> Self {
    Self {
      path,
      size,
      data_offset,
    }
  }
}

pub type IncludeFs = LazyLock<IncludeFsInner>;

pub struct IncludeFsInner {
  pub file_index: HashMap<String, FsEntry>,
  pub archive_bytes: Vec<u8>,
}

impl IncludeFsInner {
  pub fn new(archive_bytes: &[u8]) -> Result<Self, FsError> {
    if &archive_bytes[0..4] != MAGIC {
      return Err(FsError::InvalidArchive);
    }

    let file_count = u32::from_le_bytes([
      archive_bytes[4],
      archive_bytes[5],
      archive_bytes[6],
      archive_bytes[7],
    ]) as usize;

    let mut offset = 8;
    let mut file_index = HashMap::with_capacity(file_count);

    for _ in 0..file_count {
      let path_len =
        u16::from_le_bytes([archive_bytes[offset], archive_bytes[offset + 1]]) as usize;
      offset += 2;

      let path = String::from_utf8_lossy(&archive_bytes[offset..offset + path_len]).to_string();
      offset += path_len;

      let size = u64::from_le_bytes([
        archive_bytes[offset],
        archive_bytes[offset + 1],
        archive_bytes[offset + 2],
        archive_bytes[offset + 3],
        archive_bytes[offset + 4],
        archive_bytes[offset + 5],
        archive_bytes[offset + 6],
        archive_bytes[offset + 7],
      ]);
      offset += 8;

      let data_offset = u64::from_le_bytes([
        archive_bytes[offset],
        archive_bytes[offset + 1],
        archive_bytes[offset + 2],
        archive_bytes[offset + 3],
        archive_bytes[offset + 4],
        archive_bytes[offset + 5],
        archive_bytes[offset + 6],
        archive_bytes[offset + 7],
      ]);
      offset += 8;

      file_index.insert(path.clone(), FsEntry::new(path, size, data_offset));
    }

    Ok(Self {
      file_index,
      archive_bytes: archive_bytes.to_vec(),
    })
  }

  pub fn exists(&self, path: &str) -> bool {
    self.file_index.contains_key(path)
  }

  pub fn get(&self, path: &str) -> Result<&[u8], FsError> {
    let Some(entry) = self.file_index.get(path) else {
      return Err(FsError::NotFound);
    };

    let start = entry.data_offset as usize;
    let end = start + entry.size as usize;
    Ok(&self.archive_bytes[start..end])
  }

  pub fn list_paths(&self) -> Vec<&str> {
    self.file_index.keys().map(|s| s.as_str()).collect()
  }
}

#[macro_export]
macro_rules! include_fs {
  ($name:expr) => {
    ::std::sync::LazyLock::new(|| {
      let archive_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".embed_fs"));
      ::include_fs::IncludeFsInner::new(archive_bytes).expect("Failed to initialize IncludeFs")
    })
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compute_header() {
    let files = vec![
      FileEntry::new("src/main.rs", 1024),
      FileEntry::new("assets/image.png", 2048),
    ];

    let header = compute_header(&files).unwrap();

    // Verify magic
    assert_eq!(&header[0..4], b"INFS");

    // Verify file count
    let file_count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    assert_eq!(file_count, 2);

    // Basic size check (exact calculation depends on path lengths)
    let expected_min_size = 4 + 4 + // magic + count
      2 + "src/main.rs".len() + 8 + 8 + // first file
      2 + "assets/image.png".len() + 8 + 8; // second file

    assert_eq!(header.len(), expected_min_size);
  }

  #[test]
  fn test_path_too_long() {
    let long_path = "a".repeat(u16::MAX as usize + 1);
    let files = vec![FileEntry::new(long_path.clone(), 100)];

    let result = compute_header(&files);
    assert!(matches!(result, Err(ArchiveError::PathTooLong { .. })));

    if let Err(ArchiveError::PathTooLong { path, len, max }) = result {
      assert_eq!(path, long_path);
      assert_eq!(len, u16::MAX as usize + 1);
      assert_eq!(max, u16::MAX as usize);
    }
  }
}
