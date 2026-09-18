# di-migration-tee

DeepIdentifier migration TEE backend — a Nitro enclave and the host that fronts it.

**This is a skeleton.** The crates compile and implement no behaviour. `di-host` and
`di-enclave` log and exit non-zero; a skeleton that idled would read as healthy to a liveness
probe. See
[Spec: DeepIdentifier Migration TEE Setup v1](https://app.notion.com/p/worldcoin/Spec-DeepIdentifier-Migration-TEE-Setup-v1-3c08614bdf8c8014b7ddf50f3cac4e4b)
for what goes in them.

## Structure

```text
di-migration-tee/
├── Cargo.toml   # Workspace -> Cargo.lock
├── host/        # Migration API and enclave relay — the untrusted side of the boundary
└── enclave/     # Nitro enclave workload — the trusted side
```

The host↔enclave vsock contract and the client↔host HTTP contract each get their own crate
once §6 of the spec settles. They are deliberately absent rather than empty: the point of one
crate per boundary is that the boundary is known, and these are not yet.

The layout follows [worldcoin/flamingo](https://github.com/worldcoin/flamingo), which runs the
same host/enclave shape for the verifier. The two repositories share no crate — a shared crate
means an edit on one side rotates the other's PCR0.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --
cargo test --workspace --all-features
cargo deny --all-features check
```

```bash
RUST_LOG=info cargo run --bin di-host
RUST_LOG=info cargo run --bin di-enclave
```

Both exit non-zero, as above.
