//! Builds the web frontend in `web/` into `OUT_DIR/web`, from where the crate
//! embeds it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Paths in `web/` that the web frontend's build depends on.
const WEB_INPUTS: [&str; 8] = [
    "index.html",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "public",
    "src",
    "tsconfig.json",
    "vite.config.ts",
];

fn main() {
    let web_dir = Path::new("web");
    for input in WEB_INPUTS {
        println!("cargo::rerun-if-changed={}", web_dir.join(input).display());
    }

    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR should be set by Cargo"));
    pnpm(web_dir, &["install".as_ref(), "--frozen-lockfile".as_ref()]);
    pnpm(
        web_dir,
        &[
            "vite".as_ref(),
            "build".as_ref(),
            "--emptyOutDir".as_ref(),
            "--outDir".as_ref(),
            out_dir.join("web").as_os_str(),
        ],
    );
}

/// Runs pnpm with `args` in `dir`, panicking if it fails.
fn pnpm(dir: &Path, args: &[&OsStr]) {
    let status = Command::new("pnpm")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("pnpm should be installed");
    assert!(status.success(), "pnpm {args:?} failed with {status}");
}
