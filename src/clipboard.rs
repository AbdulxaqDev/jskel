//! Copy text to the system clipboard by spawning the platform's CLI utility.
//! Avoids a clipboard crate dep. Falls through a list of candidates so we
//! cover macOS, Wayland, X11, and Windows without `cfg!` branching at every
//! call site.

use std::io::Write;
use std::process::{Command, Stdio};

pub fn copy(text: &str) -> Result<(), String> {
    for (cmd, args) in candidates() {
        match try_copy(cmd, args, text) {
            Ok(()) => return Ok(()),
            Err(_) => continue,
        }
    }
    Err(platform_hint().into())
}

fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    if cfg!(target_os = "macos") {
        vec![("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        vec![("clip", &[])]
    } else {
        let mut v: Vec<(&'static str, &'static [&'static str])> = Vec::new();
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            v.push(("wl-copy", &[]));
        }
        v.push(("xclip", &["-selection", "clipboard"]));
        v.push(("xsel", &["--clipboard", "--input"]));
        v
    }
}

fn try_copy(cmd: &str, args: &[&str], text: &str) -> Result<(), String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open stdin to clipboard tool".to_string())?;
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("{cmd} exited with {status}"));
    }
    Ok(())
}

fn platform_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "no clipboard tool found (pbcopy missing?)"
    } else if cfg!(target_os = "windows") {
        "no clipboard tool found (clip missing?)"
    } else {
        "no clipboard tool found — install xclip, xsel, or wl-copy"
    }
}
