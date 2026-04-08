# Getting Started

1. [**Download the latest release**](https://github.com/clashmar/bishop/blob/main/docs/releases.md).

2. **First launch** – the program will ask you where to create the *Bishop* root
   folder. You can move or re‑select this folder later with **File → Change Root Directory**.

3. **Project folder layout** – the editor creates a fixed structure that
   works out‑of‑the‑box for both platforms:
```
Your Game/
├─ mac_os/
│  └─ Icon.icns     # macOS application icon
├─ Resources/
│  ├─ assets/       # game assets (images, sounds, etc)
│  ├─ game.ron      # main game save file 
│  ├─ Icon.png      # icon for the game app window
│  └─ scripts/      # Lua scripts for gameplay
└─ windows/
   └─ Icon.ico      # windows application icon
```

## macOS

1. Unzip the downloaded `.app` bundle.
2. If macOS warns *“App can’t be checked for malicious software...”*:
* Open **System Settings → Privacy & Security**.
* Click **Open Anyway**.

## Windows

Unzip the downloaded `.exe` archive. All required files are already
embedded, so the editor can be run directly.

## Source Code

1. **Clone or fork the repository:**
```bash
git clone https://github.com/clashmar/bishop.git
cd bishop
```
2. **Install Rust** – follow the official installer at
   <https://www.rust-lang.org/tools/install>. The installer adds `cargo`,
   `rustc`, and the necessary toolchains.
3. **Install cargo-make**
```bash
cargo install cargo-make
```
4. **Make the Release**
```bash
cd editor
cargo make release-editor-mac
or
cargo make release-editor-windows
```

Cargo make ensures that the game binaries needed to playtest and export are built and copied to the correct location.
The save root for projects will be created at the root of the workspace at `./games`.



