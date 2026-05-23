use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use mlua::Lua;

use crate::save_system::{LuaSaveProvider, SaveProvider, SavedSection};

#[test]
fn lua_save_provider_capture_returns_saved_section() {
    let lua = Lua::new();
    let capture = lua.create_function(|_, ()| Ok(String::from("room=2"))).unwrap();
    let apply = lua.create_function(|_, _: String| Ok(())).unwrap();
    let mut provider = LuaSaveProvider::new(&lua, "game.flags", 7, capture, apply).unwrap();

    let section = provider.capture().unwrap();

    assert_eq!(provider.id().as_str(), "game.flags");
    assert_eq!(section.version, 7);
    assert_eq!(section.data, "room=2");
}

#[test]
fn lua_save_provider_apply_passes_saved_data_to_lua() {
    let lua = Lua::new();
    let applied = Rc::new(RefCell::new(String::new()));
    let applied_clone = applied.clone();
    let capture = lua.create_function(|_, ()| Ok(String::from("ignored"))).unwrap();
    let apply = lua
        .create_function(move |_, data: String| {
            *applied_clone.borrow_mut() = data;
            Ok(())
        })
        .unwrap();
    let mut provider = LuaSaveProvider::new(&lua, "game.flags", 1, capture, apply).unwrap();

    provider
        .apply(&SavedSection {
            version: 1,
            data: String::from("room=5"),
        })
        .unwrap();

    assert_eq!(&*applied.borrow(), "room=5");
}

#[test]
fn lua_save_provider_lua_error_is_wrapped_as_io_error() {
    let lua = Lua::new();
    let capture = lua
        .create_function(|_, ()| Err::<String, _>(mlua::Error::RuntimeError("oops".into())))
        .unwrap();
    let apply = lua.create_function(|_, _: String| Ok(())).unwrap();
    let mut provider = LuaSaveProvider::new(&lua, "game.flags", 1, capture, apply).unwrap();

    let err = provider.capture().unwrap_err();

    assert_eq!(err.kind(), io::ErrorKind::Other);
    let msg = format!("{}", err);
    assert!(msg.contains("oops"), "error message should contain the Lua error: {msg}");
}
