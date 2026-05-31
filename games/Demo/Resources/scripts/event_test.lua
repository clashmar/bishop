---@class EventTest
local EventTest = {
    init = function(self)
        engine.on("demo_event", function(arg1, arg2)
            engine.log.info(
                "[event_test] demo_event received: " .. tostring(arg1) .. ", " .. tostring(arg2)
            )
        end)
    end,

    fire = function(self)
        engine.log.info("[event_test] firing demo_event")
        engine.emit("demo_event", 123, "payload")
    end,
}

return EventTest
