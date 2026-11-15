local Awareness = require("ai.awareness")

local AwarenessProbe = {
    public = {
        player_visible = false,
        last_seen = { x = 0, y = 0 },
    },
}

function AwarenessProbe.update(self)
    local parent = self.entity:parent()
    if not parent then
        self.public.player_visible = false
        return
    end

    local visible = Awareness.player_visible(parent)
    self.public.player_visible = visible
    if not visible then
        return
    end

    local player = engine.player()
    if not player or not player.entity then
        return
    end
    local transform = engine.get_transform(player.entity)
    if transform then
        self.public.last_seen = {
            x = transform.position.x,
            y = transform.position.y,
        }
    end
end

return AwarenessProbe
