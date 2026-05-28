local menu = require("_engine.menus")

local autosave = require("autosave")

local save_manager = {}

local function player_state()
    local player = engine.player()
    if not player then
        return nil, nil, nil
    end

    local transform = player.entity:get("Transform")
    local room_id = player.entity:current_room()
    return player, transform, room_id
end

function save_manager.register_provider()
    engine.save.register_provider({
        id = "demo.progress",
        version = 1,
        capture = function()
            local player, transform, room_id = player_state()
            assert(player and transform and room_id, "player state unavailable during capture")
            return engine.save.to_string({
                score = engine.game_manager:get_score(),
                level = engine.game_manager.public.level,
                health = player.public.health,
                room_id = room_id,
                x = transform.position.x,
                y = transform.position.y,
            })
        end,
        apply = function(data)
            local player = engine.player()
            if not player then
                return
            end
            local progress = engine.save.from_string(data)
            engine.game_manager.public.score = progress.score
            engine.game_manager.public.level = progress.level
            player.public.health = progress.health
            player.entity:move_to_room(progress.room_id)
            player.entity:teleport({ x = progress.x, y = progress.y })
        end,
    })
end

function save_manager.bind_menu_actions()
    engine.on("menu:load_latest", function()
        engine.save.load_latest()
    end)

    engine.on("menu:manual_save", function()
        engine.save.manual()
        engine.log.info("Game saved")
        engine.menu.close()
    end)

    engine.on("menu:quit_title", function()
        engine.quit_to_title()
    end)
end

save_manager.register_provider()
save_manager.bind_menu_actions()
autosave.configure({ tag = engine.tags.autosave })

return save_manager
