local id = jarvis.context.command_id
local lang = jarvis.context.language

if id == "lock_screen" then
    jarvis.system.exec("pmset displaysleepnow")

elseif id == "sleep_display" then
    jarvis.system.exec("pmset displaysleepnow")

elseif id == "screenshot" then
    -- desktop of the current user, named by date
    local name = os.date("screenshot-%Y-%m-%d-%H%M%S.png")
    local target = os.getenv("HOME") .. "/Desktop/" .. name
    local result = jarvis.system.exec("screencapture -x " .. string.format("%q", target))

    if result.success then
        jarvis.speak(lang == "ru" and "Скриншот на рабочем столе"
            or lang == "ua" and "Знімок на робочому столі"
            or "Screenshot is on your desktop")
    else
        jarvis.log("error", "screencapture: " .. (result.stderr or "failed"))
        return false
    end
end

return { chain = false }
