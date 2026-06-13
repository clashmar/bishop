-- transport_zone.lua
-- "Reach a point" transport: when the player comes within `range` of this
-- entity, they are transported to the configured world. Place it away from
-- the destination's return point so re-entry does not immediately re-trigger.
---@class Script
local TransportZone = {
    public = {
        name = "TransportZone",
        world = "Second World",
        entry = "",
        range = 16.0,
    },

    update = function(self, dt)
        local player = engine.player()
        if player == nil then
            return
        end

        local player_transform = player.entity:get(Components.Transform)
        local zone_transform = self.entity:get(Components.Transform)
        if player_transform == nil or zone_transform == nil then
            return
        end

        local dx = player_transform.position.x - zone_transform.position.x
        local dy = player_transform.position.y - zone_transform.position.y
        if dx * dx + dy * dy > self.public.range * self.public.range then
            return
        end

        local entry = self.public.entry
        if entry == "" then
            entry = nil
        end

        engine.log.info("TransportZone: transporting player to " .. self.public.world)
        player.entity:move_to_world(self.public.world, entry)
    end,
}

return TransportZone
