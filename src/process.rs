use std::error::Error;
use std::fmt;
use std::path::Path;

use anyhow::{Context, Result, bail};
use xshell::Shell;

/// Error type for commands that intentionally terminate with a specific process exit code.
///
/// This lets an `xtask` command communicate meaningful statuses to shell scripts while
/// still using `anyhow::Result` internally:
///
/// ```
/// # fn run() -> anyhow::Result<()> {
/// if true {
///     return Err(xtask::process::ExitCode::new(2).into());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ExitCode(i32);

impl ExitCode {
    /// Create an exit-code error.
    pub fn new(code: i32) -> Self {
        Self(code)
    }

    /// Return the process exit code.
    pub fn code(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "exit with status {}", self.0)
    }
}

impl Error for ExitCode {}

/// Create an [`xshell::Shell`] with context suitable for CLI errors.
pub fn shell() -> Result<Shell> {
    Shell::new().context("create shell")
}

/// Return true when `program` exists somewhere on `PATH`.
pub fn command_exists(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            let candidate = dir.join(program);
            candidate.is_file()
        })
    })
}

/// Fail with a user-facing message when `program` is not on `PATH`.
pub fn ensure_command(program: &str) -> Result<()> {
    if command_exists(program) {
        Ok(())
    } else {
        bail!("{program} not found on PATH")
    }
}

/// Temporarily change the shell's current directory for a command block.
pub fn push_dir<'a>(sh: &'a Shell, path: impl AsRef<Path>) -> xshell::PushDir<'a> {
    sh.push_dir(path)
}
