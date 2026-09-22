local lang = jarvis.context.language
local result = jarvis.system.exec("pmset -g batt")

if not result.success then
    jarvis.log("error", "pmset: " .. (result.stderr or "failed"))
    return false
end

local percent = result.stdout:match("(%d+)%%")
if not percent then
    jarvis.speak(lang == "ru" and "Не вижу батарею, похоже это десктоп"
        or lang == "ua" and "Не бачу батареї"
        or "I cannot see a battery")
    return { chain = false }
end

local charging = result.stdout:find("AC Power") ~= nil

local text
if lang == "ru" then
    text = "Заряд " .. percent .. " процентов" .. (charging and ", питание от сети" or "")
elseif lang == "ua" then
    text = "Заряд " .. percent .. " відсотків" .. (charging and ", живлення від мережі" or "")
else
    text = "Battery is at " .. percent .. " percent" .. (charging and ", charging" or "")
end

jarvis.speak(text)
return { chain = false }
