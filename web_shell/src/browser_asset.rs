#[cfg(target_os = "emscripten")]
use std::path::Path;

use web_shell::DEMO_TITLE_ASSET_PATH;

const LOAD_STARTED: &str = "Bishop browser asset load started";
#[cfg(target_os = "emscripten")]
const ASSET_PROOF_KEY: &str = "DemoTitle";

/// Starts the browser proof that fetches and validates the staged Demo title asset.
pub fn start_browser_asset_probe() {
    println!("{LOAD_STARTED}");
    println!("Bishop browser asset path: {DEMO_TITLE_ASSET_PATH}");
    start_asset_load();
}

#[cfg(target_os = "emscripten")]
fn start_asset_load() {
    crate::browser_fetch::fetch(DEMO_TITLE_ASSET_PATH, |result| match result {
        Ok(bytes) => asset_loaded(&bytes),
        Err(error) => asset_load_failed(&error),
    });
}

#[cfg(not(target_os = "emscripten"))]
fn start_asset_load() {
    println!("Bishop browser asset load skipped outside Emscripten");
}

#[cfg(target_os = "emscripten")]
fn asset_loaded(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    println!("Bishop browser asset loaded: {} bytes", bytes.len());

    match asset_proof_summary(&text) {
        Some(summary) => {
            println!("Bishop browser asset proof: {summary}");
            draw_asset_success(bytes.len(), &summary);
        }
        None => {
            let reason = format!("{} did not contain {ASSET_PROOF_KEY}", asset_file_name());
            println!("Bishop browser asset load failed: {reason}");
            draw_asset_failure(&reason);
        }
    }
}

#[cfg(target_os = "emscripten")]
fn asset_load_failed(reason: &str) {
    println!("Bishop browser asset load failed: {reason}");
    draw_asset_failure(reason);
}

#[cfg(target_os = "emscripten")]
fn asset_proof_summary(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim_start().starts_with(ASSET_PROOF_KEY))
        .map(|line| format!("{} contains {}", asset_file_name(), line.trim()))
}

#[cfg(target_os = "emscripten")]
fn asset_file_name() -> &'static str {
    Path::new(DEMO_TITLE_ASSET_PATH)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("Demo title browser asset path must have a UTF-8 filename")
}

#[cfg(target_os = "emscripten")]
#[derive(Clone, Copy)]
struct AssetMarkerStyle {
    background: &'static str,
    stroke: &'static str,
    text: &'static str,
}

#[cfg(target_os = "emscripten")]
const SUCCESS_MARKER_STYLE: AssetMarkerStyle = AssetMarkerStyle {
    background: "#052e16",
    stroke: "#86efac",
    text: "#dcfce7",
};

#[cfg(target_os = "emscripten")]
const FAILURE_MARKER_STYLE: AssetMarkerStyle = AssetMarkerStyle {
    background: "#450a0a",
    stroke: "#fca5a5",
    text: "#fee2e2",
};

#[cfg(target_os = "emscripten")]
fn draw_asset_success(byte_count: usize, summary: &str) {
    let title = format!("Asset loaded: {byte_count} bytes");
    draw_asset_marker(
        &title,
        summary,
        SUCCESS_MARKER_STYLE,
        Some("Bishop browser asset canvas marker rendered"),
    );
}

#[cfg(target_os = "emscripten")]
fn draw_asset_failure(reason: &str) {
    draw_asset_marker("Asset load failed", reason, FAILURE_MARKER_STYLE, None);
}

#[cfg(target_os = "emscripten")]
fn draw_asset_marker(
    title: &str,
    detail: &str,
    style: AssetMarkerStyle,
    rendered_status: Option<&str>,
) {
    let title = js_string_literal(title);
    let detail = js_string_literal(detail);
    let rendered_status = rendered_status.map(js_string_literal).unwrap_or_else(|| "null".into());
    let background = style.background;
    let stroke = style.stroke;
    let text = style.text;
    let script = format!(
        r##"
(function () {{
  const canvas = document.getElementById("bishop-canvas");
  if (!canvas) {{
    window.bishopAppendStatus?.("Bishop browser asset canvas update failed: missing bishop-canvas");
    return 0;
  }}

  const ctx = canvas.getContext("2d");
  if (!ctx) {{
    window.bishopAppendStatus?.("Bishop browser asset canvas update failed: 2D canvas context unavailable");
    return 0;
  }}

  const title = {title};
  const detail = {detail};
  const renderedStatus = {rendered_status};
  ctx.fillStyle = "{background}";
  ctx.fillRect(48, 302, 544, 44);
  ctx.strokeStyle = "{stroke}";
  ctx.lineWidth = 2;
  ctx.strokeRect(48, 302, 544, 44);
  ctx.font = "16px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
  ctx.fillStyle = "{text}";
  ctx.fillText(title, 64, 322);
  ctx.fillText(detail, 64, 340);
  if (renderedStatus) {{
    window.bishopAppendStatus?.(renderedStatus);
  }}
  return 1;
}})()
"##
    );
    if crate::emscripten::run_script_int(&script) != 1 {
        println!("Bishop browser asset canvas update failed: drawing script returned failure");
    }
}

#[cfg(target_os = "emscripten")]
fn js_string_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
