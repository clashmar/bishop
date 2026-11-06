//! Local runtime lookup and preview serving for exported static sites.

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Command;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub fn player_runtime_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("BISHOP_WEB_PLAYER_RUNTIME_DIR") {
        return PathBuf::from(path);
    }
    let executable = std::env::current_exe().unwrap_or_default();
    let binary_dir = executable.parent().unwrap_or_else(|| std::path::Path::new("."));
    let bundle = binary_dir.join("../Resources/bundle_assets/web-player");
    for candidate in [
        &bundle,
        &binary_dir.join("../Resources/web-player"),
        &binary_dir.join("web-player"),
    ] {
        if candidate.is_dir() {
            return candidate.to_path_buf();
        }
    }
    if cfg!(debug_assertions) {
        if let Some(target) = binary_dir.parent() {
            let dev = target.join("web-player-runtime");
            if dev.is_dir() {
                return dev;
            }
        }
    }
    bundle
}

pub struct LocalPreview {
    port: u16,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LocalPreview {
    pub fn start(site: &Path) -> io::Result<Self> {
        let root = fs::canonicalize(site)?;
        if !root.join("index.html").is_file() {
            return Err(io::Error::new(io::ErrorKind::NotFound,
                format!("missing index.html in {}", root.display())));
        }
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let running = Arc::new(AtomicBool::new(true));
        let running_worker = Arc::clone(&running);
        let worker = thread::spawn(move || {
            while running_worker.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Accepted sockets may inherit the listener's nonblocking mode on macOS
                        if let Err(error) = stream.set_nonblocking(false) {
                            log::warn!("Could not configure local preview connection: {error}");
                            continue;
                        }
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                        if let Err(error) = serve_request(&mut stream, &root) {
                            log::debug!("Local preview request failed: {error}");
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self { port, running, worker: Some(worker) })
    }

    #[cfg(test)]
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }
}

impl Drop for LocalPreview {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn open_browser(url: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]);
        cmd
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    command.arg(url).spawn()?;
    Ok(())
}

pub fn open_folder(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    command.arg(path).spawn()?;
    Ok(())
}

fn serve_request(stream: &mut TcpStream, root: &Path) -> io::Result<()> {
    let mut request = [0u8; 8192];
    let read = stream.read(&mut request)?;
    let line = String::from_utf8_lossy(&request[..read]);
    let mut parts = line.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    if !matches!(method, "GET" | "HEAD") {
        return response(stream, 405, "text/plain", b"Method not allowed", method == "HEAD");
    }
    let Some(target) = target.strip_prefix('/') else {
        return response(stream, 400, "text/plain", b"Invalid path", method == "HEAD");
    };
    let Some(decoded) = decode_path(target.split('?').next().unwrap_or("")) else {
        return response(stream, 400, "text/plain", b"Invalid path", method == "HEAD");
    };
    let relative = if decoded.is_empty() { "index.html" } else { &decoded };
    let path = Path::new(relative);
    if path.components().any(|component| !matches!(component, Component::Normal(_))) {
        return response(stream, 400, "text/plain", b"Invalid path", method == "HEAD");
    }
    let file = match fs::canonicalize(root.join(path)) {
        Ok(file) if file.starts_with(root) && file.is_file() => file,
        Ok(_) => return response(stream, 403, "text/plain", b"Forbidden", method == "HEAD"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return response(stream, 404, "text/plain", b"Not found", method == "HEAD");
        }
        Err(_) => return response(stream, 500, "text/plain", b"Read failed", method == "HEAD"),
    };
    let bytes = fs::read(&file)?;
    let mime = match file.extension().and_then(|extension| extension.to_str()).unwrap_or("") {
        "wasm" => "application/wasm",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ron" | "lua" | "toml" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    response(stream, 200, mime, &bytes, method == "HEAD")
}

fn decode_path(encoded: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(encoded.len());
    let bytes = encoded.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    if path.contains(['\\', '\0']) { None } else { Some(path) }
}

fn response(stream: &mut TcpStream, status: u16, mime: &str, body: &[u8], head: bool) -> io::Result<()> {
    let reason = match status {
        200 => "OK", 400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found",
        405 => "Method Not Allowed", _ => "Internal Server Error",
    };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n", body.len())?;
    if !head {
        stream.write_all(body)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use tempfile::TempDir;

    fn request(port: u16, method: &str, target: &str) -> io::Result<String> {
        let mut stream = TcpStream::connect(("127.0.0.1", port))?;
        stream.write_all(format!("{method} {target} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    #[test]
    fn preview_streams_the_entire_release_sized_wasm() -> io::Result<()> {
        let site = TempDir::new()?;
        let player = site.path().join("bishop-player");
        fs::create_dir(&player)?;
        fs::write(site.path().join("index.html"), "Preview")?;
        let size = 7_512_180;
        fs::write(player.join("web_shell.wasm"), vec![b'x'; size])?;
        let preview = LocalPreview::start(site.path())?;
        let response = request(preview.port(), "GET", "/bishop-player/web_shell.wasm")?;
        let (_, body) = response.split_once("\r\n\r\n").ok_or_else(|| io::Error::other("no HTTP body"))?;
        assert_eq!(body.len(), size, "response body was truncated");
        Ok(())
    }

    #[test]
    fn preview_serves_webassembly_with_correct_mime_and_rejects_traversal() -> io::Result<()> {
        let site = TempDir::new()?;
        fs::create_dir(site.path().join("bishop-player"))?;
        fs::write(site.path().join("index.html"), "Bishop preview")?;
        fs::write(site.path().join("bishop-player/web_shell.wasm"), b"wasm-test")?;
        let preview = LocalPreview::start(site.path())?;
        let port = preview.port();
        assert!(preview.url().starts_with("http://127.0.0.1:"));
        let root = request(port, "GET", "/")?;
        assert!(root.starts_with("HTTP/1.1 200"), "{root}");
        assert!(root.ends_with("Bishop preview"), "{root}");
        let wasm = request(port, "GET", "/bishop-player/web_shell.wasm")?;
        assert!(wasm.contains("Content-Type: application/wasm"), "{wasm}");
        let head = request(port, "HEAD", "/bishop-player/web_shell.wasm")?;
        assert!(head.contains("Content-Type: application/wasm"), "{head}");
        assert!(!head.ends_with("wasm-test"), "HEAD must not send the body");
        assert!(request(port, "GET", "/%2e%2e/outside.txt")?.starts_with("HTTP/1.1 400"));
        assert!(request(port, "GET", "/missing")?.starts_with("HTTP/1.1 404"));
        #[cfg(unix)]
        {
            let outside = TempDir::new()?;
            let secret = outside.path().join("secret.txt");
            fs::write(&secret, "do not serve")?;
            std::os::unix::fs::symlink(&secret, site.path().join("linked.txt"))?;
            assert!(request(port, "GET", "/linked.txt")?.starts_with("HTTP/1.1 403"));
        }
        drop(preview);
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
        Ok(())
    }
}
