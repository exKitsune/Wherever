use std::process::{exit, Command};

fn main() {
    Command::new("wasm-pack")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(".")
        .current_dir("../web/")
        .status()
        .unwrap();
}
