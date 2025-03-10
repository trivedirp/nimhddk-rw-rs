extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // println!("cargo:rustc-link-search=native=/home/drew/Repos/libmhddk/build");
    println!("cargo:rustc-link-search=native=/home/rahul/Repos/libmhddk/build");
    println!("cargo:rustc-link-lib=mhddk");
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // .clang_arg("-I/home/drew/Repos/libmhddk/src")
        .clang_arg("-I/home/rahul/Repos/libmhddk/src")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .blocklist_type("wchar_t")
        .blocklist_type("max_align_t")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
