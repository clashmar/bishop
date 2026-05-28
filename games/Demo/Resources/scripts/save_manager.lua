local menu = require("_engine.menus")

local save_manager = {
    last_room_id = nil,
    autosave_transitions = {
        ["1->2"] = true,
    },
}

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
            if not progress then
                return
            end
            engine.game_manager.public.score = progress.score
            engine.game_manager.public.level = progress.level
            player.public.health = progress.health
            player.entity:move_to_room(progress.room_id)
            player.entity:teleport({ x = progress.x, y = progress.y })
        end,
    })
end

function save_manager.on_title_menu_open()
    engine.menu.set_enabled(menu.Title, menu.Title.LoadGame, engine.save.has_latest())
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
end

function save_manager.update()
    local _, _, room_id = player_state()
    if not room_id then
        return
    end

    if save_manager.last_room_id ~= nil and save_manager.last_room_id ~= room_id then
        local transition_key = string.format("%d->%d", save_manager.last_room_id, room_id)
        if save_manager.autosave_transitions[transition_key] then
            engine.save.auto()
        end
    end

    save_manager.last_room_id = room_id
end

save_manager.register_provider()
save_manager.bind_menu_actions()

return save_manager
