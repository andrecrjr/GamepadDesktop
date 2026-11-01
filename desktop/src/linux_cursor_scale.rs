use std::{env, process::Command};

use crate::cursor_scale::{CursorScale, CursorScaleRange, retry_cursor_restore, scaled_size};

// Largest size offered by GNOME Settings.
const GNOME_MAX_CURSOR_SIZE: u32 = 96;
const DEFAULT_PLASMA_CURSOR_SIZE: u32 = 24;

enum Backend {
    Gnome,
    Plasma {
        read_tool: String,
        apply_tool: String,
    },
}

pub(crate) struct LinuxCursorScale {
    backend: Option<Backend>,
    base_size: u32,
    max_size: u32,
    current_size: Option<u32>,
    modified: bool,
}

impl LinuxCursorScale {
    pub(crate) fn new() -> Self {
        let desktop = env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_ascii_uppercase();
        if desktop.split(':').any(|name| name == "GNOME")
            && let Some(base_size) = gnome_cursor_size()
        {
            return Self {
                backend: Some(Backend::Gnome),
                base_size,
                max_size: GNOME_MAX_CURSOR_SIZE.max(base_size),
                current_size: Some(base_size),
                modified: false,
            };
        }

        if (desktop
            .split(':')
            .any(|name| name.contains("KDE") || name.contains("PLASMA"))
            || env::var_os("KDE_SESSION_VERSION").is_some())
            && let Some((read_tool, apply_tool, base_size, max_size)) = plasma_cursor_size()
        {
            return Self {
                backend: Some(Backend::Plasma {
                    read_tool,
                    apply_tool,
                }),
                base_size,
                max_size: max_size.max(base_size),
                current_size: Some(base_size),
                modified: false,
            };
        }

        Self {
            backend: None,
            base_size: 0,
            max_size: 0,
            current_size: None,
            modified: false,
        }
    }

    pub(crate) fn range(&self) -> CursorScaleRange {
        if self.backend.is_none() {
            CursorScaleRange::unsupported()
        } else {
            CursorScaleRange::from_sizes(self.base_size, self.max_size)
        }
    }

    pub(crate) fn set(&mut self, scale: CursorScale) -> Result<bool, String> {
        if self.backend.is_none() {
            return Ok(false);
        }
        let size = scaled_size(self.base_size, scale.percent(), self.max_size);
        if self.current_size == Some(size) {
            return Ok(false);
        }
        self.current_size = Some(size);
        self.modified = true;
        self.apply_size(size)?;
        Ok(true)
    }

    fn apply_size(&self, size: u32) -> Result<(), String> {
        let size = size.to_string();
        match self.backend.as_ref() {
            Some(Backend::Gnome) => run_command(
                "gsettings",
                &["set", "org.gnome.desktop.interface", "cursor-size", &size],
            ),
            Some(Backend::Plasma {
                read_tool,
                apply_tool,
            }) => {
                let theme = read_plasma_config(read_tool, "cursorTheme", "breeze_cursors")
                    .unwrap_or_else(|| "breeze_cursors".to_owned());
                run_command(apply_tool, &[&theme, "--size", &size])
            }
            None => Ok(()),
        }
    }

    fn restore(&mut self) -> Result<(), String> {
        if !self.modified {
            return Ok(());
        }
        self.apply_size(self.base_size)?;
        self.current_size = Some(self.base_size);
        self.modified = false;
        Ok(())
    }
}

impl Drop for LinuxCursorScale {
    fn drop(&mut self) {
        retry_cursor_restore(|| self.restore());
    }
}

fn gnome_cursor_size() -> Option<u32> {
    command_output(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "cursor-size"],
    )?
    .parse()
    .ok()
    .filter(|size| *size > 0)
}

fn plasma_cursor_size() -> Option<(String, String, u32, u32)> {
    let read_tool = ["kreadconfig6", "kreadconfig5"]
        .into_iter()
        .find(|tool| command_help(tool).is_some())?;
    let apply_tool = "plasma-apply-cursortheme";
    if !command_help(apply_tool)?.contains("--size") {
        return None;
    }

    let theme = read_plasma_config(read_tool, "cursorTheme", "breeze_cursors")?;
    let base_size = read_plasma_config(
        read_tool,
        "cursorSize",
        &DEFAULT_PLASMA_CURSOR_SIZE.to_string(),
    )?
    .parse::<u32>()
    .ok()
    .filter(|size| *size > 0)?;
    let max_size = plasma_theme_max_size(&theme)?;
    Some((
        read_tool.to_owned(),
        apply_tool.to_owned(),
        base_size,
        max_size,
    ))
}

fn read_plasma_config(tool: &str, key: &str, default: &str) -> Option<String> {
    command_output(
        tool,
        &[
            "--file",
            "kcminputrc",
            "--group",
            "Mouse",
            "--key",
            key,
            "--default",
            default,
        ],
    )
}

fn plasma_theme_max_size(theme: &str) -> Option<u32> {
    let cursor = xcursor::CursorTheme::load(theme).load_icon("left_ptr")?;
    let contents = std::fs::read(cursor).ok()?;
    xcursor::parser::parse_xcursor(&contents)?
        .into_iter()
        .map(|image| image.size)
        .max()
}

fn command_help(program: &str) -> Option<String> {
    let output = Command::new(program).arg("--help").output().ok()?;
    Some(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if detail.is_empty() {
        format!("{program} exited with {}", output.status)
    } else {
        detail
    })
}
