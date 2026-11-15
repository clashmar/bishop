use mlua::{Function, Lua, Table};

const PATHING_SOURCE: &str = include_str!("../../../../../games/Demo/Resources/scripts/ai/pathing.lua");

#[test]
fn dog_follow_target_when_player_jumps_stays_on_dog_grid_row() -> mlua::Result<()> {
    let lua = Lua::new();
    let pathing: Table = lua.load(PATHING_SOURCE).eval()?;
    let target_on_actor_row: Option<Function> = pathing.get("target_on_actor_row")?;
    assert!(target_on_actor_row.is_some());

    let actor_cell = lua.create_table()?;
    actor_cell.set("x", 20)?;
    actor_cell.set("y", 9)?;
    let player_cell = lua.create_table()?;
    player_cell.set("x", 8)?;
    player_cell.set("y", 5)?;

    let target: Table = target_on_actor_row
        .ok_or_else(|| mlua::Error::RuntimeError("missing ground-follow projection".to_string()))?
        .call((actor_cell, player_cell))?;

    assert_eq!(target.get::<i64>("x")?, 8);
    assert_eq!(target.get::<i64>("y")?, 9);
    Ok(())
}
