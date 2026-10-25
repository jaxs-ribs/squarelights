//! CharacterMovement, CharacterTexture, InputManager, TBL*, CameraTracking,
//! CameraBorders and MusicController.

use super::{Call, GameScene};
use crate::input::{Key, Phase};
use crate::math::{smooth_damp, v3, V3};
use crate::runtime::{MusicState, FIXED_DT};
use crate::world::Space;

/// Landing rows: index 0 is the C# "1" row (lowest), 5 is "6" (highest).
#[derive(Default, Clone)]
pub struct CharacterMovement {
    pub node: usize,
    pub movement: f32,
    pub movement_speed: f32,
    pub fall_time: i32,
    pub jump: i32,
    pub time_elapsed: i32,
    pub grav_force: f32,
    pub height: i32,
    pub jump_power: f32,
    pub mov_timer: i32,
    pub super_jump: f32,
    pub bumping: i32,
    pub gravity: i32,
    pub lives: i32,
    pub height_max: i32,
    pub x_min: [f32; 6],
    pub x_max: [f32; 6],
    pub x_min2: [f32; 6],
    pub x_max2: [f32; 6],
    pub y_min: [f32; 6],
    pub y_max: [f32; 6],
    pub bottom_left: i32,
    pub bottom_right: i32,
    pub top_left: i32,
    pub sound_tick_bool: i32,
    pub fall_bool: i32,
    pub death_bool: i32,
    pub music_stop_bool: i32,
}

impl CharacterMovement {
    pub fn new(node: usize) -> CharacterMovement {
        CharacterMovement { node, ..Default::default() }
    }
}

#[derive(Default, Clone)]
pub struct CharacterTexture {
    pub node: usize,
    pub frame: f32,
    pub frame5: f32,
    pub frame15: f32,
    pub time5: f32,
    pub time15: f32,
    pub annim: f32,
    pub as_start: i32,
    pub run_speed: f32,
    pub death: i32,
    pub game_end: i32,
}

impl GameScene {
    pub(super) fn character_start(&mut self) {
        let c = &mut self.cm;
        c.movement = 0.0;
        c.movement_speed = 0.0;
        c.fall_time = 0;
        c.jump = 0;
        c.jump_power = 0.0;
        c.height = 0;
        c.height_max = 4;
        c.mov_timer = 0;
        c.time_elapsed = 0;
        c.super_jump = 1.0;
        c.bumping = 0;
        c.gravity = 1;
        c.lives = 1;
        // (xMin, xMax, xMin2, xMax2, yMin, yMax) for rows 1..6
        let rows = [
            (0.0, 0.0, 0.0, 0.0, 0.0, 20.0),
            (-200.0, 200.0, -200.0, 200.0, 40.0, 60.0),
            (-180.0, -10.0, 65.0, 160.0, 80.0, 100.0),
            (-100.0, 75.0, -100.0, 75.0, 120.0, 140.0),
            (-180.0, -10.0, 55.0, 160.0, 160.0, 180.0),
            (-100.0, 75.0, -100.0, 75.0, 200.0, 220.0),
        ];
        for (k, r) in rows.iter().enumerate() {
            c.x_min[k] = r.0;
            c.x_max[k] = r.1;
            c.x_min2[k] = r.2;
            c.x_max2[k] = r.3;
            c.y_min[k] = r.4;
            c.y_max[k] = r.5;
        }
        c.bottom_left = 0;
        c.bottom_right = 0;
        c.top_left = 0;
        c.death_bool = 0;

        let t = &mut self.ct;
        t.as_start = 0;
        t.time5 = 0.0;
        t.time15 = 0.0;
        t.run_speed = 0.6;
        t.death = 0;
        t.game_end = 0;

        let im = &mut self.im;
        im.hit = 0;
        im.tick = 0;
        im.jump_ok = 0;
        im.side_to_go = 0;
        im.d_time = 0;
        im.data_finger = (0.0, 0.0);
        im.derivative = (0.0, 0.0);
        // widthRatio/heightRatio are 1: the host already maps into 800x480.
        im.jump_ok_value = 20.0;
        im.side_to_go_value = 20.0;
    }

    pub(super) fn character_fixed(&mut self) {
        let node = self.cm.node;
        let c = &mut self.cm;
        c.grav_force = 15.0 * (c.fall_time as f32 * FIXED_DT).powi(2);
        c.time_elapsed += 1;
        if c.jump == 0 {
            if c.gravity == 1 {
                let g = c.grav_force;
                self.ctx.w.translate(node, 0.0, 0.0, g, Space::Self_);
            }
            let c = &mut self.cm;
            c.fall_time += 1;
            c.super_jump = 1.0;
            c.sound_tick_bool = 1;
        }
        if self.cm.jump == 1 {
            let c = &mut self.cm;
            c.jump_power = c.super_jump * (17 / (c.fall_time + 1) + c.fall_time / 5) as f32;
            let p = c.jump_power;
            self.ctx.w.translate(node, 0.0, p, 0.0, Space::World);
            self.cm.fall_time += 1;
            self.cm.time_elapsed = 0;
            if self.cm.sound_tick_bool == 1 {
                self.ctx.play("woosh_new");
                self.cm.sound_tick_bool = 0;
            }
        }
        if self.cm.fall_time > 10 {
            self.cm.jump = 0;
        }
        for k in (0..6).rev() {
            let pos = self.ctx.w.position(node);
            let c = &self.cm;
            let row_ok = c.height == c.height_max - (5 - k as i32);
            let in1 = pos.x > c.x_min[k] && pos.x < c.x_max[k] && pos.y > c.y_min[k] && pos.y < c.y_max[k];
            let in2 = pos.x > c.x_min2[k] && pos.x < c.x_max2[k] && pos.y > c.y_min[k] && pos.y < c.y_max[k];
            if row_ok && (in1 || in2) {
                let y = (50 + 40 * c.height) as f32;
                self.ctx.w.set_position(node, v3(pos.x, y, pos.z));
                let up = self.ctx.input.get_key(Key::Up);
                let c = &mut self.cm;
                c.jump = 0;
                c.fall_time = 0;
                if (up && c.time_elapsed > 5) || c.top_left == 1 {
                    c.jump = 1;
                    c.top_left = 0;
                }
            }
        }
        self.cm.mov_timer += 1;
        if self.ct.death == 0 && (self.capsule_cast(1.0) || self.capsule_cast(-1.0)) {
            self.cm.mov_timer = 0;
        }
        if self.ctx.w.position(node).y < self.cam.min_y_obstacle - 40.0 {
            if self.cm.fall_bool == 0 {
                self.ctx.play("FallSound");
                self.cm.fall_bool = 1;
            }
            self.fall_death();
        }
        self.get_input_information();
        self.evaluate_gui_state();
    }

    /// Physics.CapsuleCast(p - 2x*dir, p + 2x*dir, r=2, dir, 3) against box
    /// colliders. Colliders already overlapping the capsule are ignored, as in PhysX.
    fn capsule_cast(&self, dir: f32) -> bool {
        let p = self.ctx.w.position(self.cm.node);
        let r = 2.0f32;
        for &i in &self.colliders {
            let Some((c, e)) = self.ctx.w.collider_aabb(i) else { continue };
            let dy = ((p.y - c.y).abs() - e.y).max(0.0);
            let dz = ((p.z - c.z).abs() - e.z).max(0.0);
            let seg = |x0: f32, x1: f32| -> f32 {
                let (lo, hi) = (x0.min(x1), x0.max(x1));
                let dx = if hi < c.x - e.x {
                    c.x - e.x - hi
                } else if lo > c.x + e.x {
                    lo - (c.x + e.x)
                } else {
                    0.0
                };
                dx * dx + dy * dy + dz * dz
            };
            let start = seg(p.x - 2.0, p.x + 2.0);
            if start <= r * r {
                continue;
            }
            let swept = seg(p.x - 2.0 * dir, p.x + 2.0 * dir + 3.0 * dir);
            if swept <= r * r {
                return true;
            }
        }
        false
    }

    pub(super) fn character_update(&mut self, dt: f32) {
        let node = self.cm.node;
        let pos = self.ctx.w.position(node);
        self.cm.height = ((pos.y - 35.0) / 40.0) as i32;
        if self.cm.mov_timer > 5 {
            let m = self.cm.movement * dt;
            self.ctx.w.translate(node, m, 0.0, 0.0, Space::Self_);
        }
        // The swipe direction (SideToGo) never resets in the original and wins
        // every FixedUpdate; arrow keys update it too so they keep working after a swipe.
        if self.ctx.input.get_key_down(Key::Left) && self.ct.death == 0 {
            self.cm.movement = self.cm.movement_speed;
            if self.im.side_to_go != 0 {
                self.im.side_to_go = 1;
            }
        }
        if self.ctx.input.get_key_down(Key::Right) && self.ct.death == 0 {
            self.cm.movement = -self.cm.movement_speed;
            if self.im.side_to_go != 0 {
                self.im.side_to_go = -1;
            }
        }
        if self.ctx.w.position(node).x < -175.0 {
            self.ctx.w.translate(node, 337.0, 0.0, 0.0, Space::World);
        }
        if self.ctx.w.position(node).x > 170.0 {
            self.ctx.w.translate(node, -337.0, 0.0, 0.0, Space::World);
        }
    }

    fn get_input_information(&mut self) {
        let c = &mut self.cm;
        c.bottom_left = self.bottom_left.hit;
        c.bottom_right = self.bottom_right.hit;
        if self.top_left.hit == 0 || self.im.hit == 0 {
            c.top_left = 0;
        }
        if self.top_left.hit == 1 || self.im.hit == 1 {
            c.top_left = 1;
        }
    }

    fn evaluate_gui_state(&mut self) {
        let alive = self.ct.death == 0;
        let c = &mut self.cm;
        if c.bottom_left == 1 && alive {
            c.movement = -c.movement_speed;
            c.bottom_left = 0;
            c.bottom_right = 0;
        }
        if c.bottom_right == 1 && alive {
            c.movement = c.movement_speed;
            c.bottom_right = 0;
            c.bottom_left = 0;
        }
        if self.im.side_to_go == -1 {
            c.movement = -c.movement_speed;
        }
        if self.im.side_to_go == 1 {
            c.movement = c.movement_speed;
        }
    }

    fn kill_rows(&mut self) {
        let c = &mut self.cm;
        c.jump = 0;
        c.lives = 0;
        c.y_min = [0.0; 6];
        c.y_max = [0.0; 6];
        self.ct.death = 1;
        self.ctx.invoke(Call::StopFalling, 3.0);
    }

    fn stop_music_once(&mut self) {
        if self.cm.music_stop_bool == 0 {
            self.ctx.music.state = MusicState::Stopped;
            self.cm.music_stop_bool = 1;
        }
    }

    pub(super) fn death(&mut self) {
        self.stop_music_once();
        if self.ctx.prefs.get_int("sound") == 1 && self.cm.death_bool == 0 {
            self.ctx.play("DeathSound");
            self.cm.death_bool = 1;
        }
        self.kill_rows();
    }

    fn fall_death(&mut self) {
        self.stop_music_once();
        self.kill_rows();
    }

    pub(super) fn stop_falling(&mut self) {
        let c = &mut self.cm;
        c.gravity = 0;
        c.movement = 0.0;
        c.movement_speed = 0.0;
    }

    pub(super) fn character_texture_fixed(&mut self) {
        let loading = self.ctx.prefs.get_int("loading");
        let t = &mut self.ct;
        let c = &mut self.cm;
        if t.death == 0 {
            // loading == 1 adds the 300-tick fake loading screen in front.
            let lead = if loading == 1 { 300 } else if loading == 0 { 0 } else { -1 };
            if lead >= 0 {
                if t.as_start < 37 + lead {
                    c.jump = 0;
                    t.annim = 0.375;
                    t.frame = if t.as_start - lead > 0 { ((t.as_start - lead) / 6) as f32 * 0.0625 } else { 0.0 };
                    t.as_start += 1;
                    if t.as_start == 36 + lead {
                        c.movement_speed = 125.0;
                        c.movement = -c.movement_speed;
                    }
                } else {
                    t.frame15 = t.time15.floor() * 0.0625;
                    if t.time15 > 14.0 {
                        t.time15 = t.run_speed;
                    }
                    t.time15 += 1.0;
                    if c.fall_time == 0 {
                        t.time5 = 0.0;
                        t.frame = t.frame15;
                        t.annim = if c.movement > 0.0 { 0.875 } else { 0.75 };
                    }
                    if c.fall_time != 0 || c.jump == 1 {
                        t.time5 += 0.5;
                        t.frame5 = t.time5.floor() * 0.0625;
                        if t.time5 > 4.0 {
                            t.time5 = 4.0;
                        }
                        t.frame = t.frame5;
                        t.annim = if c.movement > 0.0 { 0.5 } else { 0.625 };
                    }
                }
            }
        } else {
            t.game_end += 1;
            t.annim = 0.25;
            t.time5 += 0.2;
            t.frame5 = t.time5.floor() * 0.0625;
            if t.time5 > 6.0 {
                t.time5 = 7.0;
            }
            t.frame = t.frame5;
            if t.game_end > 120 {
                t.annim = 0.0;
            }
        }
        let (f, a) = (t.frame, t.annim);
        self.ctx.w.nodes[t.node].uv_offset = Some([f, a]);
    }

    pub(super) fn input_update(&mut self) {
        let touches = self.ctx.input.touches.clone();
        // InputManager
        let im = &mut self.im;
        im.hit = 0;
        let in_jump_zone = |x: f32, y: f32| x >= 469.0 && x <= 800.0 && y <= 200.0 && y >= 0.0;
        for t in &touches {
            if !in_jump_zone(t.x, t.y) {
                continue;
            }
            if self.ctx.w.gui_hit(im.node, t.x, t.y) {
                if im.tick < 2 {
                    im.hit = 1;
                }
                im.tick += 1;
            }
            if t.phase == Phase::Ended {
                im.tick = 0;
            }
        }
        im.d_time += 1;
        for t in &touches {
            if in_jump_zone(t.x, t.y) {
                continue;
            }
            if t.phase == Phase::Began {
                im.d_time = 0;
                im.data_finger = (t.x, t.y);
            }
            im.present_finger = (t.x, t.y);
            if im.d_time > 1 {
                im.d_time = 0;
                im.derivative = (im.data_finger.0 - im.present_finger.0, im.data_finger.1 - im.present_finger.1);
            }
            if im.d_time == 0 {
                im.data_finger = (t.x, t.y);
            }
            if t.phase == Phase::Ended {
                im.present_finger = (0.0, 0.0);
                im.data_finger = (0.0, 0.0);
                im.derivative = (0.0, 0.0);
                im.jump_ok = 0;
            }
        }
        if im.derivative.0 > im.side_to_go_value.abs() {
            im.side_to_go = 1;
        }
        if im.derivative.0 < -im.side_to_go_value.abs() {
            im.side_to_go = -1;
        }

        // TBLTopLeft
        self.top_left.hit = 0;
        for t in &touches {
            if self.ctx.w.gui_hit(self.top_left.node, t.x, t.y) {
                self.top_left.hit = 1;
            }
            if t.phase == Phase::Ended {
                self.top_left.tick = 0;
            }
        }
        // TBLBottomLeft / TBLBottomRight: the last touch decides.
        for tbl in [&mut self.bottom_left, &mut self.bottom_right] {
            if touches.is_empty() {
                continue;
            }
            for t in &touches {
                tbl.hit = self.ctx.w.gui_hit(tbl.node, t.x, t.y) as i32;
            }
        }
        // GUIConditioned (Retry, Exit, Pause)
        for b in [&mut self.retry, &mut self.exit, &mut self.pause] {
            if touches.is_empty() || b.option != 1 {
                continue;
            }
            for t in &touches {
                b.hit = 0;
                if self.ctx.w.gui_hit(b.node, t.x, t.y) {
                    if t.phase == Phase::Began {
                        b.hit = 1;
                    }
                    if t.phase == Phase::Ended {
                        b.hit = 0;
                    }
                }
            }
        }
    }

    pub(super) fn camera_start(&mut self) {
        self.cam.speed = 0.0;
        self.ctx.invoke_repeating(Call::GivePosition, 2.0, 0.01);
    }

    pub(super) fn camera_fixed(&mut self) {
        let cam = self.cam.cam;
        let target = self.ctx.w.position(self.cam.target);
        let p = self.ctx.w.position(cam);
        // Orthographic camera looking down +Z: the viewport round-trip in the
        // original reduces to "keep x/z, chase the player's y + 30".
        let dest = target.y + 30.0;
        let y = smooth_damp(p.y, dest, &mut self.cam.speed, 0.1, FIXED_DT);
        self.ctx.w.set_position(cam, v3(p.x, y, p.z));
    }

    pub(super) fn give_position(&mut self) {
        let y = self.ctx.w.position(self.cam.cam).y;
        let c = &mut self.cam;
        c.min_y = y - 111.0;
        c.max_y = y + 119.0;
        c.min_y_coin = y - 110.0;
        c.max_y_coin = y + 117.0;
        c.min_y_obstacle = y - 130.0;
        c.max_y_obstacle = y + 120.0;
    }

    pub(super) fn music_update(&mut self) {
        self.ctx.music.mute = self.ctx.prefs.get_int("music") == 0;
    }

    pub fn player_position(&self) -> V3 {
        self.ctx.w.position(self.cm.node)
    }
}
