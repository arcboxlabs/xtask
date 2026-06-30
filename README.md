# xtask-kit

Reusable building blocks for Rust `xtask` crates.

This crate is deliberately **not** a framework for defining one universal `xtask`
CLI. Each repository should keep its own `clap` command tree and product-specific
build flow. `xtask-kit` provides the small, reusable primitives that tend to be copied
between repositories: repository paths, file checks, process helpers, artifact
hashing, Apple signing, DMG packaging, Sparkle appcasts, and GitHub Actions outputs.

## Feature flags

| Feature | Default | Purpose |
| --- | --- | --- |
| `process` | yes | `xshell` setup and command lookup helpers. |
| `hash` | yes | SHA-256 and human-readable artifact sizes. |
| `github-actions` | no | Append values to `GITHUB_OUTPUT`. |
| `latest-json` | no | Update simple channel-based `latest.json` manifests. |
| `sparkle` | no | Generate and merge Sparkle RSS appcasts. |
| `apple` | no | macOS-only Xcode, plist, codesign, provisioning profile, and notarization helpers. |
| `dmg` | no | macOS-only `create-dmg` wrapper. Implies `apple` and `hash`. |

Apple modules are compiled on `target_os = "macos"` only. This keeps Linux CI and
non-Apple `xtask` crates from accidentally depending on platform-specific APIs.

## Repository root discovery

Library crates cannot use their own `env!("CARGO_MANIFEST_DIR")` to discover a
downstream repository. Pass the manifest directory from your repository's `xtask`
crate instead:

```rust
let root = xtask_kit::repo::root_from_xtask_manifest(env!("CARGO_MANIFEST_DIR"))?;
# anyhow::Ok(())
```

## Basic usage

```rust,no_run
use xtask_kit::{fs, hash, repo};

let root = repo::root_from_xtask_manifest(env!("CARGO_MANIFEST_DIR"))?;
let app = root.join("target/release/MyApp.app");

fs::ensure_dir(&app)?;
let sha = hash::sha256_file(root.join("dist/MyApp.dmg"))?;
println!("sha256={sha}");
# anyhow::Ok(())
```

## macOS signing example

```rust,no_run
#[cfg(target_os = "macos")]
fn sign_app(app: &std::path::Path) -> anyhow::Result<()> {
    let options = xtask_kit::apple::CodesignOptions::runtime("Developer ID Application", app);
    xtask_kit::apple::codesign(&options)?;
    xtask_kit::apple::verify_signature(app)
}
# #[cfg(target_os = "macos")]
# sign_app(std::path::Path::new("MyApp.app"))?;
# anyhow::Ok(())
```

## DMG example

```rust,no_run
#[cfg(target_os = "macos")]
fn package(app: std::path::PathBuf, output: std::path::PathBuf) -> anyhow::Result<()> {
    let options = xtask_kit::dmg::CreateDmgOptions::new("MyApp", app, output);
    xtask_kit::dmg::create(&options)
}
# #[cfg(target_os = "macos")]
# package("MyApp.app".into(), "MyApp.dmg".into())?;
# anyhow::Ok(())
```

## Publishing

Publish both package names for each release:

- `xtask` keeps existing users compatible.
- `xtask-kit` is the preferred non-colliding package name for repositories that
  already have their own `xtask` package.

For each package name, run:

```sh
cargo publish --dry-run --all-features
cargo publish --all-features
```

Keep the version number identical for both packages. The repository normally
tracks the `xtask-kit` package metadata; publish the compatibility `xtask`
package from a temporary clean checkout or release branch by changing only
`package.name` and docs.rs metadata for that publish.
