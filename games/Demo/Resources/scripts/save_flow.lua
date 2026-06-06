---@class SaveFlow
local SaveFlow = {
    active_anchor_value = nil,
    pending_anchor_value = nil,
    is_bound = false,
}

local SAVE_TRIGGER_CHECKPOINT = engine.save.triggers.checkpoint

---@return table|nil
function SaveFlow.active_anchor()
    return SaveFlow.active_anchor_value
end

---@param anchor table|nil
---@return nil
function SaveFlow.set_active_anchor(anchor)
    SaveFlow.active_anchor_value = anchor
end

---@param anchor table
---@return nil
function SaveFlow.begin_checkpoint(anchor)
    SaveFlow.pending_anchor_value = anchor
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

---@param progress table
---@param snapshot table
---@return table
function SaveFlow.build_save_document(progress, snapshot)
    return {
        progress = progress,
        snapshot = snapshot,
        active_anchor = SaveFlow.active_anchor_value,
    }
end

---@param saved table
---@return table|nil
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

---@param anchor table
---@return nil
function SaveFlow.request_checkpoint(anchor)
    SaveFlow.begin_checkpoint(anchor)
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
