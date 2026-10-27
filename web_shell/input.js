window.bishopInstallInput = function () {
  const canvas = document.getElementById("bishop-canvas");
  if (!canvas || !window.Module?.ccall) return false;
  if (canvas.dataset.bishopInputInstalled === "true") return true;
  canvas.dataset.bishopInputInstalled = "true";

  const call = (name, types, args) => window.Module.ccall(name, "number", types, args);
  const active = () => document.activeElement === canvas && !document.hidden && document.hasFocus();
  const focusChanged = () => call("bishop_focus_changed", ["number"], [active() ? 1 : 0]);
  const position = event => {
    const rect = canvas.getBoundingClientRect();
    return [(event.clientX - rect.left - canvas.clientLeft) * canvas.width / canvas.clientWidth,
            (event.clientY - rect.top - canvas.clientTop) * canvas.height / canvas.clientHeight];
  };

  canvas.addEventListener("focus", focusChanged);
  canvas.addEventListener("blur", focusChanged);
  window.addEventListener("focus", focusChanged);
  window.addEventListener("blur", () => call("bishop_focus_changed", ["number"], [0]));
  document.addEventListener("visibilitychange", focusChanged);

  canvas.addEventListener("keydown", event => {
    if (!active()) return;
    const handled = call("bishop_key_down", ["string", "string", "number"],
                         [event.code, event.key, event.repeat ? 1 : 0]);
    if (handled && (event.code.startsWith("Arrow") || event.code === "Space")) event.preventDefault();
  });
  canvas.addEventListener("keyup", event => {
    call("bishop_key_up", ["string"], [event.code]);
  });
  canvas.addEventListener("mousedown", event => {
    canvas.focus();
    if (!active()) return;
    const [x, y] = position(event);
    call("bishop_mouse_down", ["number", "number", "number", "number"],
         [event.button, x, y, performance.now()]);
    if (event.button === 1 || event.button === 2) event.preventDefault();
  });
  document.addEventListener("mouseup", event => {
    call("bishop_mouse_up", ["number"], [event.button]);
  });
  canvas.addEventListener("mousemove", event => {
    const [x, y] = position(event);
    call("bishop_mouse_move", ["number", "number"], [x, y]);
  });
  canvas.addEventListener("wheel", event => {
    if (!active() || event.deltaMode === 2) return;
    call("bishop_wheel", ["number", "number", "number"],
         [event.deltaX, event.deltaY, event.deltaMode]);
    event.preventDefault();
  }, { passive: false });
  canvas.addEventListener("contextmenu", event => event.preventDefault());

  focusChanged();
  return true;
};
