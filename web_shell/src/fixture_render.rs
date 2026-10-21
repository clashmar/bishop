/// Draw the scripted fixture's target and status on the existing fallback canvas.
pub(crate) fn render(goal_x: f32, goal_y: f32, won: bool) -> bool {
    let label = if won {
        "You won - R to reset"
    } else {
        "Reach the goal"
    };
    let script = format!(
        r##"(function () {{
  const ctx = document.getElementById('bishop-canvas')?.getContext('2d');
  if (!ctx) return 0;
  ctx.save();
  ctx.beginPath(); ctx.rect(0, 0, 640, 300); ctx.clip();
  ctx.strokeStyle = '#86efac'; ctx.lineWidth = 4;
  ctx.strokeRect({goal_x}, {goal_y}, 20, 20);
  ctx.font = '16px monospace'; ctx.fillStyle = '#f8fafc';
  ctx.fillText('{label}', 16, 80);
  ctx.restore();
  return 1;
}})()"##
    );
    crate::emscripten::run_script_int(&script) == 1
}
