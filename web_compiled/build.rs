use std::env;
use std::io;
use std::path::Path;
use std::process::{exit, Command};

fn main() -> io::Result<()> {
    let out_dir = Path::new(&env::var_os("OUT_DIR").unwrap()).join("web");
    std::fs::copy("../web/index.html", out_dir.join("index.html")).unwrap();
    let exit_code = Command::new("wasm-pack")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&out_dir)
        .current_dir("../web/")
        .status()?;
    exit(exit_code.code().unwrap_or(1))
    //println!(
    //    "cargo:rerun-if-changed={}",
    //    std::env::current_dir()
    //        .unwrap()
    //        .join("../web/src/")
    //        .to_str()
    //        .unwrap()
    //);
    //println!(
    //    "cargo:rerun-if-changed={}",
    //    std::env::current_dir()
    //        .unwrap()
    //        .join("../web/index.html")
    //        .to_str()
    //        .unwrap()
    //);
}
