# include-fs

`include-fs` allows embedding entire directories into binaries at compile time, similar to how `include_bytes!` and `include_str!` work for single files. The library packages directories into an archive, embeds the archive using `include_bytes!`, and allows accessing the files by path at runtime.

## Usage

### Build Script (`build.rs`)

```rust
fn main() {
  include_fs::embed_fs("src/assets", "assets").unwrap();
}
```

### Runtime Usage

```rust
static ASSETS: IncludeFs = include_fs!("assets");

fn main() {
  let image_exists = ASSETS.exists("assets/image.jpg");
  let config_content = ASSETS.get("assets/config.toml").unwrap();
}
```

## Planned Features

- Improved API with better error messages
- Feature-flagged glob support
- Directory listing

## Archive Format

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
- **Lexicographic sorting**: Files are sorted by path for deterministic builds
- **8-byte sizes/offsets**: Supports files and archives up to 16 exabytes
- **2-byte path length**: Supports paths up to 65,535 bytes
