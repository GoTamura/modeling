# modeling
Toy WebGPU-based modeling tools in Rust, written as learning project(WIP)  

This application runs on Windows and Linux, and also works in web browsers.

## Features
- Works on Windows and Linux, and web browsers.
- Load Wavefront OBJ, GLTF


## Getting started
### Build
To compile for Windows or Linux: 
```
$ cargo build --release
```
or Web Browsers:
```
$ bash run.sh
```
and access localhost:1234
### Requirements
- Rust 1.55.0 or higher

For WebAssembly
- wasm-bindgen 0.2.78 or higher
- Python 3.8.10 or higher
