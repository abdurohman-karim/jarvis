-- command id -> application bundle name
local apps = {
    open_terminal = "Terminal",
    open_finder   = "Finder",
    open_music    = "Music",
    open_mail     = "Mail",
    open_calendar = "Calendar",
    open_settings = "System Settings",
}

local app = apps[jarvis.context.command_id]
if not app then
    jarvis.log("error", "no application mapped to " .. jarvis.context.command_id)
    return false
end

-- jarvis.system.open handles `open -a` for us and needs no shell access
if not jarvis.system.open("/System/Applications/" .. app .. ".app") then
    jarvis.system.open("/Applications/" .. app .. ".app")
end

return { chain = false }
