local Awareness = {}

---@param entity Entity
---@param options table?
---@return boolean
function Awareness.player_visible(entity, options)
    local player = engine.player()
    if not player or not player.entity then
        return false
    end

    local entity_transform = engine.get_transform(entity)
    local player_transform = engine.get_transform(player.entity)
    if not entity_transform or not player_transform then
        return false
    end

    return engine.spatial.has_line_of_sight(entity, player.entity, options)
end

return Awareness
