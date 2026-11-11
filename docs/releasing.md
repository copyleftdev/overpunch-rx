# Publishing releases

The [Publish to crates.io workflow](https://github.com/copyleftdev/overpunch-rx/actions/workflows/publish.yml)
runs manually from `main`. It checks formatting, Clippy, release tests and package
verification before an optional upload. Ordinary pushes and pull requests do not
publish a version.

## Credential

Set the repository's GitHub Actions secret `CARGO_REGISTRY_TOKEN` to a crates.io
API token authorized to publish `overpunch-rx`. The first release requires
permission to publish a new crate; subsequent releases require permission to
update that crate. Keep the token out of repository files and command arguments.

The workflow checks that the secret is present. A dry run validates the package
without uploading; it does not establish the token's validity or publish scope.
The registry checks authorization during the actual upload.

## Release procedure

1. Update the version in `Cargo.toml`, regenerate `Cargo.lock`, and commit the
   changes to `main`. Use a version that has not already been published.
2. Wait for the Rust checks workflow to pass on that commit.
3. Run the publishing workflow with its default dry-run setting:

   ```sh
   gh workflow run publish.yml --ref main -f publish=false
   ```

4. Inspect the run. To upload the version from `main`, run:

   ```sh
   gh workflow run publish.yml --ref main -f publish=true
   ```

The same controls are available under **Actions → Publish to crates.io → Run
workflow**. Leave the upload checkbox clear for verification, or select it to
publish. Each upload repeats verification on the selected commit. Publishing runs
are serialized, and an in-progress release is never canceled by a newer run.

See Cargo's [publishing documentation](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
for registry behavior and package verification details.
