use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Serialize, Deserialize)]
struct Cached {
    token: String,
}

/// Return a usable token, logging in via the browser if none is cached.
pub fn ensure_token(web_url: &str) -> Result<String> {
    if let Some(token) = load_token()? {
        return Ok(token);
    }
    login(web_url, false)?;
    load_token()?.context("login did not produce a token")
}

pub fn login(web_url: &str, manual: bool) -> Result<()> {
    let token = if manual {
        manual_login(web_url)?
    } else {
        loopback_login(web_url)?
    };
    save_token(&token)?;
    eprintln!("spill: signed in.");
    Ok(())
}

pub fn logout() -> Result<()> {
    clear_token()?;
    eprintln!("spill: signed out.");
    Ok(())
}

fn manual_login(web_url: &str) -> Result<String> {
    eprintln!(
        "Open {web_url}/api/token while signed in, then paste the token or JSON response here:"
    );
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("read token from stdin")?;
    token_from_manual_input(&input)
}

fn token_from_manual_input(input: &str) -> Result<String> {
    let pasted = input.trim();
    if pasted.is_empty() {
        bail!("no token pasted");
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(pasted) {
        if let Some(token) = value.get("token").and_then(|token| token.as_str()) {
            return non_empty_token(token);
        }
        if let Some(token) = value.as_str() {
            return non_empty_token(token);
        }
        bail!("pasted JSON did not contain a token string");
    }

    non_empty_token(pasted)
}

fn non_empty_token(token: &str) -> Result<String> {
    let token = token.trim();
    if token.is_empty() {
        bail!("no token pasted");
    }
    Ok(token.to_owned())
}

fn loopback_login(web_url: &str) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind loopback listener")?;
    let port = listener.local_addr()?.port();
    let state = Uuid::new_v4().to_string();
    let url = format!("{web_url}/cli/login?cb=http://127.0.0.1:{port}&state={state}");

    eprintln!("spill: opening your browser to sign in…");
    eprintln!("If it doesn't open, visit:\n  {url}");
    eprintln!("(headless/SSH? run `spill login --manual` instead.)");
    let _ = open_browser(&url);

    listener.set_nonblocking(true).context("set nonblocking")?;
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => return handle_callback(stream, &state),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    bail!("login timed out after {}s", LOGIN_TIMEOUT.as_secs());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err).context("accept loopback connection"),
        }
    }
}

fn handle_callback(mut stream: std::net::TcpStream, expected_state: &str) -> Result<String> {
    stream.set_nonblocking(false).ok();
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut buf = [0u8; 8192];
    let read = stream.read(&mut buf).context("read callback request")?;
    let request = String::from_utf8_lossy(&buf[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");

    let mut token = None;
    let mut state = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("token", value)) => token = Some(value.to_string()),
            Some(("state", value)) => state = Some(value.to_string()),
            _ => {}
        }
    }

    let body = "<!doctype html><meta charset=utf-8><body style=\"font-family:sans-serif\">\
        Spill: signed in. You can close this tab and return to the terminal.</body>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());

    if state.as_deref() != Some(expected_state) {
        bail!("login state mismatch (possible interception); aborted");
    }
    token
        .filter(|t| !t.is_empty())
        .context("callback carried no token")
}

fn open_browser(url: &str) -> Result<()> {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else {
        ("xdg-open", vec![url])
    };
    Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .with_context(|| format!("launch {program}"))
}

fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir().context("no config dir")?.join("spill");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    restrict(&dir, 0o700);
    Ok(dir)
}

fn token_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("token.json"))
}

fn load_token() -> Result<Option<String>> {
    let path = token_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let cached: Cached = serde_json::from_str(&raw).context("parse cached token")?;
    Ok(Some(cached.token).filter(|t| !t.is_empty()))
}

fn save_token(token: &str) -> Result<()> {
    let path = token_path()?;
    let body = serde_json::to_string(&Cached {
        token: token.to_string(),
    })?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    restrict(&path, 0o600);
    Ok(())
}

pub fn clear_token() -> Result<()> {
    let path = token_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn restrict(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict(_path: &std::path::Path, _mode: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_login_accepts_raw_token() {
        assert_eq!(
            token_from_manual_input("ey.raw.token\n").unwrap(),
            "ey.raw.token"
        );
    }

    #[test]
    fn manual_login_extracts_token_from_json_response() {
        assert_eq!(
            token_from_manual_input(r#"{"token":"ey.json.token","email":"ava@example.com"}"#)
                .unwrap(),
            "ey.json.token"
        );
    }

    #[test]
    fn manual_login_rejects_json_without_token() {
        assert!(token_from_manual_input(r#"{"email":"ava@example.com"}"#).is_err());
    }
}
