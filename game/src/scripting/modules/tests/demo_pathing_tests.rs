use mlua::{Function, Lua, Table};

const PATHING_SOURCE: &str = include_str!("../../../../../games/Demo/Resources/scripts/ai/pathing.lua");

#[test]
fn pathing_velocity_uses_cell_inside_bottom_center_actor() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(
        r#"
        engine = {
            get_transform = function()
                return { position = { x = 188, y = 72 } }
            end,
            grid = {
                cell_at = function(position)
                    return {
                        x = math.floor(position.x / 8),
                        y = math.floor(position.y / 8),
                    }
                end,
                cell_for_entity = function()
                    return { x = 23, y = 8 }
                end,
                next_step_towards = function()
                    return { x = 24, y = 8 }
                end,
            },
        }
        "#,
    )
    .exec()?;

    let pathing: Table = lua.load(PATHING_SOURCE).eval()?;
    let next_velocity: Function = pathing.get("next_velocity")?;
    let entity = lua.create_table()?;
    let target = lua.create_table()?;
    let config = lua.create_table()?;
    config.set("speed", 48.0)?;

    let velocity: Table = next_velocity.call((entity, target, config))?;

    assert_eq!(velocity.get::<f64>("x")?, 48.0);
    assert_eq!(velocity.get::<f64>("y")?, 0.0);
    Ok(())
}

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
