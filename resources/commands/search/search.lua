-- everything after the trigger words is the query
local triggers = {
    "найди информацию о", "найди в интернете", "поищи в интернете", "загугли",
    "знайди в інтернеті", "пошукай в інтернеті",
    "search the web for", "look up", "google",
}

local query = jarvis.context.phrase:lower()
for _, trigger in ipairs(triggers) do
    query = query:gsub(trigger, "")
end
query = query:gsub("^%s+", ""):gsub("%s+$", "")

if query == "" then
    jarvis.audio.play("not_found")
    return false
end

-- percent-encode everything that is not unreserved
local encoded = query:gsub("[^%w%-%._~]", function(c)
    return string.format("%%%02X", string.byte(c))
end)

jarvis.system.open("https://duckduckgo.com/?q=" .. encoded)
return { chain = false }
