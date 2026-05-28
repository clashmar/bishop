local autosave = {
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
    if autosave.is_bound then
        return
    end

    engine.on(engine.events.room_entered, function(_, ...)
        if not autosave.configured_tag then
            return
        end

        local tags = { ... }
        if has_tag(tags, autosave.configured_tag)
            and (not autosave.condition or autosave.condition()) then
            engine.save.auto()
        end
    end)

    autosave.is_bound = true
end

function autosave.configure(cfg)
    assert(type(cfg) == "table", "autosave.configure expects a config table")
    assert(type(cfg.tag) == "string", "autosave.configure requires a string tag")
    if cfg.condition ~= nil then
        assert(type(cfg.condition) == "function", "autosave.configure condition must be a function")
    end

    autosave.configured_tag = cfg.tag
    autosave.condition = cfg.condition
    bind_listener()
end

return autosave
