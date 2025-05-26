fn main() {
  include_fs::embed_fs("src", "source").expect("embed source dir");
}
