local lang = jarvis.context.language

-- networksetup works without extra privileges and on every recent macOS
local result = jarvis.system.exec("networksetup -getairportnetwork en0")
local network = result.success and result.stdout:match("Current Wi%-Fi Network: (.+)")

if network then
    network = network:gsub("%s+$", "")
    jarvis.speak((lang == "ru" and "Сеть " or lang == "ua" and "Мережа " or "You are on ") .. network)
else
    jarvis.speak(lang == "ru" and "Вайфай не подключен"
        or lang == "ua" and "Вайфай не підключено"
        or "Wi-Fi is not connected")
end

return { chain = false }
