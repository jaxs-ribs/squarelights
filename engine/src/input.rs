//! Unity-style input snapshot for one frame. The original was authored for an
//! 800x480 screen; the host maps pointer coordinates into that space
//! (origin bottom-left, like Unity's Input.GetTouch().position).

pub const SCREEN_W: f32 = 800.0;
pub const SCREEN_H: f32 = 480.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Began,
    Moved,
    Stationary,
    Ended,
}

#[derive(Clone, Copy, Debug)]
pub struct Touch {
    pub x: f32,
    pub y: f32,
    pub phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left = 0,
    Right = 1,
    Up = 2,
    P = 3,
    R = 4,
    E = 5,
    S = 6,
    M = 7,
    C = 8,
    O = 9,
    T = 10,
    N = 11,
}

pub const KEY_COUNT: usize = 12;

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub held: [bool; KEY_COUNT],
    pub down: [bool; KEY_COUNT],
    pub touches: Vec<Touch>,
}

impl Input {
    pub fn get_key(&self, k: Key) -> bool {
        self.held[k as usize]
    }

    pub fn get_key_down(&self, k: Key) -> bool {
        self.down[k as usize]
    }
}

/// Accumulates host events between frames and produces per-frame snapshots,
/// so a key tapped between two frames still registers as GetKeyDown.
#[derive(Default)]
pub struct InputQueue {
    held: [bool; KEY_COUNT],
    pressed: [bool; KEY_COUNT],
    pointers: Vec<(i32, f32, f32, Phase)>,
}

impl InputQueue {
    pub fn key(&mut self, k: usize, down: bool) {
        if k >= KEY_COUNT {
            return;
        }
        if down && !self.held[k] {
            self.pressed[k] = true;
        }
        self.held[k] = down;
    }

    pub fn pointer(&mut self, id: i32, x: f32, y: f32, kind: u8) {
        // kind: 0 down, 1 move, 2 up
        match kind {
            0 => {
                self.pointers.retain(|p| p.0 != id);
                self.pointers.push((id, x, y, Phase::Began));
            }
            1 => {
                if let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) {
                    p.1 = x;
                    p.2 = y;
                    if p.3 != Phase::Began {
                        p.3 = Phase::Moved;
                    }
                }
            }
            _ => {
                if let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) {
                    p.1 = x;
                    p.2 = y;
                    // A touch that began and ended within one frame still reports Began first.
                    if p.3 != Phase::Began {
                        p.3 = Phase::Ended;
                    } else {
                        p.0 = -p.0 - 1_000_000;
                    }
                }
            }
        }
    }

    pub fn frame(&mut self) -> Input {
        let mut held = self.held;
        for k in 0..KEY_COUNT {
            // a tap shorter than one tick still counts as held for that tick
            held[k] |= self.pressed[k];
        }
        let input = Input {
            held,
            down: self.pressed,
            touches: self.pointers.iter().map(|p| Touch { x: p.1, y: p.2, phase: p.3 }).collect(),
        };
        self.pressed = [false; KEY_COUNT];
        let mut next = Vec::new();
        for p in self.pointers.drain(..) {
            match p.3 {
                Phase::Ended => {}
                Phase::Began if p.0 <= -1_000_000 => next.push((p.0, p.1, p.2, Phase::Ended)),
                _ => next.push((p.0, p.1, p.2, Phase::Stationary)),
            }
        }
        self.pointers = next;
        input
    }
}
