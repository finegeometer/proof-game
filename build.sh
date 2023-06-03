#!/bin/sh

set -ex

WASM_BINDGEN_WEAKREF=1 wasm-pack build --target web
python3 -m http.server