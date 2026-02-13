# Building from Source

This guide explains how to build and run **cronbird** directly from the source code.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Edition 2024, latest stable recommended)
- [Just](https://github.com/casey/just) (optional, but recommended for development)

## Clone the Repository

```bash
git clone https://github.com/lebe-dev/cronbird.git
cd cronbird
```

## Build and Run

### Using `just` (Recommended)

```bash
# Prepare environment
cp .env.example .env

# Run the backend
just run-backend
```

### Using `cargo`

```bash
# Build the project
cargo build --release

# Run the binary
./target/release/cronbird
```

## Running Tests

Before contributing, ensure all tests pass:

```bash
just test
# or
cargo test
```

## Environment Variables

When running from source, you can use a `.env` file or export variables:

```bash
export CRONBIRD_LOG_LEVEL=debug
cargo run
```

## Production Build

For a production-ready binary, we recommend using the multi-stage Docker build, as it includes aggressive optimizations like `upx` compression and `strip` to minimize binary size.

If building manually for production:
1. Build with `--release`.
2. `strip` the binary.
3. (Optional) Compress with `upx`.
