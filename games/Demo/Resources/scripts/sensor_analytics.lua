---@class SensorAnalytics
local SensorAnalytics = {
    enter_count = 0,
    stay_count = 0,
    exit_count = 0,
}

local function entity_id(entity)
    if entity == nil then
        return "nil"
    end

    return tostring(entity.id)
end

local function describe(event)
    return "kind="
        .. tostring(event.kind)
        .. " sensor="
        .. entity_id(event.sensor)
        .. " body="
        .. entity_id(event.body)
end

function SensorAnalytics:init()
    engine.on(engine.events.sensor_enter, function(event)
        self.enter_count = self.enter_count + 1
        engine.log.info(
            "[sensor analytics] enter #" .. tostring(self.enter_count) .. " " .. describe(event)
        )
    end)

    engine.on(engine.events.sensor_stay, function(event)
        self.stay_count = self.stay_count + 1
        engine.log.info(
            "[sensor analytics] stay #" .. tostring(self.stay_count) .. " " .. describe(event)
        )
    end)

    engine.on(engine.events.sensor_exit, function(event)
        self.exit_count = self.exit_count + 1
        engine.log.info(
            "[sensor analytics] exit #" .. tostring(self.exit_count) .. " " .. describe(event)
        )
    end)
end

return SensorAnalytics
