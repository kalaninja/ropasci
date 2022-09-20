# Implementation of Rock Paper Scissors game in Substrate

Run Tests

```bash
cargo test --features runtime-benchmarks
```

Build Benchmarks

```bash
cargo build -p node-template --release --features runtime-benchmarks
```

Run Benchmarks

```bash
./target/release/node-template benchmark pallet --chain=dev --execution=wasm --wasm-execution=compiled --pallet=pallet_ropasci --extrinsic=* --steps=20 --repeat=50 --output=./weights.rs
```
