#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// CPU architecture names used by release tooling.
pub mod arch;
/// Filesystem helpers for repository automation.
pub mod fs;
/// Repository root helpers for `xtask` crates.
pub mod repo;

#[cfg(feature = "github-actions")]
#[cfg_attr(docsrs, doc(cfg(feature = "github-actions")))]
/// GitHub Actions output helpers.
pub mod github_actions;

#[cfg(feature = "hash")]
#[cfg_attr(docsrs, doc(cfg(feature = "hash")))]
/// Hashing helpers for release artifacts.
pub mod hash;

#[cfg(feature = "latest-json")]
#[cfg_attr(docsrs, doc(cfg(feature = "latest-json")))]
/// Small channel-based `latest.json` manifests.
pub mod latest_json;

#[cfg(feature = "process")]
#[cfg_attr(docsrs, doc(cfg(feature = "process")))]
/// Process and shell helpers.
pub mod process;

#[cfg(feature = "sparkle")]
#[cfg_attr(docsrs, doc(cfg(feature = "sparkle")))]
/// Sparkle appcast generation and merge helpers.
pub mod sparkle;

#[cfg(all(feature = "apple", any(target_os = "macos", docsrs)))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "apple", target_os = "macos"))))]
/// Apple platform automation: Xcode, codesign, and provisioning profiles.
pub mod apple;

#[cfg(all(feature = "dmg", any(target_os = "macos", docsrs)))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "dmg", target_os = "macos"))))]
/// macOS DMG packaging helpers.
pub mod dmg;
