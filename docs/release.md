# Releasing

Two release lines, each on its own tag:

- `enclave/vX.Y.Z`: the reproducible OCI image and the EIF, handled by
  [`.github/workflows/release-enclave.yml`](../.github/workflows/release-enclave.yml)
- `host/vX.Y.Z`: the host container image, handled by
  [`.github/workflows/release-host.yml`](../.github/workflows/release-host.yml)

## Cutting an enclave release

1. **Bump the version.** Edit `workspace.package.version` in `Cargo.toml`, then refresh the
   lockfile:
   ```
   cargo update --workspace
   ```
2. **Tag and push:**
   ```
   git tag enclave/v0.2.0 <sha-on-main>
   git push origin enclave/v0.2.0
   ```
3. **Wait for the build.**
4. **Review the draft**, check the PCR table, publish. The draft is the last human step: the
   release is created unpublished, so nothing is live until someone publishes it.

> The `release` GitHub environment carries no protection rules today, so publishing passes
> straight through and images are pushed as soon as the enclave build succeeds. To require a
> human before anything is published, add required reviewers to that environment — a settings
> change, no workflow edit.

To exercise the pipeline without a tag, dispatch it:

```
gh workflow run release-enclave.yml -f ref=main -f version=0.1.0 -f dry_run=true
```

A dry run builds, verifies and publishes nothing.

## How the measurement is produced

Nix builds both artifacts from the same root filesystem. `dockerTools.buildLayeredImage`
produces the OCI image, while `aws-nitro-util` produces the EIF using pinned AWS kernel,
init, and NSM blobs. Both are content-addressed Nix outputs.

PCR0 covers the kernel, the cmdline and both ramdisks; PCR1 the kernel and boot ramdisk;
PCR2 the application ramdisk. The EIF metadata section — which carries a wall-clock
`BuildTime` — is *not* measured, so the timestamp does not reach a PCR.

## Building and measuring locally

Needs x86_64-linux with Nix. No credentials: the enclave has no private dependencies and no
model artifacts.

```
scripts/build-enclave.sh target/eif
jq . target/eif/di-pcr.json
```

## Rotating a measurement in production

Clients pin PCR0. A client configuration that accepts several measurements at once is what
makes a rotation safe:

1. Publish the new release. Add its PCR0 to the client allow-list **alongside** the old one.
2. Wait for clients to pick up the new allow-list. Until they have, deploying the new enclave
   alone would break every client still pinning only the old measurement.
3. Deploy the new enclave.
4. Retire the old measurement from the allow-list once nothing is verifying against it.

## Verifying a published release

```
gh release download enclave/v0.2.0 -R worldcoin/di-migration-tee
gh attestation verify manifest.json --repo worldcoin/di-migration-tee \
   --signer-workflow worldcoin/di-migration-tee/.github/workflows/release-enclave.yml
```

The attestation binds the assets to the workflow and commit that produced them. Then reproduce
the measurements from source and compare against `manifest.json`.

## Notes

- `host` and `enclave` are skeletons that exit with a failure code. A tag exercises the release
  pipeline; it does not ship a working service.
