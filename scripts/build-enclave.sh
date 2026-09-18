#!/bin/bash
set -euo pipefail

# Build the enclave EIF and emit its PCR measurements.
#
# Nix constructs a reproducible OCI image and converts its root filesystem directly into
# an EIF with aws-nitro-util and AWS's EIF builder.
#
# Needs x86_64-linux. Nitro hardware is only needed to run.
#
# Usage: scripts/build-enclave.sh [output-dir]
#        (output-dir defaults to target/eif)
#
# Outputs in <output-dir>:
#   di-enclave.eif   the enclave image
#   di-pcr.json      PCR measurements extracted from the EIF

out_dir="target/eif"
output_dir_provided=false

usage() {
  printf '%s\n' \
    "Usage: scripts/build-enclave.sh [output-dir]" \
    "" \
    "Build the enclave EIF and emit its PCR measurements." \
    "" \
    "Options:" \
    "  -h, --help  Show this help."
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "[ERROR] Unknown option: $1" >&2
      exit 2
      ;;
    *)
      if [[ "$output_dir_provided" == "true" ]]; then
        echo "[ERROR] Only one output directory may be provided." >&2
        exit 2
      fi
      out_dir="$1"
      output_dir_provided=true
      ;;
  esac
  shift
done

command -v nix >/dev/null || {
  echo "[ERROR] nix not found. The OCI image and EIF are built by flake.nix." >&2
  exit 1
}

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"

# --no-update-lock-file on the flake calls below: an input added to flake.nix without a
# matching `nix flake update` would otherwise be resolved to whatever upstream serves right
# now, and the lock silently rewritten. The PCRs must follow the committed lock or nothing.
echo "Building reproducible OCI image..."
if ! oci_store=$(nix build ".#di-oci" --no-update-lock-file --no-link --print-out-paths); then
  echo >&2
  echo "[ERROR] OCI image build failed; the error above says why. A 'platform" >&2
  echo "        mismatch' for x86_64-linux means this host needs a remote builder." >&2
  exit 1
fi

echo "Building EIF..."
if ! eif_store=$(nix build ".#di-eif" --no-update-lock-file --no-link --print-out-paths); then
  echo >&2
  echo "[ERROR] EIF build failed; the error above says why." >&2
  exit 1
fi

install -m 0644 "$eif_store/image.eif" "$out_dir/di-enclave.eif"
install -m 0644 "$eif_store/pcr.json" "$out_dir/di-pcr.json"

echo "Validating measurements..."
# Registering a missing or malformed PCR with a client would weaken verification.
for pcr in PCR0 PCR1 PCR2; do
  value="$(jq -r --arg k "$pcr" '.[$k] // ""' "$out_dir/di-pcr.json")"
  if [[ ! "$value" =~ ^[0-9a-f]{96}$ ]]; then
    echo "[ERROR] di-pcr.json holds no usable $pcr (got '$value')." >&2
    echo "        eif_build's output format may have changed; do not register these." >&2
    exit 1
  fi
done

echo
echo "OCI image:    $oci_store"
echo "EIF:          $out_dir/di-enclave.eif"
echo "Measurements: $out_dir/di-pcr.json"
jq . "$out_dir/di-pcr.json"
