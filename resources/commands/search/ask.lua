local lang = jarvis.context.language

local triggers = {
    "спроси у нейросети", "спроси искусственный интеллект", "вопрос нейросети",
    "запитай у нейромережі", "ask the ai", "ask the model",
}

local question = jarvis.context.phrase
local lowered = question:lower()
for _, trigger in ipairs(triggers) do
    if lowered:find(trigger, 1, true) then
        question = question:sub(#trigger + 1)
        break
    end
end
question = question:gsub("^%s+", ""):gsub("%s+$", "")

if question == "" then
    jarvis.speak(lang == "ru" and "О чём спросить?" or lang == "ua" and "Про що запитати?" or "What should I ask?")
    return { chain = true }
end

local answer, err = jarvis.ai(question)
if answer then
    jarvis.speak(answer)
else
    jarvis.log("warn", "ai: " .. tostring(err))
    jarvis.audio.play("error")
    return false
end

return { chain = false }
