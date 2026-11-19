//! Builds the app into `OUT_DIR/app`, from where the server embeds it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Paths in the app directory that its build depends on.
const APP_INPUTS: [&str; 8] = [
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
    let app_dir = Path::new("../app");
    for input in APP_INPUTS {
        println!("cargo::rerun-if-changed={}", app_dir.join(input).display());
    }

    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR should be set by Cargo"));
    pnpm(app_dir, &["install".as_ref(), "--frozen-lockfile".as_ref()]);
    pnpm(
        app_dir,
        &[
            "vite".as_ref(),
            "build".as_ref(),
            "--emptyOutDir".as_ref(),
            "--outDir".as_ref(),
            out_dir.join("app").as_os_str(),
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
