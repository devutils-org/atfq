// `include_dir!` only tracks files that existed at the last build. Watching the
// whole directory makes a new or deleted card trigger a rebuild too.
fn main() {
    println!("cargo:rerun-if-changed=content");
}
