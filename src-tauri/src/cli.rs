use anyhow::{anyhow, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

const PROMPT: &str = "请用一句简体中文，写一段克制而深刻的人生感悟。\n\
要求：\n\
1. 不要鸡汤、励志套话或常见格言的改写\n\
2. 含一点张力、悖论或反直觉的观察\n\
3. 30~60 字，单行，不要引号、不要标题\n\
4. 直接输出这一句话本身，不要任何前言、解释、备注\n";

pub fn generate() -> Result<(String, String)> {
    let path = enriched_path();
    if let Some(text) = try_claude(&path) {
        return Ok((text, "claude".into()));
    }
    if let Some(text) = try_codex(&path) {
        return Ok((text, "codex".into()));
    }
    Err(anyhow!(
        "未找到可用的 claude 或 codex CLI——请确认已安装并位于 PATH"
    ))
}

fn try_claude(path: &OsString) -> Option<String> {
    let out = Command::new("claude")
        .env("PATH", path)
        .args(["-p", PROMPT])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = clean(String::from_utf8_lossy(&out.stdout).to_string());
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn try_codex(path: &OsString) -> Option<String> {
    let out = Command::new("codex")
        .env("PATH", path)
        .args(["exec", PROMPT])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = clean(String::from_utf8_lossy(&out.stdout).to_string());
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn clean(raw: String) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let last_block = trimmed.rsplit("\n\n").next().unwrap_or(trimmed);
    last_block
        .trim()
        .trim_matches(|c: char| {
            matches!(c, '"' | '\'' | '“' | '”' | '「' | '」' | '《' | '》' | '`')
        })
        .to_string()
}

fn enriched_path() -> OsString {
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let extras = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];
    for e in extras {
        let p = PathBuf::from(e);
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        for sub in [".local/bin", ".cargo/bin", ".bun/bin", "bin"] {
            let p = home.join(sub);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    std::env::join_paths(paths).unwrap_or_default()
}
