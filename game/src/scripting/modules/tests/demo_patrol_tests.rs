use mlua::{Function, Lua, Table};

const PATROL_SOURCE: &str = include_str!("../../../../../games/Demo/Resources/scripts/ai/patrol.lua");

#[test]
fn patrol_step_when_crossing_max_x_stops_and_reverses_before_exiting() -> mlua::Result<()> {
    let lua = Lua::new();
    let patrol: Table = lua.load(PATROL_SOURCE).eval()?;
    let choose_step: Function = patrol.get("choose_step")?;
    let state = lua.create_table()?;
    state.set("direction", 1)?;
    let config = lua.create_table()?;
    config.set("speed", 32.0)?;
    config.set("position_x", 222.0)?;
    config.set("min_x", 144.0)?;
    config.set("max_x", 224.0)?;
    config.set("wait_time", 0.3)?;

    let step: Table = choose_step.call((state.clone(), config, 0.1))?;

    assert_eq!(step.get::<f64>("x")?, 0.0);
    assert_eq!(state.get::<i64>("direction")?, -1);
    Ok(())
}

#[test]
fn flee_when_crossing_min_x_is_outside_patrol_bounds() -> mlua::Result<()> {
    let lua = Lua::new();
    let patrol: Table = lua.load(PATROL_SOURCE).eval()?;
    let would_exceed_bounds: Option<Function> = patrol.get("would_exceed_bounds")?;
    assert!(would_exceed_bounds.is_some());

    let outside: bool = would_exceed_bounds
        .ok_or_else(|| mlua::Error::RuntimeError("missing patrol bounds check".to_string()))?
        .call((50.0, -1, 44.0, 0.1, 48.0, 112.0))?;

    assert!(outside);
    Ok(())
}
