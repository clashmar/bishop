---@class SensorProbe
local SensorProbe = {}

local function entity_id(entity)
    if entity == nil then
        return "nil"
    end

    return tostring(entity.id)
end

local function describe(event)
    return "kind="
        .. tostring(event.kind)
        .. " role="
        .. tostring(event.role)
        .. " self="
        .. entity_id(event.self_entity)
        .. " other="
        .. entity_id(event.other)
        .. " sensor="
        .. entity_id(event.sensor)
        .. " body="
        .. entity_id(event.body)
end

function SensorProbe:init()
    engine.on(engine.events.sensor_enter, function(event)
        engine.log.info("[global sensor enter] " .. describe(event))
    end)

    engine.on(engine.events.sensor_stay, function(event)
        engine.log.info("[global sensor stay] " .. describe(event))
    end)

    engine.on(engine.events.sensor_exit, function(event)
        engine.log.info("[global sensor exit] " .. describe(event))
    end)
end

function SensorProbe:on_sensor_enter(event)
    engine.log.info("[local sensor enter] " .. describe(event))
end

function SensorProbe:on_sensor_stay(event)
    engine.log.info("[local sensor stay] " .. describe(event))
end

function SensorProbe:on_sensor_exit(event)
    engine.log.info("[local sensor exit] " .. describe(event))
end

return SensorProbe
