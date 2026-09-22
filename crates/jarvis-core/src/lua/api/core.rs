// Core Lua API: log, sleep, print, etc.

use mlua::{Lua, Table, MultiValue};

pub fn register(lua: &Lua, jarvis: &Table) -> mlua::Result<()> {

    // @ jarvis.log(level, message)
    // log something
    let log_fn = lua.create_function(|_, (level, message): (String, String)| {
        match level.to_lowercase().as_str() {
            "debug" => log::debug!("[Lua] {}", message),
            "info" => log::info!("[Lua] {}", message),
            "warn" => log::warn!("[Lua] {}", message),
            "error" => log::error!("[Lua] {}", message),
            _ => log::info!("[Lua] {}", message),
        }
        Ok(())
    })?;
    jarvis.set("log", log_fn)?;
    
    // @ jarvis.print(...)
    // simple print
    let print_fn = lua.create_function(|_, args: MultiValue| {
        let parts: Vec<String> = args.iter()
            .map(|v| format!("{:?}", v))
            .collect();
        log::info!("[Lua] {}", parts.join(" "));
        Ok(())
    })?;
    jarvis.set("print", print_fn)?;
    
    // @ jarvis.sleep(ms)
    // ..zZZ
    let sleep_fn = lua.create_function(|_, ms: u64| {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(())
    })?;
    jarvis.set("sleep", sleep_fn)?;
    
    // @ jarvis.speak(text)
    // Reads text out with the operating system's speech synthesis, so commands can say
    // things no voice pack recorded (the time, a battery level, an answer).
    let speak_fn = lua.create_function(|_, text: String| {
        log::info!("[Lua] SPEAK: {}", text);
        let language = crate::i18n::get_language();
        Ok(crate::speech::say(&text, &language))
    })?;
    jarvis.set("speak", speak_fn)?;

    // @ jarvis.ai(question) -> answer | nil, error
    // Asks the configured language model. Available to every sandbox level: it only
    // reaches the model, not the machine.
    #[cfg(feature = "ai")]
    {
        let ai_fn = lua.create_function(|lua, question: String| {
            let language = crate::i18n::get_language();
            match crate::ai::ask(&question, &language) {
                Ok(answer) => Ok((mlua::Value::String(lua.create_string(&answer)?), mlua::Value::Nil)),
                Err(e) => Ok((mlua::Value::Nil, mlua::Value::String(lua.create_string(&e.to_string())?))),
            }
        })?;
        jarvis.set("ai", ai_fn)?;
    }

    Ok(())
}