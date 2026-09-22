use std::{collections::HashMap, path::PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {

    #[serde(skip)]
    pub path: PathBuf,
    
    pub voice: VoiceMeta,

    // Multi-language reactions
    pub reactions: HashMap<String, VoiceReactions>,

    // Recording used as the reference when cloning this voice, per language.
    #[serde(default)]
    pub clone: HashMap<String, CloneReference>,
}

impl VoiceConfig {
    // Reference clip for cloning this voice in `language`, if the pack declares one.
    pub fn clone_reference(&self, language: &str) -> Option<&CloneReference> {
        self.clone.get(language)
    }
}

// A clip of this voice plus what is said in it: cloning needs both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneReference {
    // path relative to the voice pack, e.g. "ru/joke2.mp3"
    pub reference: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMeta {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub author: String,

    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceReactions {
    // app startup (time-based or generic)
    #[serde(default)]
    pub greet: Vec<String>,

    #[serde(default)]
    pub greet_morning: Vec<String>,
    #[serde(default)]
    pub greet_day: Vec<String>,
    #[serde(default)]
    pub greet_evening: Vec<String>,
    #[serde(default)]
    pub greet_night: Vec<String>,
    
    // wake word detected
    #[serde(default)]
    pub reply: Vec<String>,

    // command executed
    #[serde(default)]
    pub ok: Vec<String>,
    
    // command not found
    #[serde(default)]
    pub not_found: Vec<String>,

    // thank you
    #[serde(default)]
    pub thanks: Vec<String>,
    
    // error
    #[serde(default)]
    pub error: Vec<String>,
    
    // shutdown
    #[serde(default)]
    pub goodbye: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Reaction {
    Greet,      // app startup
    Reply,      // wake word detected
    Ok,         // command executed
    NotFound,
    Thanks,
    Error,
    Goodbye,
}
