//! Shared runtime services every scene uses: time, Invoke scheduling, RNG,
//! PlayerPrefs, one-shot sounds and music state.

use std::collections::BTreeMap;

use crate::input::Input;
use crate::world::World;

pub const FIXED_DT: f32 = 0.02;

/// Unity's Random.Range semantics on top of a small deterministic PRNG.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Random.Range(int min, int max): max exclusive.
    pub fn range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        min + ((self.next() >> 33) % (max - min) as u64) as i32
    }
}

#[derive(Clone, Debug, Default)]
pub struct Prefs {
    pub map: BTreeMap<String, i32>,
    pub dirty: bool,
}

impl Prefs {
    pub fn get_int(&self, k: &str) -> i32 {
        *self.map.get(k).unwrap_or(&0)
    }

    pub fn has_key(&self, k: &str) -> bool {
        self.map.contains_key(k)
    }

    pub fn set_int(&mut self, k: &str, v: i32) {
        if self.map.get(k) != Some(&v) {
            self.map.insert(k.to_string(), v);
            self.dirty = true;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct Music {
    pub clip: &'static str,
    pub volume: f32,
    pub state: MusicState,
    pub mute: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneId {
    Menu,
    Game,
}

pub struct Invoke<C> {
    pub due: f64,
    pub rate: Option<f64>,
    pub call: C,
}

pub struct Ctx<C> {
    pub w: World,
    pub input: Input,
    pub rng: Rng,
    pub prefs: Prefs,
    pub sounds: Vec<&'static str>,
    pub music: Music,
    /// Scaled game time (Time.time); stops while Time.timeScale == 0.
    pub time: f64,
    pub time_scale: f32,
    pub frame: u64,
    pub invokes: Vec<Invoke<C>>,
    pub load_request: Option<SceneId>,
}

impl<C: Copy> Ctx<C> {
    pub fn new(w: World, rng: Rng, prefs: Prefs, music: Music) -> Ctx<C> {
        Ctx {
            w,
            input: Input::default(),
            rng,
            prefs,
            sounds: Vec::new(),
            music,
            time: 0.0,
            time_scale: 1.0,
            frame: 0,
            invokes: Vec::new(),
            load_request: None,
        }
    }

    pub fn invoke(&mut self, call: C, delay: f64) {
        self.invokes.push(Invoke { due: self.time + delay, rate: None, call });
    }

    pub fn invoke_repeating(&mut self, call: C, delay: f64, rate: f64) {
        self.invokes.push(Invoke { due: self.time + delay, rate: Some(rate), call });
    }

    /// Pops every invoke due at the current time, in due order. Repeating
    /// invokes run at most once per frame, like Unity's.
    pub fn due_invokes(&mut self) -> Vec<C> {
        let now = self.time + 1e-9;
        let mut due: Vec<(f64, usize, C)> = Vec::new();
        let mut keep = Vec::with_capacity(self.invokes.len());
        for (i, mut inv) in self.invokes.drain(..).enumerate() {
            if inv.due <= now {
                due.push((inv.due, i, inv.call));
                if let Some(rate) = inv.rate {
                    inv.due += rate;
                    while inv.due <= now {
                        inv.due += rate;
                    }
                    keep.push(inv);
                }
            } else {
                keep.push(inv);
            }
        }
        self.invokes = keep;
        due.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        due.into_iter().map(|d| d.2).collect()
    }

    pub fn play(&mut self, clip: &'static str) {
        if self.prefs.get_int("sound") == 1 {
            self.sounds.push(clip);
        }
    }
}
