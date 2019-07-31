cfg_if::cfg_if! {
    if #[cfg(not(feature = "std"))] {
        mod no_std;
        pub use self::no_std::*;
    } else if #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))] {
        mod wasm;
        pub use self::wasm::*;
    } else {
        mod normal;
        pub use self::normal::*;
    }
}
