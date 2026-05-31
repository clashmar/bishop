---@class Autosave
local Autosave = {
    configured_tag = nil,
    condition = nil,
    is_bound = false,
}

local function has_tag(tags, configured_tag)
    for _, tag in ipairs(tags) do
        if tag == configured_tag then
            return true
        end
    end
    return false
end

local function bind_listener()
    if Autosave.is_bound then
        return
    end

    engine.on(engine.events.room_entered, function(_, ...)
        if not Autosave.configured_tag then
            return
        end

        local tags = { ... }
        if
            has_tag(tags, Autosave.configured_tag)
            and (not Autosave.condition or Autosave.condition())
        then
            engine.save.auto()
        end
    end)

    Autosave.is_bound = true
end

--- Configure autosave behavior.
---@param cfg {tag: string, condition?: fun(): boolean}
---@return nil
function Autosave.configure(cfg)
    assert(type(cfg) == "table", "Autosave.configure expects a config table")
    assert(type(cfg.tag) == "string", "Autosave.configure requires a string tag")
    if cfg.condition ~= nil then
        assert(type(cfg.condition) == "function", "Autosave.configure condition must be a function")
    end

    Autosave.configured_tag = cfg.tag
    Autosave.condition = cfg.condition
    bind_listener()
end

return Autosave
