use std::{env, process::Command};
use build_helper::rustc::{link_lib, link_search};

// This script actually builds Unicorn using "make", unlike the actual bindings which already expect it to be built...

const UNICORN_PATH: &str = "../unicorn";
const UNICORN_MAKE_PARAMS: &[&str] = &["UNICORN_ARCHS=aarch64", "UNICORN_SHARED=no"];

fn main() {
    let _ = Command::new("make")
        .current_dir(UNICORN_PATH)
        .args(UNICORN_MAKE_PARAMS)
        .status()
        .unwrap();
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let unicorn = "libunicorn.a";
    let _ = Command::new("cp")
        .current_dir(UNICORN_PATH)
        .arg(&unicorn)
        .arg(&out_dir)
        .status()
        .unwrap();

    link_search(Some(build_helper::SearchKind::Native), build_helper::out_dir());
    link_lib(Some(build_helper::LibKind::Static), "unicorn");
}
