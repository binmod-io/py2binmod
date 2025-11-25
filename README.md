# py2binmod

Compile Python projects to Binmod WebAssembly modules.

## Overview

py2binmod transpiles Python code that uses the Binmod Python MDK into Rust, bundles a Python interpreter (RustPython), and compiles everything to WebAssembly. This allows you to write plugins in Python and run them in any Binmod runtime.

**Trade-offs:** Larger module sizes and slower startup times compared to native Rust modules, but enables using Python code in the Binmod ecosystem.

## Installation

### Option 1: Docker (Recommended)

No local dependencies required:

```bash
docker run -ti -v "$(pwd)":/app ghcr.io/binmod-io/py2binmod:0.1.0 build
```

### Option 2: Install via pip

Requires Rust toolchain:

```bash
pip install py2binmod
```

**Prerequisites for pip installation:**
- Install Rust from [rustup.rs](https://rustup.rs/)
- Add the WebAssembly WASI Preview 1 target:
  ```bash
  rustup target add wasm32-wasip1
  ```

## Usage

### Build Command

Compile your Python module to WebAssembly:

**With Docker:**
```bash
docker run -ti -v "$(pwd)":/app ghcr.io/binmod-io/py2binmod:0.1.0 build [--release]
```

**With pip:**
```bash
py2binmod build [--release]
```

Your compiled WebAssembly module will be at:
```
artifacts/wasm32-wasip1/[debug|release]/<project_name>.wasm
```

### Custom Output Directory

```bash
py2binmod build --out-dir ./custom_output
```

### Transpile Command

Generate Rust code without compiling:

```bash
py2binmod transpile [--out-dir ./rust_output]
```

Without `--out-dir`, prints to stdout. Useful for debugging.

## How It Works

py2binmod:
1. Parses your Python project that uses the Binmod Python MDK
2. Generates a Rust wrapper using the Binmod Rust MDK
3. Bundles RustPython (a Python interpreter written in Rust)
4. Compiles everything to WebAssembly with the `wasm32-wasip1` target

Your Python code runs in the bundled interpreter, wrapped in a Rust layer that handles the Binmod protocol.

## License

MIT License

## Support

If you encounter issues or have questions, please open an issue on GitHub.