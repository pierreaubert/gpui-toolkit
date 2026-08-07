//! Installed Python application host. It shares the showcase implementation
//! so protocol behavior remains identical between development and bundles.

#![recursion_limit = "512"]
#![forbid(unsafe_code)]
#![allow(unused_attributes)]

#[path = "showcase.rs"]
mod showcase;

fn main() {
    showcase::main();
}
