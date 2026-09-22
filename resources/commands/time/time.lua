local lang = jarvis.context.language
local hour = tonumber(jarvis.context.time.hour)
local minute = tonumber(jarvis.context.time.minute)

-- "14:05" is read as "fourteen oh five"; spell it out instead
local text
if lang == "ru" then
    text = string.format("Сейчас %d часов %d минут", hour, minute)
elseif lang == "ua" then
    text = string.format("Зараз %d годин %d хвилин", hour, minute)
else
    text = string.format("It is %d %02d", hour, minute)
end

jarvis.speak(text)
return { chain = false }
