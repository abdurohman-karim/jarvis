use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Duration;
use std::process::{Child, Command};

use seqdiff::ratio;

mod structs;
pub use structs::*;

use std::sync::Arc;

use crate::{config, i18n, APP_DIR, COMMANDS_LIST};

#[cfg(feature = "lua")]
use crate::lua::{self, SandboxLevel, CommandContext};

pub fn parse_commands() -> Result<Vec<JCommandsList>, String> {
    let mut commands: Vec<JCommandsList> = Vec::new();

    let commands_path = APP_DIR.join(config::COMMANDS_PATH);
    let cmd_dirs = fs::read_dir(&commands_path)
        .map_err(|e| format!("Error reading commands directory {:?}: {}", commands_path, e))?;

    for entry in cmd_dirs.flatten() {
        let cmd_path = entry.path();
        let toml_file = cmd_path.join("command.toml");
        
        if !toml_file.exists() {
            continue;
        }
        
        let content = match fs::read_to_string(&toml_file) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {}: {}", toml_file.display(), e);
                continue;
            }
        };

        let file: JCommandsList = match toml::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to parse {}: {}", toml_file.display(), e);
                continue;
            }
        };

        let (supported, skipped): (Vec<JCommand>, Vec<JCommand>) = file.commands
            .into_iter()
            .partition(|c| c.supports_current_platform());
        for cmd in &skipped {
            debug!("Skipping command '{}' ({}): not for {}", cmd.id, toml_file.display(), std::env::consts::OS);
        }
        if supported.is_empty() {
            continue;
        }

        commands.push(JCommandsList {
            path: cmd_path,
            commands: supported,
        });
    }

    if commands.is_empty() {
        Err("No commands found".into())
    } else {
        info!("Loaded {} command pack(s)", commands.len());
        Ok(commands)
    }
}


// current command packs (snapshot)
pub fn list() -> Arc<Vec<JCommandsList>> {
    COMMANDS_LIST.read().clone()
}

pub fn set_list(commands: Vec<JCommandsList>) {
    *COMMANDS_LIST.write() = Arc::new(commands);
}

// re-read all command packs from disk and replace the current list
pub fn reload() -> Result<Arc<Vec<JCommandsList>>, String> {
    let commands = parse_commands()?;
    info!("Commands reloaded. Count: {}, List: {:?}", commands.len(), list_paths(&commands));
    set_list(commands);
    Ok(list())
}

pub fn commands_hash(commands: &[JCommandsList]) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    
    let lang = i18n::get_language();
    hasher.update(lang.as_bytes());
    hasher.update(b"|");

    // collect all command ids and phrases for current language, sorted
    let mut all_data: Vec<(&str, _)> = commands.iter()
        .flat_map(|ac| ac.commands.iter().map(|c| (c.id.as_str(), c.get_phrases(&lang))))
        .collect();
    all_data.sort_by_key(|(id, _)| *id);
    
    for (id, phrases) in all_data {
        hasher.update(id.as_bytes());
        for phrase in phrases.iter() {
            hasher.update(phrase.as_bytes());
        }
    }
    
    format!("{:x}", hasher.finalize())
}


pub fn fetch_command<'a>(
    phrase: &str,
    commands: &'a [JCommandsList],
) -> Option<(&'a PathBuf, &'a JCommand)> {
    let lang = i18n::get_language();

    let phrase = phrase.trim().to_lowercase();
    if phrase.is_empty() {
        return None;
    }

    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_words: Vec<&str> = phrase.split_whitespace().collect();

    let mut result: Option<(&PathBuf, &JCommand)> = None;
    let mut best_score = config::CMD_RATIO_THRESHOLD;

    for cmd_list in commands {
        for cmd in &cmd_list.commands {
            let cmd_phrases = cmd.get_normalized_phrases(&lang);

            for cmd_phrase in cmd_phrases.iter() {
                // character-level similarity
                let char_ratio = ratio(&phrase_chars, &cmd_phrase.chars);

                // word-level similarity
                let word_score = word_overlap_score(&phrase_words, &cmd_phrase.word_chars);

                // combined score
                let score = (char_ratio * 0.6) + (word_score * 0.4);
                
                // early exit on perfect match
                if score >= 99.0 {
                    debug!("Perfect match: '{}' -> '{}'", phrase, cmd_phrase.text);
                    return Some((&cmd_list.path, cmd));
                }
                
                if score > best_score {
                    best_score = score;
                    result = Some((&cmd_list.path, cmd));
                }
            }
        }
    }

    if let Some((_, cmd)) = result {
        info!("Fuzzy match: '{}' -> cmd '{}' (score: {:.1}%)", phrase, cmd.id, best_score);
    } else {
        debug!("No match for '{}' (best: {:.1}%)", phrase, best_score);
    }
    
    result
}


fn word_overlap_score(input_words: &[&str], cmd_word_chars: &[Vec<char>]) -> f64 {
    if input_words.is_empty() || cmd_word_chars.is_empty() {
        return 0.0;
    }

    let mut matched = 0.0;

    for input_word in input_words {
        let input_chars: Vec<char> = input_word.chars().collect();
        
        let best_word_match = cmd_word_chars
            .iter()
            .map(|cw| ratio(&input_chars, cw))
            .fold(0.0_f64, f64::max);
        
        if best_word_match > 70.0 {
            matched += best_word_match / 100.0;
        }
    }

    let max_words = input_words.len().max(cmd_word_chars.len()) as f64;
    (matched / max_words) * 100.0
}




pub fn execute_exe(exe: &str, args: &[String]) -> std::io::Result<Child> {
    Command::new(exe).args(args).spawn()
}

pub fn execute_cli(cmd: &str, args: &[String]) -> std::io::Result<Child> {
    debug!("Spawning: cmd /C {} {:?}", cmd, args);

    if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(cmd).args(args).spawn()
    } else {
        Command::new("sh").arg("-c").arg(cmd).args(args).spawn()
    }
}

pub fn execute_command(cmd_path: &PathBuf, cmd_config: &JCommand, phrase: Option<&str>, slots: Option<&HashMap<String, SlotValue>>) -> Result<bool, String> {
    // execute command by the type
    match cmd_config.cmd_type.as_str() {

        // BRUH
        "voice" => Ok(true),
        
        // LUA command
        #[cfg(feature = "lua")]
        "lua" => {
            execute_lua_command(cmd_path, cmd_config, phrase, slots)
        }

        // AutoHotkey command
        // @TODO: Consider adding ahk source files execution?
        "ahk" => {
            let exe_path_absolute = Path::new(&cmd_config.exe_path);
            let exe_path_local = cmd_path.join(&cmd_config.exe_path);

            let exe_path = if exe_path_absolute.exists() {
                exe_path_absolute
            } else {
                exe_path_local.as_path()
            };

            execute_exe(exe_path.to_str().unwrap(), &cmd_config.exe_args)
                .map(|_| true)
                .map_err(|e| format!("AHK process spawn error: {}", e))
        }
        
        // CLI command type
        // @TODO: Consider security restrictions
        "cli" => {
            execute_cli(&cmd_config.cli_cmd, &cmd_config.cli_args)
                .map(|_| true)
                .map_err(|e| format!("CLI command error: {}", e))
        }
        
        // TERMINATOR command (T1000)
        "terminate" => {
            std::thread::sleep(Duration::from_secs(2));
            std::process::exit(0);
        }
        
        // STOP CHANING
        "stop_chaining" => Ok(false),

        // other
        _ => {
            error!("Command type unknown: {}", cmd_config.cmd_type);
            Err(format!("Command type unknown: {}", cmd_config.cmd_type).into())
        }
    }
}

// look up a command by its ID
pub fn get_command_by_id<'a>(
    commands: &'a [JCommandsList],
    id: &str,
) -> Option<(&'a PathBuf, &'a JCommand)> {
    for cmd_list in commands {
        for cmd in &cmd_list.commands {
            if cmd.id == id {
                return Some((&cmd_list.path, cmd));
            }
        }
    }
    None
}

pub fn list_paths(commands: &[JCommandsList]) -> Vec<&Path> {
    commands.iter().map(|x| x.path.as_path()).collect()
}

#[cfg(feature = "lua")]
fn execute_lua_command(
    cmd_path: &PathBuf,
    cmd_config: &JCommand,
    phrase: Option<&str>,
    slots: Option<&HashMap<String, SlotValue>>
) -> Result<bool, String> {
    // get script path

    let script_name = if cmd_config.script.is_empty() {
        "script.lua"
    } else {
        &cmd_config.script
    };
    
    let script_path = cmd_path.join(script_name);
    
    if !script_path.exists() {
        return Err(format!("Lua script not found: {}", script_path.display()));
    }
    
    // parse sandbox level
    let sandbox = SandboxLevel::from_str(&cmd_config.sandbox);

    // create context
    let context = CommandContext {
        phrase: phrase.unwrap_or("").to_string(),
        command_id: cmd_config.id.clone(),
        command_path: cmd_path.clone(),
        language: i18n::get_language(),
        slots: slots.map(|s| s.clone()),
    };
    
    // get timeout
    let timeout = Duration::from_millis(cmd_config.timeout);
    
    info!("Executing Lua command: {} (sandbox: {:?}, timeout: {:?})", 
          cmd_config.id, sandbox, timeout);
    
    // execute
    match lua::execute(&script_path, context, sandbox, timeout) {
        Ok(result) => {
            info!("Lua command {} completed (chain: {})", cmd_config.id, result.chain);
            Ok(result.chain)
        }
        Err(e) => {
            error!("Lua command {} failed: {}", cmd_config.id, e);
            Err(e.to_string())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // every command pack shipped in resources/commands must parse
    fn packs() -> Vec<JCommandsList> {
        let toml = r#"
            [[commands]]
            id = "browser_open"
            type = "lua"
            phrases.en = ["open browser", "launch browser"]
            phrases.ru = ["открой браузер"]

            [[commands]]
            id = "counter"
            type = "lua"
            phrases.en = ["counter", "count"]
            phrases.ru = ["счётчик"]
        "#;
        let list: JCommandsList = toml::from_str(toml).unwrap();
        vec![JCommandsList { path: PathBuf::from("/tmp/test"), commands: list.commands }]
    }

    // i18n is not initialized in unit tests, so matching runs against DEFAULT_LANGUAGE ("en")
    #[test]
    fn fetch_command_exact_and_fuzzy() {
        let packs = packs();

        let (_, cmd) = fetch_command("open browser", &packs).expect("exact phrase");
        assert_eq!(cmd.id, "browser_open");

        // case / whitespace insensitive
        let (_, cmd) = fetch_command("  Launch Browser ", &packs).expect("normalized phrase");
        assert_eq!(cmd.id, "browser_open");

        // small recognition error still matches
        let (_, cmd) = fetch_command("open brouser", &packs).expect("fuzzy phrase");
        assert_eq!(cmd.id, "browser_open");
    }

    #[test]
    fn fetch_command_rejects_unrelated_text() {
        let packs = packs();
        assert!(fetch_command("what time is it", &packs).is_none());
        assert!(fetch_command("", &packs).is_none());
    }

    #[test]
    fn get_command_by_id_works() {
        let packs = packs();
        assert!(get_command_by_id(&packs, "counter").is_some());
        assert!(get_command_by_id(&packs, "nope").is_none());
    }

    // The same id may appear twice when the variants are for different operating systems
    // (an AutoHotkey one for Windows, a Lua one for macOS): it is one intent with two
    // implementations. Two variants that can be active at once are a mistake - the intent
    // classifier would train one intent from both and command lookup would pick either.
    #[test]
    fn command_ids_do_not_collide_on_one_platform() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/commands");
        let mut seen: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

        for entry in fs::read_dir(&root).expect("resources/commands missing").flatten() {
            let toml_file = entry.path().join("command.toml");
            if !toml_file.exists() {
                continue;
            }
            let content = fs::read_to_string(&toml_file).unwrap();
            let parsed: JCommandsList = toml::from_str(&content).unwrap();

            for cmd in parsed.commands {
                seen.entry(cmd.id.clone())
                    .or_default()
                    .push((toml_file.display().to_string(), cmd.platforms.clone()));
            }
        }

        for (id, variants) in seen {
            for (i, (pack_a, platforms_a)) in variants.iter().enumerate() {
                for (pack_b, platforms_b) in variants.iter().skip(i + 1) {
                    let overlap = platforms_a.is_empty()
                        || platforms_b.is_empty()
                        || platforms_a.iter().any(|a| platforms_b.iter().any(|b| a.eq_ignore_ascii_case(b)));

                    assert!(!overlap,
                        "command id '{}' is active on the same platform in {} ({:?}) and {} ({:?})",
                        id, pack_a, platforms_a, pack_b, platforms_b);
                }
            }
        }
    }

    // every command pack shipped in resources/commands must parse
    #[test]
    #[test]
    fn shipped_command_packs_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/commands");
        let mut checked = 0;
        let mut available_here = 0;

        for entry in fs::read_dir(&root).expect("resources/commands missing").flatten() {
            let toml_file = entry.path().join("command.toml");
            if !toml_file.exists() {
                continue;
            }
            let content = fs::read_to_string(&toml_file).unwrap();
            let parsed: Result<JCommandsList, _> = toml::from_str(&content);
            assert!(parsed.is_ok(), "{}: {}", toml_file.display(), parsed.err().unwrap());

            let commands = parsed.unwrap().commands;
            for cmd in &commands {
                assert!(!cmd.id.is_empty(), "{}: command without id", toml_file.display());
                assert!(!cmd.phrases.is_empty(), "{}: '{}' has no phrases", toml_file.display(), cmd.id);
            }
            // a pack may legitimately be for another OS (the AutoHotkey ones are Windows only)
            if commands.iter().any(|c| c.supports_current_platform()) {
                available_here += 1;
            }
            checked += 1;
        }

        assert!(checked > 0, "no command packs found in {}", root.display());
        assert!(available_here > 0, "no command pack works on {}", std::env::consts::OS);
    }
}
