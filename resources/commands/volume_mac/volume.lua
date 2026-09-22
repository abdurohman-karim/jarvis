-- one script for every volume command; the id says what to do
local id = jarvis.context.command_id
local step = 12

local script
if id == "volume_up" then
    script = "set volume output volume (output volume of (get volume settings) + " .. step .. ")"
elseif id == "volume_down" then
    script = "set volume output volume (output volume of (get volume settings) - " .. step .. ")"
elseif id == "volume_mute" then
    script = "set volume with output muted"
else
    script = "set volume without output muted"
end

local result = jarvis.system.exec("osascript -e " .. string.format("%q", script))
if not result.success then
    jarvis.log("error", "volume: " .. (result.stderr or "failed"))
    return false
end

return { chain = false }
