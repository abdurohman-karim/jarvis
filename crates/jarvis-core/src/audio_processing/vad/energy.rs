// Adaptive energy-based VAD.
//
// Frame loudness is measured as RMS in dBFS and compared against a noise floor that
// tracks the environment: the floor follows quiet frames quickly and loud frames very
// slowly, so speech does not drag it up while a fan turning on eventually does.
// Voice starts when the level exceeds floor + ON margin for a couple of frames (ignores
// clicks) and ends when it drops below floor + OFF margin for a few frames (hangover),
// which gives the state machine hysteresis instead of flickering around one threshold.

use crate::config;

const SILENCE_DB: f32 = -96.0; // level assigned to an all-zero frame

pub struct EnergyVad {
    noise_floor_db: f32,
    frames_seen: u32,
    is_voice: bool,
    // consecutive frames above / below the thresholds
    above_frames: u32,
    below_frames: u32,
}

impl EnergyVad {
    pub fn new() -> Self {
        Self {
            noise_floor_db: config::VAD_ENERGY_INITIAL_FLOOR_DB,
            frames_seen: 0,
            is_voice: false,
            above_frames: 0,
            below_frames: 0,
        }
    }

    // returns (is_voice, confidence 0..1)
    pub fn detect(&mut self, input: &[i16]) -> (bool, f32) {
        let level = rms_dbfs(input);
        self.frames_seen += 1;

        // thresholds relative to the current floor, never below the absolute minimum
        let on_threshold = (self.noise_floor_db + config::VAD_ENERGY_ON_MARGIN_DB)
            .max(config::VAD_ENERGY_MIN_DB);
        let off_threshold = (self.noise_floor_db + config::VAD_ENERGY_OFF_MARGIN_DB)
            .max(config::VAD_ENERGY_MIN_DB - config::VAD_ENERGY_ON_MARGIN_DB + config::VAD_ENERGY_OFF_MARGIN_DB);

        if self.is_voice {
            if level < off_threshold {
                self.below_frames += 1;
                if self.below_frames >= config::VAD_ENERGY_HANGOVER_FRAMES {
                    self.is_voice = false;
                    self.below_frames = 0;
                    self.above_frames = 0;
                }
            } else {
                self.below_frames = 0;
            }
        } else {
            if level > on_threshold {
                self.above_frames += 1;
                if self.above_frames >= config::VAD_ENERGY_ONSET_FRAMES {
                    self.is_voice = true;
                    self.above_frames = 0;
                    self.below_frames = 0;
                }
            } else {
                self.above_frames = 0;
            }
        }

        self.update_floor(level);

        let confidence = ((level - off_threshold) / (on_threshold - off_threshold + 6.0)).clamp(0.0, 1.0);
        (self.is_voice, confidence)
    }

    // The floor follows quieter frames quickly and louder frames slowly; while speech is
    // detected it still creeps up, very slowly, so a sound that never stops (a fan that
    // just switched on) eventually becomes background instead of "voice" forever.
    fn update_floor(&mut self, level: f32) {
        let alpha = if self.frames_seen <= config::VAD_ENERGY_CALIBRATION_FRAMES {
            // first fraction of a second: converge fast on whatever the room sounds like
            0.5
        } else if level < self.noise_floor_db {
            config::VAD_ENERGY_FLOOR_DOWN_ALPHA
        } else if self.is_voice {
            config::VAD_ENERGY_FLOOR_UP_ALPHA_DURING_VOICE
        } else {
            config::VAD_ENERGY_FLOOR_UP_ALPHA
        };

        self.noise_floor_db += alpha * (level - self.noise_floor_db);
    }

    // end of an utterance / command: forget the voice state, keep the room's noise floor
    pub fn reset(&mut self) {
        self.is_voice = false;
        self.above_frames = 0;
        self.below_frames = 0;
    }

    pub fn noise_floor_db(&self) -> f32 {
        self.noise_floor_db
    }
}

// frame loudness in dB relative to full scale (0 dBFS = square wave at max amplitude)
pub fn rms_dbfs(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return SILENCE_DB;
    }

    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms = (sum / samples.len() as f64).sqrt();

    if rms < 1.0 {
        return SILENCE_DB;
    }

    (20.0 * (rms / 32768.0).log10() as f32).max(SILENCE_DB)
}

#[cfg(test)]
mod tests {
    use super::*;

    // deterministic pseudo-noise frame with the given RMS level (dBFS)
    fn frame(level_db: f32, seed: &mut u32) -> Vec<i16> {
        let amplitude = 32768.0 * 10f32.powf(level_db / 20.0) * 2f32.sqrt();
        (0..512)
            .map(|_| {
                *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let x = (*seed >> 8) as f32 / (1u32 << 24) as f32; // 0..1
                (amplitude * (x * 2.0 - 1.0)) as i16
            })
            .collect()
    }

    fn run(vad: &mut EnergyVad, level_db: f32, frames: usize, seed: &mut u32) -> bool {
        let mut voice = false;
        for _ in 0..frames {
            voice = vad.detect(&frame(level_db, seed)).0;
        }
        voice
    }

    #[test]
    fn silence_is_not_voice() {
        let mut vad = EnergyVad::new();
        let zeros = vec![0i16; 512];
        for _ in 0..50 {
            assert!(!vad.detect(&zeros).0);
        }
    }

    #[test]
    fn steady_noise_adapts_and_is_not_voice() {
        let mut vad = EnergyVad::new();
        let mut seed = 1;
        // loud, constant background (fan) at -35 dBFS, well above the old fixed threshold
        let voice = run(&mut vad, -35.0, 200, &mut seed);
        assert!(!voice, "steady background noise must not count as voice");
        assert!((vad.noise_floor_db() - -35.0).abs() < 3.0, "floor should track the noise: {}", vad.noise_floor_db());
    }

    #[test]
    fn speech_above_noise_is_voice_and_ends_with_hangover() {
        let mut vad = EnergyVad::new();
        let mut seed = 7;
        run(&mut vad, -50.0, 100, &mut seed);
        assert!(run(&mut vad, -30.0, 5, &mut seed), "speech 20 dB above the floor must be voice");
        // floor must not have been dragged up by the speech
        assert!(vad.noise_floor_db() < -45.0, "floor drifted to {}", vad.noise_floor_db());
        // one quiet frame is not the end (hangover)
        assert!(run(&mut vad, -50.0, 1, &mut seed));
        assert!(!run(&mut vad, -50.0, 10, &mut seed));
    }

    #[test]
    fn constant_noise_starting_mid_session_becomes_background() {
        let mut vad = EnergyVad::new();
        let mut seed = 5;
        run(&mut vad, -55.0, 100, &mut seed);
        // a fan switches on: 20 dB louder, forever
        assert!(run(&mut vad, -35.0, 5, &mut seed), "initially it looks like voice");
        assert!(!run(&mut vad, -35.0, 600, &mut seed), "after ~20 s it must be background");
        // and real speech above the new floor still works
        assert!(run(&mut vad, -20.0, 5, &mut seed));
    }

    #[test]
    fn quiet_room_quiet_speech() {
        let mut vad = EnergyVad::new();
        let mut seed = 3;
        run(&mut vad, -75.0, 100, &mut seed);
        // soft speech well above the absolute minimum is voice...
        assert!(run(&mut vad, -45.0, 5, &mut seed));
        assert!(!run(&mut vad, -75.0, 10, &mut seed));
        // ...but rustle below VAD_ENERGY_MIN_DB is not, even in a dead-quiet room
        assert!(!run(&mut vad, -58.0, 20, &mut seed));
    }

    #[test]
    fn single_click_is_ignored() {
        let mut vad = EnergyVad::new();
        let mut seed = 11;
        run(&mut vad, -60.0, 100, &mut seed);
        assert!(!run(&mut vad, -20.0, 1, &mut seed), "a single loud frame must not start voice");
        assert!(!run(&mut vad, -60.0, 3, &mut seed));
    }
}
