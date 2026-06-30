---@class SaveFlow
local SaveFlow = {
    active_anchor_value = nil,
    pending_anchor_value = nil,
    is_bound = false,
}

local SAVE_TRIGGER_CHECKPOINT = engine.save.triggers.checkpoint

---@return RestoreLocation|nil
function SaveFlow.active_anchor()
    return SaveFlow.active_anchor_value
end

---@param anchor RestoreLocation|nil
---@return nil
function SaveFlow.set_active_anchor(anchor)
    SaveFlow.active_anchor_value = anchor
end

---@param location RestoreLocation
---@return nil
function SaveFlow.begin_checkpoint(location)
    SaveFlow.pending_anchor_value = location
end

---@param trigger string
---@return nil
function SaveFlow.handle_save_succeeded(trigger)
    if trigger == SAVE_TRIGGER_CHECKPOINT and SaveFlow.pending_anchor_value ~= nil then
        SaveFlow.active_anchor_value = SaveFlow.pending_anchor_value
        SaveFlow.pending_anchor_value = nil
    end
end

---@param trigger string
---@return nil
function SaveFlow.handle_save_failed(trigger)
    if trigger == SAVE_TRIGGER_CHECKPOINT then
        SaveFlow.pending_anchor_value = nil
    end
end

---@param transform table
---@param room_id integer
---@return RestoreLocation
function SaveFlow.capture_location(transform, room_id)
    return {
        world_id = engine.current_world().id,
        room_id = room_id,
        x = transform.position.x,
        y = transform.position.y,
    }
end

---@param progress table
---@param snapshot RestoreLocation
---@return table
function SaveFlow.build_save_document(progress, snapshot)
    return {
        progress = progress,
        snapshot = snapshot,
        active_anchor = SaveFlow.active_anchor_value,
    }
end

---@param target RestoreLocation
---@return nil
function SaveFlow.apply_restore_target(target)
    engine.restore_location(target)
end

---@param saved table
---@return RestoreLocation|nil
function SaveFlow.resolve_restore_target(saved)
    if saved.active_anchor ~= nil then
        return saved.active_anchor
    end
    return saved.snapshot
end

---@return nil
function SaveFlow.request_manual()
    engine.save.manual()
end

---@return nil
function SaveFlow.request_autosave()
    engine.save.auto()
end

---@param location RestoreLocation
---@return nil
function SaveFlow.request_checkpoint(location)
    SaveFlow.begin_checkpoint(location)
    engine.save.checkpoint()
end

---@return nil
function SaveFlow.bind_runtime_handlers()
    if SaveFlow.is_bound then
        return
    end

    engine.on(engine.events.save_succeeded, function(_, trigger)
        SaveFlow.handle_save_succeeded(trigger)
    end)

    engine.on(engine.events.save_failed, function(_, trigger, _)
        SaveFlow.handle_save_failed(trigger)
    end)

    SaveFlow.is_bound = true
end

return SaveFlow
