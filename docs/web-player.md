# Bishop Web Player

The reusable browser player runs the existing `game::Engine` on Bishop WebGL2. It loads a separate authored game bundle; the player is **not** compiled from that game's scripts or sprites.

## Build once, export a selected game

Activate Emscripten (`emcc` and `em++` on `PATH`) and install the Rust `wasm32-unknown-emscripten` target. From the workspace root:

```sh
cargo xtask build-web-player-runtime
cargo xtask export-web-game --resources games/Demo/Resources --out /tmp/bishop-demo-site
python3 /tmp/bishop-demo-site/preview.py
```

The launcher opens the browser at its printed `127.0.0.1` URL; press Ctrl-C to stop it. The release runtime is built once into `target/web-player-runtime/`; subsequent `export-web-game` invocations copy that **prebuilt** runtime and never run a compiler. To select the small independent authored integration game instead, run `cargo xtask export-web-game --resources xtask/tests/fixtures/web_smoke/Resources --out /tmp/bishop-web-smoke-site`, serve that directory and open its host page. Both exports contain the exact same player JS and WASM bytes; only their game bundle and host selection differ. If the runtime is missing, export fails with an instruction to build it. Export requires a **new** output directory and never merges into or overwrites one already present.

## Editor export and local preview

Before packaging an editor for authors, build the compatible player once with `cargo xtask build-web-player-runtime`. The macOS `cargo make release-editor-mac` flow copies it into the app's bundled resources; the Windows release task copies it next to the editor executable. Editors do **not** invoke Emscripten or Rust during export. For a local development editor you can also set `BISHOP_WEB_PLAYER_RUNTIME_DIR` to the prebuilt `target/web-player-runtime/` directory. A missing runtime produces a visible export error rather than a silent build.

Use **File → Export… → Web (browser player)** and select a parent folder. The editor creates **`<Game> Web Export`** inside it, saving the current game and edited menus before packaging. Overwrite replaces only that folder after staging succeeds; Cancel leaves it unchanged. **Preview** serves the new export on `127.0.0.1` and stops when the result panel or editor closes. To preview a previous export, use **File → Preview Web Export…** and choose its folder, or run `python3 preview.py` (`py preview.py` on Windows) from the export folder.

Packaged-editor exports with the current browser-save runtime and native binaries still require rebuilding and verification. Windows packaging and hosted delivery have not been verified.

The site has one entry page and two independent folders:

```text
index.html                         host-owned click-to-reveal example
preview.py                         local preview launcher
bishop-player/                     reusable player.html, config, input/audio JS, JS/WASM
  player.html
  theme.css
  fonts/gnf.regular.ttf
  player/config.mjs
  input.js
  audio.js
  web_shell.js
  web_shell.wasm
games/Demo/                       validated authored Resources plus versioned manifest
  manifest.ron
  game.ron
  ...
```

The example host creates an iframe **only on Load Player**, and its player fetches WASM and the game **only on Play Game**. Both clicks are deliberate; a site's existing button, modal or secret-code trigger can replace the example Load Player behavior. The standalone iframe URL is `http://127.0.0.1:<printed-port>/bishop-player/player.html?game=../games/Demo/manifest.ron`. The `game` URL is resolved against `player.html` and must be same-origin HTTP(S); do not use `file://`.

To use the runtime on a different site, copy `bishop-player/` unchanged, copy `games/<selected-game>/` separately, and point the iframe's `?game=` parameter to the selected manifest URL relative to `player.html`. Use a fresh iframe for each play session; removing it tears down the browser browsing context. The reusable player is not a `mount()` API or a hot-swap API.

## Browser saves

The player stores runtime saves in IndexedDB under the game's stable authored ID from `game.ron`, within the player's browser origin and profile. A save document and its latest pointer commit together; `save:succeeded` fires only after the transaction commits, while a failed write reports `save:failed` and does not replace the previous latest save. A menu element can declare `enabled_when: HasLatestSave`; the engine automatically disables it until a committed save exists, independently of its button action. An authored `Custom("load_latest")` action can invoke `engine.save.load_latest()` from a `menu:load_latest` listener without any menu-open callback. A new iframe or page reload can load the same save; it does not resume automatically.

Browser storage can be disabled or denied (including in private browsing or by quota). The game still starts and room traversal continues, but latest is unavailable and save requests report failure. Saves are local to that origin and browser profile; this is not cloud sync or a portable file export.

## Hosting, caching, and limitations

Upload the complete directory to static HTTPS hosting. Serve `.wasm` as `application/wasm`, JavaScript as a JavaScript MIME type, and `.ttf` as `font/ttf`. Keep the manifest and all its resource paths under the same origin. For immutable long-lived player caching, publish under a **versioned deployment path** (for example `/releases/<revision>/bishop-player/`) and change iframe URLs with each runtime update. Do not mark unversioned player files or a mutable `games/<game>/manifest.ron` immutable. Cache game data consistently with manifest updates.

Each v1 game is eagerly prefetched before Engine startup. The manifest declares at most **1,024 files and 32 MiB of game bytes**; requests are bounded to four at a time, and received byte counts must match the declaration. The player’s on-page status retains recent diagnostics; use the browser console for the full log when a game prints frequently. Full Demo parity, hosted release, speaker-verified audio and quit-to-title rebootstrap are **not** established. Play Game provides the audio gesture and focuses the canvas when ready; click the canvas to refocus controls if needed. Closing the iframe ends that play session.
