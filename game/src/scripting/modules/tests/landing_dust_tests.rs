use engine_core::ecs::{Grounded, comp_type_name};
use mlua::{Function, Lua, Table};
use std::cell::Cell;
use std::rc::Rc;

const LANDING_DUST_SOURCE: &str =
    include_str!("../../../../../games/Demo/Resources/scripts/landing_dust.lua");

#[test]
fn landing_dust_emits_once_on_each_landing_transition() -> mlua::Result<()> {
    let lua = Lua::new();
    let components = lua.create_table()?;
    let grounded_component = comp_type_name::<Grounded>();
    components.set(grounded_component, grounded_component)?;
    lua.globals().set("Components", components)?;

    let grounded = Rc::new(Cell::new(true));
    let parent = lua.create_table()?;
    let grounded_for_get = Rc::clone(&grounded);
    parent.set(
        "get",
        lua.create_function(move |_, (_this, _component): (Table, String)| {
            Ok(grounded_for_get.get())
        })?,
    )?;

    let bursts = Rc::new(Cell::new(0));
    let emitter = lua.create_table()?;
    let parent_for_emitter = parent.clone();
    emitter.set(
        "parent",
        lua.create_function(move |_, _this: Table| Ok(parent_for_emitter.clone()))?,
    )?;
    let bursts_for_emitter = Rc::clone(&bursts);
    emitter.set(
        "burst_particles",
        lua.create_function(move |_, _this: Table| {
            bursts_for_emitter.set(bursts_for_emitter.get() + 1);
            Ok(())
        })?,
    )?;

    let script: Table = lua.load(LANDING_DUST_SOURCE).eval()?;
    script.set("entity", emitter)?;
    let init: Function = script.get("init")?;
    let update: Function = script.get("update")?;
    init.call::<()>(script.clone())?;
    update.call::<()>((script.clone(), 1.0 / 60.0))?;
    assert_eq!(bursts.get(), 0);

    grounded.set(false);
    update.call::<()>((script.clone(), 1.0 / 60.0))?;
    grounded.set(true);
    update.call::<()>((script.clone(), 1.0 / 60.0))?;
    update.call::<()>((script.clone(), 1.0 / 60.0))?;
    assert_eq!(bursts.get(), 1);

    grounded.set(false);
    update.call::<()>((script.clone(), 1.0 / 60.0))?;
    grounded.set(true);
    update.call::<()>((script, 1.0 / 60.0))?;
    assert_eq!(bursts.get(), 2);
    Ok(())
}
