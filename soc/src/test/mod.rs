pub mod context;
pub mod image;

// mod integration;
//mod mooneye_suite;
mod util;

pub use context::*;

pub fn base_path_to(target: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    // let mut path = std::path::PathBuf::from("../");
    // path.push(target);
    // path
    ["../".as_ref(), target.as_ref()].iter().collect()
}
