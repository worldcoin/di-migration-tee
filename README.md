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

## Building the enclave image

The enclave is a reproducible OCI image and an AWS Nitro EIF, built by Nix: it converts the OCI
root filesystem directly with aws-nitro-util, so the same commit yields the same PCRs.

```bash
# Needs Linux x86_64; Nitro hardware is only needed to run.
scripts/build-enclave.sh          # -> target/eif/di-enclave.eif, di-pcr.json

# Build or inspect only the reproducible OCI boundary.
nix build .#di-oci
skopeo inspect "oci:$(readlink -f result):$(nix eval --raw .#packages.x86_64-linux.di-enclave.version)"
```

The build needs no credentials — no private dependencies, no model artifacts.

`scripts/Dockerfile.carrier` wraps a built EIF in an ordinary container, since an enclave cannot
be scheduled directly. It exits non-zero when the enclave dies, so the pod restarts.

## Nitro-enabled development host

Use an Amazon Linux 2023 EC2 instance type that supports Nitro Enclaves and launch it with
Nitro Enclaves enabled. Limit inbound SSH access to your current public IP.

On the instance, install the development tools and Nitro Enclaves runtime:

```bash
sudo dnf install -y \
  git wget jq tmux tree unzip tar gzip \
  gcc gcc-c++ make cmake clang pkgconf-pkg-config openssl-devel \
  docker aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel

sudo usermod -aG ne "$USER"
sudo usermod -aG docker "$USER"
sudo systemctl enable --now nitro-enclaves-allocator.service
sudo systemctl enable --now docker
```

These commands follow the [AWS Nitro Enclaves setup for Amazon Linux 2023](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave-cli-install.html).

The allocator defaults to 2 vCPUs and 512 MiB. Adjust
`/etc/nitro_enclaves/allocator.yaml` before starting the service when the enclave needs more.
Log out and reconnect after changing group membership, then verify the host:

```bash
nitro-cli --version
nitro-cli describe-enclaves
docker version
```

Install Rust and the components this workspace uses:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt clippy
cargo test --all
```
