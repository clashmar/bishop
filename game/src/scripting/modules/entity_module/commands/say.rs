use crate::game_global::push_command;
use crate::scripting::commands::text_commands::ShowSpeechCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_text;
use mlua::{Table, UserDataMethods};
use std::collections::HashMap;

pub struct SayMethod;

impl LuaMethod<EntityHandle> for SayMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(
            lua_text::SAY,
            |lua, this, (dialogue_id, key, opts): (String, String, Option<Table>)| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                ensure_live_entity(&game_instance.game.ecs, this.entity)?;
                let config = game_instance.game.text_manager.config.clone();

                let text = match game_instance
                    .game
                    .text_manager
                    .select_text(&dialogue_id, &key)
                {
                    Some(t) => t,
                    None => {
                        log::warn!("Dialogue not found: {}:{}", dialogue_id, key);
                        return Ok(());
                    }
                };
                drop(game_instance);

                let text = if let Some(ref opts_table) = opts {
                    if let Ok(vars_table) = opts_table.get::<Table>("vars") {
                        let mut vars = HashMap::new();
                        for (k, v) in vars_table.pairs::<String, String>().flatten() {
                            vars.insert(k, v);
                        }
                        interpolate(&text, &vars)
                    } else {
                        text
                    }
                } else {
                    text
                };

                let duration = opts
                    .as_ref()
                    .and_then(|t| t.get::<f32>("duration").ok())
                    .unwrap_or(config.default_duration);

                let color = opts.as_ref().and_then(|t| {
                    t.get::<Table>("color").ok().and_then(|c| {
                        Some([
                            c.get::<f32>(1).ok()?,
                            c.get::<f32>(2).ok()?,
                            c.get::<f32>(3).ok()?,
                            c.get::<f32>(4).ok().unwrap_or(1.0),
                        ])
                    })
                });

                let offset = opts.as_ref().and_then(|t| {
                    t.get::<Table>("offset")
                        .ok()
                        .and_then(|o| Some((o.get::<f32>(1).ok()?, o.get::<f32>(2).ok()?)))
                });

                let font_size = opts.as_ref().and_then(|t| t.get::<f32>("font_size").ok());
                let max_width = opts.as_ref().and_then(|t| t.get::<f32>("max_width").ok());
                let show_background = opts
                    .as_ref()
                    .and_then(|t| t.get::<bool>("show_background").ok());

                let background_color = opts.as_ref().and_then(|t| {
                    t.get::<Table>("background_color").ok().and_then(|c| {
                        Some([
                            c.get::<f32>(1).ok()?,
                            c.get::<f32>(2).ok()?,
                            c.get::<f32>(3).ok()?,
                            c.get::<f32>(4).ok().unwrap_or(0.7),
                        ])
                    })
                });

                push_command(Box::new(ShowSpeechCmd {
                    entity: this.entity,
                    text,
                    duration,
                    color,
                    offset,
                    font_size,
                    max_width,
                    show_background,
                    background_color,
                }));
                Ok(())
            },
        );
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Shows a speech bubble with text from a dialogue file.");
        out.line("---@param dialogue_id string The dialogue file ID (e.g. \"npc_merchant\")");
        out.line("---@param key string The dialogue key (e.g. \"greeting\")");
        out.line("---@param opts? {vars?: table<string, string>, duration?: number, color?: number[], offset?: number[], font_size?: number, max_width?: number, show_background?: boolean, background_color?: number[]}");
        out.line(&format!(
            "function Entity:{}(dialogue_id, key, opts) end",
            lua_text::SAY
        ));
        out.line("");
    }
}
