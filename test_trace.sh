#!/bin/bash
set -x
./target/debug/coolc samples/hello_world.cl -o hello_world.wasm
ls -lh hello_world.wasm
wasmtime hello_world.wasm
