use include_fs::{include_fs, IncludeFs};

static SOURCE: IncludeFs = include_fs!("source");

fn main() {
  println!("paths: {:?}", SOURCE.list_paths());

  // Check if a specific file exists in the embedded file system
  let file_exists = SOURCE.exists("example.txt");
  println!("Does 'example.txt' exist? {}", file_exists);

  // Retrieve the content of a file
  match SOURCE.get("src/main.rs") {
    Ok(content) => {
      println!("{}", String::from_utf8_lossy(content));
    }
    Err(e) => println!("Error retrieving 'example.txt': {}", e),
  }
}
