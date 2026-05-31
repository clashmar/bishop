use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo xtask <subcommand>");
        return ExitCode::FAILURE;
    }

    match args[1].as_str() {
        "check-lua" => check_lua(),
        other => {
            eprintln!("Unknown subcommand: {other}");
            eprintln!("Available: check-lua");
            ExitCode::FAILURE
        }
    }
}

fn check_lua() -> ExitCode {
    let mut failed = false;
    let root = std::env::current_dir().expect("xtask must run from workspace root");

    match Command::new("luacheck")
        .args(["games/Demo/Resources/scripts/", "--no-color"])
        .status()
    {
        Ok(status) if status.success() => println!("luacheck: PASS"),
        Ok(_) => {
            eprintln!("luacheck: FAIL (see warnings above)");
            failed = true;
        }
        Err(e) => {
            eprintln!("luacheck: NOT FOUND ({e})");
            failed = true;
        }
    }

    let stylua_files = demo_lua_files(&root);
    let mut stylua = Command::new("stylua");
    stylua.arg("--check");
    for file in &stylua_files {
        stylua.arg(file);
    }

    match stylua.status() {
        Ok(status) if status.success() => println!("stylua: PASS"),
        Ok(_) => {
            eprintln!("stylua: FAIL (formatting differences found)");
            failed = true;
        }
        Err(e) => {
            eprintln!("stylua: NOT FOUND ({e})");
            failed = true;
        }
    }

    let config_path = root.join(".luarc.json");
    let mut luals = Command::new("lua-language-server");
    luals.arg(format!("--configpath={}", config_path.display()));
    luals.arg("--check=.");
    luals.arg("--check_format=pretty");

    match luals.status() {
        Ok(status) if status.success() => println!("lua-language-server: PASS"),
        Ok(_) => {
            eprintln!("lua-language-server: FAIL (type errors found)");
            failed = true;
        }
        Err(e) => {
            eprintln!("lua-language-server: NOT FOUND ({e})");
            failed = true;
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        println!("All Lua checks passed.");
        ExitCode::SUCCESS
    }
}

fn demo_lua_files(root: &PathBuf) -> Vec<String> {
    let scripts_dir = root.join("games/Demo/Resources/scripts");
    let mut files: Vec<String> = fs::read_dir(&scripts_dir)
        .expect("Demo scripts directory should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "lua"))
        .map(|path| path.display().to_string())
        .collect();
    files.sort();
    files
}
