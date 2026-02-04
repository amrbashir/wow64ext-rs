//! wow64ext is a Rust bindings for [rewolf-wow64ext]
//!
//! <div class="warning">
//!
//! Because [rewolf-wow64ext] uses DllMain to initialize the library, which causes issues when linking with Rust.
//! This crate applies a patch to remove DllMain and provides [`Wow64ExtInitialize()`] as an alternative initialization function.
//! Therefore, you must call [`Wow64ExtInitialize()`] before using any other functions in this crate.
//!
//! </div>
//!
//!
//! [rewolf-wow64ext]: https://github.com/rwfpl/rewolf-wow64ext

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
