use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::macros::format_description;

/// Options for updating a channel-based `latest.json` file.
#[derive(Clone, Debug)]
pub struct UpdateOptions {
    /// Release version, with or without a leading `v`.
    pub version: String,
    /// Update channel, for example `stable` or `beta`.
    pub channel: String,
    /// Output manifest path.
    pub output: PathBuf,
    /// Existing manifest to merge into.
    pub existing: Option<PathBuf>,
}

/// Update or create a channel-based `latest.json` manifest.
pub fn update(options: &UpdateOptions) -> Result<()> {
    let mut data = load_existing(options.existing.as_deref())?;
    let display_version = options.version.trim_start_matches('v');
    data.insert(
        options.channel.clone(),
        json!({
            "version": display_version,
            "date": OffsetDateTime::now_utc()
                .format(format_description!(
                    "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
                ))
                .context("format latest.json timestamp")?,
        }),
    );

    let content = serde_json::to_string_pretty(&Value::Object(data))? + "\n";
    crate::fs::write_string(&options.output, content)
}

/// Load an existing `latest.json` object, returning an empty map if the path is absent.
pub fn load_existing(existing: Option<&Path>) -> Result<Map<String, Value>> {
    let Some(existing) = existing.filter(|path| path.is_file()) else {
        return Ok(Map::new());
    };
    let content = std::fs::read_to_string(existing)
        .with_context(|| format!("read existing latest.json {}", existing.display()))?;
    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("parse existing latest.json {}", existing.display()))?;
    Ok(value.as_object().cloned().unwrap_or_default())
}
