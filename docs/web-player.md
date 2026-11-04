# Bishop Web Player (local static export)

This is the route to a reusable player and a separate authored game bundle. A browser build uses the existing `game::Engine` and Bishop WebGL2; it is **not** compiled from the selected game's scripts or sprites. The macOS packaged editor's Web Export and Preview have been exercised locally; Windows packaging and hosted delivery have not.

## Build once, export a selected game

Activate Emscripten (`emcc` and `em++` on `PATH`) and install the Rust `wasm32-unknown-emscripten` target. From the workspace root:

```sh
cargo xtask build-web-player-runtime
cargo xtask export-web-game --resources games/Demo/Resources --out /tmp/bishop-demo-site
python3 -m http.server --directory /tmp/bishop-demo-site 8000
```

Open `http://localhost:8000/`. The release runtime is built once into `target/web-player-runtime/`; subsequent `export-web-game` invocations copy that **prebuilt** runtime and never run a compiler. To select the small independent authored integration game instead, run `cargo xtask export-web-game --resources xtask/tests/fixtures/web_smoke/Resources --out /tmp/bishop-web-smoke-site`, serve that directory and open its host page. Both exports contain the exact same player JS and WASM bytes; only their game bundle and host selection differ. If the runtime is missing, export fails with an instruction to build it. Export requires a **new** output directory and never merges into or overwrites one already present.

## Editor export and local preview

Before packaging an editor for authors, build the compatible player once with `cargo xtask build-web-player-runtime`. The macOS `cargo make release-editor-mac` flow copies it into the app's bundled resources; the Windows release task copies it next to the editor executable. Editors do **not** invoke Emscripten or Rust during export. For a local development editor you can also set `BISHOP_WEB_PLAYER_RUNTIME_DIR` to the prebuilt `target/web-player-runtime/` directory. A missing runtime produces a visible export error rather than a silent build.

In the editor use **File → Export… → Web (browser player)**, then pick a parent folder. The export folder is named **Bishop Web Export** inside the selected parent. The target chooser also lists the native host package; other native platforms are shown unavailable. Export first saves the current game and any edited menus, or reports a save error without packaging stale content. Confirming overwrite replaces only that named export after staging succeeds; Cancel leaves it unchanged. After a successful Web export, **Preview** starts a local HTTP server on `127.0.0.1` with an automatically selected port and opens the browser; **Open folder** reveals the output. Closing the result panel or editor stops the preview server. If the browser cannot launch, the error is shown. On macOS, a bundled release app was verified to contain the prebuilt WASM and to export through the native folder picker. The loopback Preview served the complete 7,512,180-byte WASM response with `application/wasm`; Chrome rendered Demo's authored title and entered its room through New Game. Cancel preserved an existing export, confirmed overwrite replaced only the named export, Open folder showed the expected three site entries in Finder, and closing the panel refused subsequent connections to the preview port. This local observation does not verify Windows packaging or remote hosting.

The site has one entry page and two independent folders:

```text
index.html                         host-owned click-to-reveal example
bishop-player/                     reusable player.html, config, input/audio JS, JS/WASM
  player.html
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

The example host creates an iframe **only on Reveal**, and its player fetches WASM and the game **only on Play**. Both clicks are deliberate; a site's existing button, modal or secret-code trigger can replace the example Reveal behavior. The standalone iframe URL is `http://localhost:8000/bishop-player/player.html?game=../games/Demo/manifest.ron`. The `game` URL is resolved against `player.html` and must be same-origin HTTP(S); do not use `file://`.

To use the runtime on a different site, copy `bishop-player/` unchanged, copy `games/<selected-game>/` separately, and point the iframe's `?game=` parameter to the selected manifest URL relative to `player.html`. Use a fresh iframe for each play session; removing it tears down the browser browsing context. The reusable player is not a `mount()` API or a hot-swap API.

## Hosting, caching, and limitations

Upload the complete directory to static HTTPS hosting. Serve `.wasm` as `application/wasm` and JavaScript as a JavaScript MIME type. Keep the manifest and all its resource paths under the same origin. For immutable long-lived player caching, publish under a **versioned deployment path** (for example `/releases/<revision>/bishop-player/`) and change iframe URLs with each runtime update. Do not mark unversioned player files or a mutable `games/<game>/manifest.ron` immutable. Cache game data consistently with manifest updates.

Each v1 game is eagerly prefetched before Engine startup. The manifest declares at most **1,024 files and 32 MiB of game bytes**; requests are bounded to four at a time, and received byte counts must match the declaration. The browser title/menu and real game route work locally, but full Demo parity, hosted release, browser saves, speaker-verified audio and quit-to-title rebootstrap are **not** established. Click/focus the canvas for controls and a browser audio gesture. Closing the iframe ends that play session.

## Local release measurement (Demo)

Measured with a fresh headless Chrome profile against a local Python static server, then replayed in the same profile after closing and revealing a new iframe. These are **local proxies**, not hosted download/latency figures or an agreed site-impact budget. The server did not compress HTTP responses.

| Artifact | Raw bytes | gzip -6 bytes | SHA-256 |
| --- | ---: | ---: | --- |
| Player JS | 161,465 | 37,241 | `500f5b90ea17c836dd48bd659340661a206e53a510d949f4b7c67c8ae54c7b1d` |
| Player WASM | 7,512,180 | 2,433,662 | `6d13ce58846733becc52a5161a2953499ed72628219acd41180da1df692da0e6` |

- Cold Reveal + Play: 93 requests, 7,904,216 encoded network bytes, **707 ms** from Play click to first rendered title.
- Warm new iframe: 93 observed requests, **0 transferred bytes**, 89 cache-served flags, **310 ms** from Play to title.
- WASM linear memory at title: **17,891,328 bytes** in both runs. CDP top-page JS heap used: about **2.4–2.7 MB**. Chrome reported multiple renderer processes (roughly 49–205 MiB cold and 49–218 MiB warm); their RSS cannot be attributed solely to this game.

Demo's authored `menus/title.ron` refers to `save_manager.on_title_menu_open`, which its current `save_manager.lua` does not define. Chrome logs that authored callback error even though the title renders and New Game enters its room. Do not treat the first rendered room as full Demo parity. A numerical site-impact budget remains to be agreed after hosted measurements.

The separate `xtask/tests/fixtures/web_smoke/Resources` game exports as **WebSmoke** with its own Lantern Workshop title, room, player sprite and `scripts/main.lua` action. Local Chrome reached its title, entered its rendered room and logged `WEB_SMOKE_LANTERN_LIT` on T, using player JS/WASM with the exact SHA-256 hashes in the table above. Its cold host Reveal + Play took 19 requests, 7,697,388 transferred bytes and 431 ms to title; a warm new iframe transferred zero bytes and took 130 ms. In local negative checks, removing `menus/title.ron` yielded a visible path-bearing fetch error and setting bundle version 999 yielded an unsupported-version error; neither reached ready. These figures are local proxies, not site-impact or hosted-parity measurements.
