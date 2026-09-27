use std::fmt::Display;
use std::io::IsTerminal;

/// Stream target to evaluate color support for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

impl Stream {
    fn is_terminal(self) -> bool {
        match self {
            Stream::Stdout => std::io::stdout().is_terminal(),
            Stream::Stderr => std::io::stderr().is_terminal(),
        }
    }
}

/// Detects whether colors should be enabled based on standard environment variables
/// and terminal detection.
///
/// Precedence:
/// 1. `CLICOLOR_FORCE`: if present and != "0", colors enabled.
/// 2. `NO_COLOR`: if present and non-empty, colors disabled (per no-color.org).
/// 3. `CLICOLOR`: if present and == "0", colors disabled.
/// 4. Terminal detection: enabled if the target stream is a TTY.
pub fn colors_enabled_for_stream(stream: Stream) -> bool {
    colors_enabled_with_env(
        stream.is_terminal(),
        std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
        std::env::var_os("CLICOLOR_FORCE").map(|v| v != "0"),
        std::env::var_os("CLICOLOR").map(|v| v != "0"),
    )
}

/// Pure helper to determine color enablement from explicit flags.
pub fn colors_enabled_with_env(
    is_tty: bool,
    no_color: bool,
    clicolor_force: Option<bool>,
    clicolor: Option<bool>,
) -> bool {
    if clicolor_force == Some(true) {
        return true;
    }

    if no_color {
        return false;
    }

    if clicolor == Some(false) {
        return false;
    }

    is_tty
}

/// Terminal style configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    enabled: bool,
}

impl Theme {
    /// Create a theme explicitly enabled or disabled.
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Auto-detect theme settings for stdout.
    pub fn stdout() -> Self {
        Self::new(colors_enabled_for_stream(Stream::Stdout))
    }

    /// Auto-detect theme settings for stderr.
    pub fn stderr() -> Self {
        Self::new(colors_enabled_for_stream(Stream::Stderr))
    }

    /// Returns whether colors are enabled for this theme.
    pub const fn is_enabled(self) -> bool {
        self.enabled
    }

    /// Accent style: Cyan (target versions, tags, fix commands).
    pub fn accent<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[36m")
    }

    /// Muted style: Dim / Gray (secondary labels, paths).
    pub fn muted<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[90m")
    }

    /// Success style: Soft Green (✔ and success states).
    pub fn success<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[32m")
    }

    /// Warning style: Amber / Yellow (ℹ [DRY RUN] and warnings).
    pub fn warning<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[33m")
    }

    /// Error style: Coral Red (✖ and error labels).
    pub fn error<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[31m")
    }

    /// Bold style.
    pub fn bold<T: Display>(&self, text: T) -> String {
        self.wrap(text, "\x1b[1m")
    }

    /// Success icon: ✔
    pub fn success_icon(&self) -> String {
        self.success("✔")
    }

    /// Error icon: ✖
    pub fn error_icon(&self) -> String {
        self.error("✖")
    }

    /// Info icon: ℹ
    pub fn info_icon(&self) -> String {
        self.warning("ℹ")
    }

    /// Bullet icon: •
    pub fn bullet(&self) -> String {
        if self.enabled {
            self.muted("•")
        } else {
            "•".to_string()
        }
    }

    /// Arrow icon: ->
    pub fn arrow(&self) -> String {
        if self.enabled {
            self.muted("->")
        } else {
            "->".to_string()
        }
    }

    fn wrap<T: Display>(&self, text: T, code: &str) -> String {
        if self.enabled {
            format!("{code}{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

// Global convenience functions detecting stdout/stderr automatically or falling back.

pub fn success_icon() -> String {
    Theme::stdout().success_icon()
}

pub fn error_icon() -> String {
    Theme::stderr().error_icon()
}

pub fn info_icon() -> String {
    Theme::stdout().info_icon()
}

pub fn bullet() -> String {
    Theme::stdout().bullet()
}

pub fn arrow() -> String {
    Theme::stdout().arrow()
}

pub fn accent<T: Display>(text: T) -> String {
    Theme::stdout().accent(text)
}

pub fn muted<T: Display>(text: T) -> String {
    Theme::stdout().muted(text)
}

pub fn success<T: Display>(text: T) -> String {
    Theme::stdout().success(text)
}

pub fn warning<T: Display>(text: T) -> String {
    Theme::stdout().warning(text)
}

pub fn error<T: Display>(text: T) -> String {
    Theme::stderr().error(text)
}

pub fn bold<T: Display>(text: T) -> String {
    Theme::stdout().bold(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colors_enabled_logic() {
        // TTY true, no env -> true
        assert!(colors_enabled_with_env(true, false, None, None));
        // TTY false, no env -> false
        assert!(!colors_enabled_with_env(false, false, None, None));

        // NO_COLOR=1 disables even on TTY
        assert!(!colors_enabled_with_env(true, true, None, None));
        // CLICOLOR_FORCE overrides NO_COLOR
        assert!(colors_enabled_with_env(true, true, Some(true), None));
        assert!(colors_enabled_with_env(false, true, Some(true), None));
        // CLICOLOR_FORCE=0 does not override NO_COLOR
        assert!(!colors_enabled_with_env(true, true, Some(false), None));

        // CLICOLOR=0 disables
        assert!(!colors_enabled_with_env(true, false, None, Some(false)));
        // CLICOLOR=1 on non-TTY still requires TTY unless CLICOLOR_FORCE
        assert!(!colors_enabled_with_env(false, false, None, Some(true)));
    }

    #[test]
    fn test_theme_colored_output() {
        let theme = Theme::new(true);
        assert_eq!(theme.accent("test"), "\x1b[36mtest\x1b[0m");
        assert_eq!(theme.muted("test"), "\x1b[90mtest\x1b[0m");
        assert_eq!(theme.success("test"), "\x1b[32mtest\x1b[0m");
        assert_eq!(theme.warning("test"), "\x1b[33mtest\x1b[0m");
        assert_eq!(theme.error("test"), "\x1b[31mtest\x1b[0m");
        assert_eq!(theme.bold("test"), "\x1b[1mtest\x1b[0m");
        assert_eq!(theme.success_icon(), "\x1b[32m✔\x1b[0m");
        assert_eq!(theme.error_icon(), "\x1b[31m✖\x1b[0m");
        assert_eq!(theme.info_icon(), "\x1b[33mℹ\x1b[0m");
        assert_eq!(theme.bullet(), "\x1b[90m•\x1b[0m");
        assert_eq!(theme.arrow(), "\x1b[90m->\x1b[0m");
    }

    #[test]
    fn test_theme_uncolored_output() {
        let theme = Theme::new(false);
        assert_eq!(theme.accent("test"), "test");
        assert_eq!(theme.muted("test"), "test");
        assert_eq!(theme.success("test"), "test");
        assert_eq!(theme.warning("test"), "test");
        assert_eq!(theme.error("test"), "test");
        assert_eq!(theme.bold("test"), "test");
        assert_eq!(theme.success_icon(), "✔");
        assert_eq!(theme.error_icon(), "✖");
        assert_eq!(theme.info_icon(), "ℹ");
        assert_eq!(theme.bullet(), "•");
        assert_eq!(theme.arrow(), "->");
    }
}
