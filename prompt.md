# Rust Directory Embedding Library

## Project Overview

This is a Rust library that allows embedding entire directories into binaries at compile time, similar to how `include_bytes!` and `include_str!` work for single files. The library packages directories into a custom binary archive format, embeds the archive using `include_bytes!`, and provides runtime extraction capabilities.

## Architecture

### Build-Time Process
1. A build script recursively walks a specified directory
2. Files are packaged into a custom binary archive format
3. The archive is written to `OUT_DIR` 
4. The archive is embedded into the binary using `include_bytes!`

### Runtime Process
1. The embedded archive bytes are parsed
2. A file index is built from the archive header
3. Individual files can be extracted on-demand without loading the entire archive into memory

## Custom Binary Archive Format

The archive uses a simple binary format:

### Layout Structure
```
[Header]
[File Data Section - concatenated file contents]
```

### Header Format
```
Magic Number:     4 bytes  (b"INFS")
File Count:       4 bytes  (u32, little-endian)

For each file:
  Path Length:    2 bytes  (u16, little-endian)
  Path:          variable  (UTF-8 string)
  File Size:      8 bytes  (u64, little-endian) 
  Data Offset:    8 bytes  (u64, little-endian)
```

### Design Decisions
- **No compression**: Keeps implementation simple and allows random access
- **No metadata**: Timestamps, permissions, etc. are not stored
- **Little-endian**: Standard for most target platforms
- **Lexicographic sorting**: Files are sorted by path for deterministic builds and binary search capability
- **8-byte sizes/offsets**: Supports files and archives up to 16 exabytes
- **2-byte path length**: Supports paths up to 65,535 bytes

## Key Requirements

- Pure Rust implementation (no external tools or system dependencies)
- Compile-time directory embedding via build scripts
- Runtime file extraction without holding entire archive in memory
- No file metadata storage needed
- Deterministic archive generation for reproducible builds
- Uses `include_bytes!` for embedding (requires file written to disk at build time)
- Build script writes archive to `env!("OUT_DIR")`

## Error Handling

Uses `thiserror` for structured error handling:

```rust
#[derive(Error, Debug)]
pub enum ArchiveError {
  #[error("Path too long: {path} ({len} bytes, max {max} bytes)")]
  PathTooLong { path: String, len: usize, max: usize },
  
  #[error("Too many files: {count} (max {max})")]
  TooManyFiles { count: usize, max: usize },
}
```

## Current Implementation Status

### Completed
- Binary archive format specification
- Header generation function with proper error handling
- File entry structure (`FileEntry`)
- Basic validation (path length, file count limits)

## Usage Pattern

### Build Script (`build.rs`)
```rust
fn main() {
  embed_directory("src/assets", "assets").unwrap();
}
```

### Runtime Usage
```rust
static ASSETS: IncludeFs = include_fs!("assets");

fn main() {
  let image_exists = ASSETS.exists("image.jpg");
  let config_content = ASSETS.get("config.toml").unwrap();
}
```

## Dependencies

- `thiserror` - Structured error handling
- Standard library only (no external runtime dependencies)

## Development Notes

- Consider using `std::collections::HashMap` for file index at runtime
- File paths should use forward slashes internally for cross-platform consistency
- Archive format is designed to be forward-compatible (version field can be added to header if needed)
- Binary search optimization possible due to sorted file table
- Memory usage is O(number of files) for index, O(1) for file access