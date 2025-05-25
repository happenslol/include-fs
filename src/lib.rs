use thiserror::Error;

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
}

#[derive(Debug)]
pub struct FileEntry {
  pub path: String,
  pub size: u64,
  pub data_offset: u64,
}

impl FileEntry {
  pub fn new(path: impl Into<String>, size: u64, data_offset: u64) -> Self {
    Self {
      path: path.into(),
      size,
      data_offset,
    }
  }
}

pub fn compute_header(files: &[FileEntry]) -> Result<Vec<u8>, ArchiveError> {
  // Validate file count fits in u32
  if files.len() > u32::MAX as usize {
    return Err(ArchiveError::TooManyFiles {
      count: files.len(),
      max: u32::MAX as usize,
    });
  }

  // Calculate total header size to pre-allocate
  let mut header_size = 4 + 4; // magic + file count
  for file in files {
    let path_len = file.path.len();
    if path_len > u16::MAX as usize {
      return Err(ArchiveError::PathTooLong {
        path: file.path.clone(),
        len: path_len,
        max: u16::MAX as usize,
      });
    }

    // path_len + path + size + offset
    header_size += 2 + path_len + 8 + 8;
  }

  let mut header = Vec::with_capacity(header_size);

  header.extend_from_slice(MAGIC);
  header.extend_from_slice(&(files.len() as u32).to_le_bytes());

  for file in files {
    let path_bytes = file.path.as_bytes();

    // Path length (2 bytes) - already validated above
    header.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());

    // UTF-8 path
    header.extend_from_slice(path_bytes);

    // File size (8 bytes)
    header.extend_from_slice(&file.size.to_le_bytes());

    // Data offset (8 bytes)
    header.extend_from_slice(&file.data_offset.to_le_bytes());
  }

  Ok(header)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compute_header() {
    let files = vec![
      FileEntry::new("src/main.rs", 1024, 0),
      FileEntry::new("assets/image.png", 2048, 1024),
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
    let files = vec![FileEntry::new(long_path.clone(), 100, 0)];

    let result = compute_header(&files);
    assert!(matches!(result, Err(ArchiveError::PathTooLong { .. })));

    if let Err(ArchiveError::PathTooLong { path, len, max }) = result {
      assert_eq!(path, long_path);
      assert_eq!(len, u16::MAX as usize + 1);
      assert_eq!(max, u16::MAX as usize);
    }
  }
}
