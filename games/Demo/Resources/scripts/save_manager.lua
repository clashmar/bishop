local autosave = require("autosave")
local save_flow = require("save_flow")

---@class SaveManager
local save_manager = {}

local function player_state()
    local player = engine.player()
    if not player then
        return nil, nil, nil
    end

    local transform = player.entity:get(Components.Transform)
    local room_id = player.entity:current_room()
    return player, transform, room_id
end

local function capture_snapshot(transform, room_id)
    return {
        world_id = engine.current_world().id,
        room_id = room_id,
        x = transform.position.x,
        y = transform.position.y,
    }
end

---@return nil
function save_manager.register_provider()
    engine.save.register_provider({
        id = "demo.progress",
        version = 2,
        capture = function()
            local player, transform, room_id = player_state()
            assert(player and transform and room_id, "player state unavailable during capture")
            return engine.save.to_string(save_flow.build_save_document({
                score = engine.game_manager:get_score(),
                level = engine.game_manager.public.level,
                health = player.public.health,
            }, capture_snapshot(transform, room_id)))
        end,
        apply = function(data)
            local player = engine.player()
            if not player then
                return
            end
            local saved = engine.save.from_string(data)
            if not saved then
                return
            end
            engine.game_manager.public.score = saved.progress.score
            engine.game_manager.public.level = saved.progress.level
            player.public.health = saved.progress.health
            save_flow.set_active_anchor(saved.active_anchor)
            local restore = save_flow.resolve_restore_target(saved)
            if restore ~= nil then
                engine.restore_location(
                    restore.world_id or engine.current_world().id,
                    restore.room_id,
                    restore.x,
                    restore.y
                )
            end
        end,
    })
end

---@return nil
function save_manager.bind_menu_actions()
    engine.on("menu:load_latest", function()
        engine.save.load_latest()
    end)

    engine.on("menu:manual_save", function()
        save_flow.request_manual()
        engine.log.info("Game saved")
        engine.menu.close()
    end)

    engine.on("menu:quit_title", function()
        engine.quit_to_title()
    end)
end

save_flow.bind_runtime_handlers()
save_manager.register_provider()
save_manager.bind_menu_actions()
autosave.configure({ tag = engine.tags.Autosave })

return save_manager
