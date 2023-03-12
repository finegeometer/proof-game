mod bezier;

#[cfg(target_family = "wasm")]
mod wasm;
#[cfg(target_family = "wasm")]
pub use wasm::*;

#[cfg(not(target_family = "wasm"))]
compile_error!(
    "Game compiled in a configuration that does not support rendering.
Currently supported:
  - Web Assembly (HTML/Svg rendering)
"
);
