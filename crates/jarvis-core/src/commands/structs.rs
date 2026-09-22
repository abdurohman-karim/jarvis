use std::{collections::HashMap, path::PathBuf, sync::Arc};
use serde::{Serialize, Deserialize};
use parking_lot::RwLock;

#[derive(Serialize, Deserialize, Debug)]
pub struct JCommandsList {
    #[serde(skip)]
    pub path: PathBuf,

    pub commands: Vec<JCommand>,
}



#[derive(Serialize, Deserialize, Debug)]
pub struct JCommand {
    pub id: String,

    // Available command types are: "lua", "ahk", "cli", "voice", "terminate", "stop_chaining"
    #[serde(rename = "type")]
    pub cmd_type: String,
    
    #[serde(default)]
    pub description: String,

    // Operating systems this command is available on ("windows", "macos", "linux").
    // Empty = every platform. Lets a pack ship an `ahk` variant for Windows and a
    // Lua variant of the same command id for the others.
    #[serde(default)]
    pub platforms: Vec<String>,
    
    // for "ahk" type
    #[serde(default)]
    pub exe_path: String,
    #[serde(default)]
    pub exe_args: Vec<String>,
    
    // for "cli" type
    #[serde(default)]
    pub cli_cmd: String,
    #[serde(default)]
    pub cli_args: Vec<String>,
    
    // #[serde(default)]
    // pub sounds: Vec<String>,

    // for "lua" type
    #[serde(default)]
    pub script: String,

    // Lua sandbox level: "minimal", "standard", "full"
    // basically this is an access level
    #[serde(default)]
    pub sandbox: String,

    // Script timeout in milliseconds (default 10000 = 10s)
    #[serde(default)]
    pub timeout: u64,

    // Multi-language sounds
    #[serde(default)]
    pub sounds: HashMap<String, Vec<String>>,

    // Multi-language phrases
    #[serde(default)]
    pub phrases: HashMap<String, Vec<String>>,

    // Slot definitions: slot_name -> how to extract it
    #[serde(default)]
    pub slots: HashMap<String, SlotDefinition>,

    // CACHE
    #[serde(skip, default)]
    sounds_cache: RwLock<HashMap<String, Arc<Vec<String>>>>,
    
    #[serde(skip, default)]
    phrases_cache: RwLock<HashMap<String, Arc<Vec<String>>>>,

    #[serde(skip, default)]
    normalized_cache: RwLock<HashMap<String, Arc<Vec<NormalizedPhrase>>>>,
}

// custom Clone 
// A command phrase decomposed once for repeated fuzzy comparisons
#[derive(Debug)]
pub struct NormalizedPhrase {
    pub text: String,
    pub chars: Vec<char>,
    pub word_chars: Vec<Vec<char>>,
}

impl NormalizedPhrase {
    fn new(phrase: &str) -> Self {
        let text = phrase.trim().to_lowercase();
        let chars: Vec<char> = text.chars().collect();
        let word_chars: Vec<Vec<char>> = text.split_whitespace().map(|w| w.chars().collect()).collect();
        Self { text, chars, word_chars }
    }
}

impl Clone for JCommand {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),

            cmd_type: self.cmd_type.clone(),
            description: self.description.clone(),
            platforms: self.platforms.clone(),

            exe_path: self.exe_path.clone(),
            exe_args: self.exe_args.clone(),

            cli_cmd: self.cli_cmd.clone(),
            cli_args: self.cli_args.clone(),

            script: self.script.clone(),
            sandbox: self.sandbox.clone(),
            timeout: self.timeout.clone(),

            sounds: self.sounds.clone(),
            phrases: self.phrases.clone(),

            slots: self.slots.clone(),

            // empty caches for cloned instance
            sounds_cache: RwLock::new(HashMap::new()),
            phrases_cache: RwLock::new(HashMap::new()),
            normalized_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl JCommand {
    // get phrases for current language
    pub fn get_phrases(&self, lang: &str) -> Arc<Vec<String>> {
        if let Some(cached) = self.phrases_cache.read().get(lang) {
            return Arc::clone(cached);
        }
        
        let result = Arc::new(self.resolve_localized(&self.phrases, lang));
        self.phrases_cache.write().insert(lang.to_string(), Arc::clone(&result));
        
        result
    }

    // get all phrases (for backwards compat)
    pub fn get_all_phrases(&self) -> Vec<String> {
        self.phrases.values().flatten().cloned().collect()
    }

    // Phrases prepared for fuzzy matching. The levenshtein fallback compares the spoken
    // text against every phrase of every command, and used to lowercase and decompose each
    // of them on every call; this is done once per language instead.
    pub fn get_normalized_phrases(&self, lang: &str) -> Arc<Vec<NormalizedPhrase>> {
        if let Some(cached) = self.normalized_cache.read().get(lang) {
            return Arc::clone(cached);
        }

        let normalized: Vec<NormalizedPhrase> = self
            .get_phrases(lang)
            .iter()
            .map(|phrase| NormalizedPhrase::new(phrase))
            .collect();

        let result = Arc::new(normalized);
        self.normalized_cache.write().insert(lang.to_string(), Arc::clone(&result));

        result
    }

    // get sounds for current language
    pub fn get_sounds(&self, lang: &str) -> Arc<Vec<String>> {
        if let Some(cached) = self.sounds_cache.read().get(lang) {
            return Arc::clone(cached);
        }
        
        let result = Arc::new(self.resolve_localized(&self.sounds, lang));
        self.sounds_cache.write().insert(lang.to_string(), Arc::clone(&result));
        
        result
    }

    // get all sounds (for backwards compat)
    pub fn get_all_sounds(&self) -> Vec<String> {
        self.sounds.values().flatten().cloned().collect()
    }


    // shared fallback
    fn resolve_localized(&self, map: &HashMap<String, Vec<String>>, lang: &str) -> Vec<String> {
        // exact match
        if let Some(values) = map.get(lang) {
            return values.clone();
        }

        // fallback to "en"
        if lang != "en" {
            if let Some(values) = map.get("en") {
                return values.clone();
            }
        }

        // fallback to first available
        map.values().next().cloned().unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SlotDefinition {
    // Entity label for GLiNER (e.g. "city name", "song title", "number")
    // This is a free-form description - GLiNER matches it semantically
    #[serde(default)]
    pub entity: String,

    // Optional: fallback context words for template-based extraction
    // e.g. ["in", "for", "at"] for a city slot
    #[serde(default)]
    pub context: Vec<String>,
}

// Extracted slot value passed to commands
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SlotValue {
    Text(String),
    Number(f64),
}

impl JCommand {
    // whether this command applies to the OS we are running on
    pub fn supports_current_platform(&self) -> bool {
        self.platforms.is_empty()
            || self.platforms.iter().any(|p| p.eq_ignore_ascii_case(std::env::consts::OS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(platforms: &[&str]) -> JCommand {
        let toml = format!(
            "id = \"x\"\ntype = \"lua\"\nplatforms = [{}]\n",
            platforms.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", ")
        );
        toml::from_str(&toml).unwrap()
    }

    #[test]
    fn platforms_filter() {
        assert!(cmd(&[]).supports_current_platform(), "no platforms = everywhere");
        assert!(cmd(&[std::env::consts::OS]).supports_current_platform());
        assert!(cmd(&[&std::env::consts::OS.to_uppercase()]).supports_current_platform(), "case-insensitive");
        assert!(!cmd(&["plan9"]).supports_current_platform());
    }
}
