// SPDX-License-Identifier: MIT OR Apache-2.0
//! On Windows, embed the app icon (`assets/brand/icon.ico`) and the program's name in the
//! executable, so Explorer, the taskbar and the file types registered to it show the brand. Does
//! nothing for any other target.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

const ICON: &str = "../../assets/brand/icon.ico";

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");
    // The target, not the machine running the build script.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ICON)
        .set("ProductName", "Wordsmith Manuscript")
        .set("FileDescription", "Wordsmith Manuscript")
        .set("CompanyName", "The James C. J. Foundation")
        .set("LegalCopyright", "Copyright (c) 2026 the Wordsmith Manuscript contributors. MIT OR Apache-2.0");
    res.compile()
}
