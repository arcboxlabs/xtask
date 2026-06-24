/// A CPU architecture normalized for release artifact names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Architecture {
    /// 64-bit ARM, named `arm64` by Apple and many package managers.
    Arm64,
    /// 64-bit x86, named `x86_64` by Rust and Apple.
    X86_64,
}

impl Architecture {
    /// Return the architecture for the current compilation target.
    pub fn current() -> Option<Self> {
        match std::env::consts::ARCH {
            "aarch64" => Some(Self::Arm64),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    /// Rust target architecture name.
    pub const fn rust(self) -> &'static str {
        match self {
            Self::Arm64 => "aarch64",
            Self::X86_64 => "x86_64",
        }
    }

    /// Apple artifact architecture name.
    pub const fn apple(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64",
            Self::X86_64 => "x86_64",
        }
    }

    /// Debian / nfpm architecture name.
    pub const fn debian(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64",
            Self::X86_64 => "amd64",
        }
    }
}
