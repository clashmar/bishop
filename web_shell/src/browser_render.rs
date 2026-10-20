const RENDER_STARTUP_BEGAN: &str = "Bishop browser rendering startup began";
const WGPU_BLOCKED: &str =
    "Bishop wgpu browser path: blocked: winit 0.30.12 does not support wasm32-unknown-emscripten";
#[cfg(target_os = "emscripten")]
const FALLBACK_STARTED: &str = "Bishop browser fallback renderer: started";
#[cfg(target_os = "emscripten")]
const FALLBACK_RENDERED: &str = "Bishop browser fallback renderer: rendered";

pub fn run_browser_render_probe() {
    println!("{RENDER_STARTUP_BEGAN}");
    println!("{WGPU_BLOCKED}");
    render_fallback_probe();
}

#[cfg(target_os = "emscripten")]
fn render_fallback_probe() {
    println!("{FALLBACK_STARTED}");
    match crate::emscripten::run_script_int(RENDER_SCRIPT) {
        1 => println!("{FALLBACK_RENDERED}"),
        _ => println!(
            "Bishop browser fallback renderer: blocked: canvas drawing script returned failure"
        ),
    }
}

#[cfg(not(target_os = "emscripten"))]
fn render_fallback_probe() {
    println!("Bishop browser fallback renderer: skipped outside Emscripten");
}

#[cfg(target_os = "emscripten")]
const RENDER_SCRIPT: &str = r##"
(function () {
  const canvas = document.getElementById("bishop-canvas");
  if (!canvas) {
    window.bishopAppendStatus?.("Bishop browser fallback renderer: blocked: missing bishop-canvas");
    return 0;
  }

  const ctx = canvas.getContext("2d");
  if (!ctx) {
    window.bishopAppendStatus?.("Bishop browser fallback renderer: blocked: 2D canvas context unavailable");
    return 0;
  }

  const width = canvas.width;
  const height = canvas.height;

  ctx.fillStyle = "#0f172a";
  ctx.fillRect(0, 0, width, height);

  ctx.fillStyle = "#fbbf24";
  ctx.fillRect(72, 72, 220, 128);

  ctx.beginPath();
  ctx.moveTo(420, 70);
  ctx.lineTo(540, 290);
  ctx.lineTo(300, 290);
  ctx.closePath();
  ctx.fillStyle = "#22d3ee";
  ctx.fill();

  ctx.lineWidth = 6;
  ctx.strokeStyle = "#f8fafc";
  ctx.strokeRect(3, 3, width - 6, height - 6);

  ctx.font = "20px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
  ctx.fillStyle = "#f8fafc";
  ctx.fillText("Bishop Rust-driven render probe", 72, 236);

  window.bishopAppendStatus?.("Bishop browser fallback renderer: canvas primitive rendered");
  return 1;
})()
"##;

/// Draws the interactive browser scene without covering the asset marker.
#[cfg(target_os = "emscripten")]
pub(crate) fn render_interactive_frame(
    x: f32,
    y: f32,
    mx: f32,
    my: f32,
    button: i32,
    frame: u64,
) -> bool {
    let script = format!(
        r##"(function () {{
  const canvas = document.getElementById('bishop-canvas');
  const ctx = canvas?.getContext('2d');
  if (!ctx) return 0;

  ctx.save();
  ctx.beginPath();
  ctx.rect(0, 0, 640, 300);
  ctx.clip();
  ctx.fillStyle = '#0f172a';
  ctx.fillRect(0, 0, 640, 300);
  ctx.font = '16px monospace';
  ctx.fillStyle = '#f8fafc';
  ctx.fillText('Frames: {frame}', 16, 28);
  ctx.fillText('Click: {button} (1 left, 2 middle, 3 right)', 16, 53);
  ctx.fillStyle = ['#fbbf24', '#22d3ee', '#a78bfa', '#fb7185'][{button}];
  ctx.fillRect({x}, {y}, 20, 20);
  if ({mx} >= 0 && {mx} < 640 && {my} >= 0 && {my} < 300) {{
    ctx.strokeStyle = '#f8fafc';
    ctx.strokeRect({mx} - 4, {my} - 4, 8, 8);
  }}
  ctx.restore();
  return 1;
}})()"##
    );
    crate::emscripten::run_script_int(&script) == 1
}
