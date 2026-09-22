local lang = jarvis.context.language
local day = tonumber(jarvis.context.time.day)
local month = tonumber(jarvis.context.time.month)

local months = {
    ru = {"января","февраля","марта","апреля","мая","июня","июля","августа","сентября","октября","ноября","декабря"},
    ua = {"січня","лютого","березня","квітня","травня","червня","липня","серпня","вересня","жовтня","листопада","грудня"},
    en = {"January","February","March","April","May","June","July","August","September","October","November","December"},
}

local names = months[lang] or months.en
local text
if lang == "en" then
    text = string.format("Today is %s %d", names[month], day)
else
    text = string.format("%s %d %s", lang == "ru" and "Сегодня" or "Сьогодні", day, names[month])
end

jarvis.speak(text)
return { chain = false }
