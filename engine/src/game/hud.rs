//! ScoreManager (score, pause, game over screen, highscores), LifeCount1..3,
//! TriangleEsthetic, CoinCounter and Rotation.

use super::{Call, GameScene};
use crate::input::Key;
use crate::math::{v3, Quat, V3};
use crate::runtime::MusicState;
use crate::world::Space;

const YELLOW: [f32; 4] = [1.0, 0.85, 0.47, 1.0];

impl GameScene {
    pub(super) fn hud_start(&mut self) {
        let w = &self.ctx.w;
        let f = |n: &str| w.find(n);
        let n = &mut self.score.n;
        n.button = f("Button");
        n.bottom_left = f("BottomLeft");
        n.bottom_right = f("BottomRight");
        n.top_left = f("TopLeft");
        n.loading_screen = f("LoadingScreen");
        n.pause_symbol = f("PAUSE");
        n.new_highscore = f("NewHighscore");
        n.highscores = f("Highscores");
        n.game_paused = f("GamePaused");
        n.your_score = f("YourScore");
        n.coins_collected = f("CoinsCollected");
        n.score = f("Score");
        n.coin_counter = f("CoinCounter");
        n.triangle_esthetic = f("TriangleEsthetic");
        n.life = [f("1Life"), f("2Life"), f("3Life")];
        n.button_retry = f("Button_Retry");
        n.button_exit = f("Button_Exit");
        n.background = f("Background");
        n.ranks = [f("rank1"), f("rank2"), f("rank3"), f("rank4"), f("rank5")];

        let s = &mut self.score;
        s.init_bottom_left = w.position(s.n.bottom_left);
        s.init_bottom_right = w.position(s.n.bottom_right);
        s.init_top_left = w.position(s.n.top_left);
        s.pause_pos_shift = v3(-1000.0, -1000.0, 0.0);
        s.offset1 = v3(-10.0, 80.0, -300.0);
        s.offset2 = v3(-9001.0, -9001.0, -9001.0);
        s.difficulty = 0.0035;
        s.time = 0;
        s.init_pause_gui_pos = w.position(self.pause.node);
        s.action = 0;
        s.teleport_timer = 0;
        let yellow_nodes = [s.node, s.n.your_score, s.n.coins_collected, s.n.highscores, s.n.game_paused]
            .into_iter()
            .chain(s.n.ranks);
        for node in yellow_nodes.collect::<Vec<_>>() {
            self.ctx.w.nodes[node].color = YELLOW;
        }
        self.ctx.w.nodes[self.score.n.new_highscore].color = [1.0, 0.75, 0.3, 1.0];
        self.pause.option = 1;
        // CoinCounter
        self.ctx.w.nodes[self.score.n.coin_counter].color = YELLOW;
        self.ctx.invoke_repeating(Call::CoinCounterSlow, 0.0, 0.1);
        // LifeCount1..3
        for lc in &mut self.life_counts {
            lc.1 = 1;
        }
    }

    pub(super) fn score_update(&mut self) {
        self.score.pause_timer += 1;
        let text = self.score.points.to_string();
        self.ctx.w.set_text(self.score.node, &text);
        let key = |k| self.ctx.input.get_key_down(k);
        let (retry, exit, pause) = (key(Key::R), key(Key::E), key(Key::P));
        if self.score.action == 0 && (self.retry.hit == 1 || retry) {
            self.ctx.play("Bubble");
            self.score.action = 1;
            self.button_anim(self.score.n.button_retry, 0.06);
            self.ctx.invoke(Call::RetryReturnAnim, 0.3);
            self.ctx.invoke(Call::RestartGame, 0.4);
        }
        if self.score.action == 0 && (self.exit.hit == 1 || exit) {
            self.ctx.play("Bubble");
            self.score.action = 1;
            self.button_anim(self.score.n.button_exit, 0.2);
            self.ctx.invoke(Call::ExitReturnAnim, 0.3);
            self.ctx.invoke(Call::BackToMenu, 0.4);
        }
        if self.score.action == 0 && (self.pause.hit == 1 || pause) {
            if self.score.pause_bool == 0 {
                self.ctx.music.state = pause_music(self.ctx.music.state);
                self.score.pause_bool = 1;
            }
            self.score.pause_timer = 0;
            self.ctx.play("Bubble");
            self.score.action = 1;
            if self.score.pause_timer < 5 {
                let p = self.ctx.w.position(self.score.n.background) + v3(-40.0, 27.0, 0.0);
                self.ctx.w.set_position(self.score.n.game_paused, p);
            }
            self.ctx.time_scale = 0.0;
        }
        if self.score.action == 1 && (self.pause.hit == 1 || pause) && self.score.pause_timer > 5 {
            if self.score.pause_bool == 1 {
                if self.ctx.music.state == MusicState::Paused {
                    self.ctx.music.state = MusicState::Playing;
                }
                self.score.pause_bool = 0;
            }
            self.ctx.play("Bubble");
            let p = self.score.pause_pos_shift;
            self.ctx.w.set_position(self.score.n.game_paused, p);
            self.score.action = 0;
            self.ctx.time_scale = 1.0;
        }
    }

    pub(super) fn button_anim(&mut self, node: usize, d: f32) {
        for _ in 0..5 {
            self.ctx.w.add_scale(node, v3(d, d, 0.0));
        }
    }

    fn set_hud_hidden(&mut self, hidden: bool) {
        let n = &self.score.n;
        let w = &mut self.ctx.w;
        for node in [n.pause_symbol, n.score, n.coin_counter] {
            w.nodes[node].visible = !hidden;
        }
        let s = &self.score;
        let (pause, bl, br, tl) = if hidden {
            (s.offset2, s.offset2, s.offset2, s.offset2)
        } else {
            (s.init_pause_gui_pos, s.init_bottom_left, s.init_bottom_right, s.init_top_left)
        };
        w.set_position(self.pause.node, pause);
        w.set_position(n.bottom_left, bl);
        w.set_position(n.bottom_right, br);
        w.set_position(n.top_left, tl);
        self.score.renderer_hidden = hidden;
    }

    pub(super) fn score_fixed(&mut self) {
        let loading = self.ctx.prefs.get_int("loading");
        let alive = self.ct.death == 0;
        if loading == 1 {
            if self.score.time < 337 {
                self.score.time += 1;
            }
            let ls = self.score.n.loading_screen;
            if self.score.time > 300 {
                let o = self.score.offset2;
                self.ctx.w.set_position(ls, o);
                if self.score.renderer_hidden {
                    self.set_hud_hidden(false);
                }
            } else {
                let o = self.score.offset1;
                self.ctx.w.set_position(ls, o);
                if !self.score.renderer_hidden {
                    self.set_hud_hidden(true);
                }
            }
            if self.score.time == 337 && alive {
                self.difficulty_step();
            }
        }
        if loading == 0 {
            let o = self.score.offset2;
            self.ctx.w.set_position(self.score.n.loading_screen, o);
            if self.score.time < 37 {
                self.score.time += 1;
            }
            if self.score.time == 37 && alive {
                self.difficulty_step();
            }
        }
        if self.cm.lives == 0 && self.cm.gravity == 0 {
            self.score.teleport_timer += 1;
            let tt = self.score.teleport_timer;
            if tt > 15 && tt < 20 {
                let bg = self.ctx.w.position(self.score.n.background);
                self.ctx.w.set_position(self.score.n.button_retry, bg + v3(5.77, 31.8, -100.0));
                self.ctx.w.set_position(self.score.n.button_exit, bg + v3(8.77, -17.4, -100.0));
            }
            if tt < 60 {
                self.save_score();
                self.end_game();
            }
        }
    }

    fn difficulty_step(&mut self) {
        self.score.points += 2;
        if self.cm.movement_speed < 275.0 {
            self.cm.movement_speed += self.score.difficulty;
            self.ct.run_speed += self.score.difficulty / 186.0;
        }
    }

    fn end_game(&mut self) {
        let off2 = self.score.offset2;
        let n = &self.score.n;
        let bg = self.ctx.w.position(n.background);
        let (bl, br, tl, button) = (n.bottom_left, n.bottom_right, n.top_left, n.button);
        let w = &mut self.ctx.w;
        for node in [bl, br, tl, button] {
            w.set_position(node, off2);
        }
        self.retry.option = 1;
        self.exit.option = 1;
        self.ctx.prefs.set_int("loading", 0);
        if self.score.new_high {
            let p = bg + v3(-42.0, 75.0, 0.0);
            self.ctx.w.set_position(self.score.n.new_highscore, p);
            self.score.new_high = false;
        }
        let shift = self.score.pause_pos_shift;
        self.ctx.w.set_position(self.score.n.pause_symbol, shift);
        self.ctx.w.set_position(self.pause.node, shift);
        for k in 0..3 {
            self.ctx.w.set_position(self.score.n.life[k], v3(1000.0, 0.0, 1.0));
        }
        for k in 0..5 {
            let v = self.ctx.prefs.get_int(&format!("rank{}", k + 1));
            let t = format!("{}.   {}", k + 1, v);
            self.ctx.w.set_text(self.score.n.ranks[k], &t);
        }
        let tt = self.score.teleport_timer;
        let n = &self.score.n;
        let mut moves: Vec<(usize, V3)> = Vec::new();
        if tt > 5 && tt < 10 {
            moves.push((n.your_score, v3(-132.0, 75.0, 0.0)));
        }
        if tt > 10 && tt < 15 {
            moves.push((n.score, v3(-117.5, 40.0, -100.0)));
        }
        if tt > 15 && tt < 20 {
            moves.push((n.coins_collected, v3(-140.0, 15.0, 0.0)));
        }
        if tt > 20 && tt < 25 {
            moves.push((n.coin_counter, v3(-105.0, -20.0, -100.0)));
            moves.push((n.triangle_esthetic, v3(-135.0, -30.0, -100.0)));
        }
        if tt > 25 && tt < 30 {
            moves.push((n.highscores, v3(70.0, 75.0, 0.0)));
        }
        let rank_y = [35.0, 19.0, 3.0, -13.0, -29.0];
        for k in 0..5 {
            let lo = 30 + 5 * k as i32;
            if tt > lo && tt < lo + 5 {
                moves.push((n.ranks[k], v3(85.0, rank_y[k], 0.0)));
            }
        }
        let te = n.triangle_esthetic;
        for (node, off) in moves {
            self.ctx.w.set_position(node, bg + off);
            if node == te {
                self.ctx.w.set_rotation(te, Quat::euler(0.0, 180.0, 0.0));
            }
        }
    }

    fn save_score(&mut self) {
        let p = &mut self.ctx.prefs;
        for k in 1..=5 {
            let key = format!("rank{k}");
            if !p.has_key(&key) {
                p.set_int(&key, 0);
            }
        }
        let pts = self.score.points;
        let r = |p: &crate::runtime::Prefs, k: i32| p.get_int(&format!("rank{k}"));
        if pts > r(p, 5) && pts < r(p, 4) {
            p.set_int("rank5", pts);
        }
        if pts > r(p, 4) && pts < r(p, 3) {
            let v = r(p, 4);
            p.set_int("rank5", v);
            p.set_int("rank4", pts);
        }
        if pts > r(p, 3) && pts < r(p, 2) {
            let (v4, v3_) = (r(p, 4), r(p, 3));
            p.set_int("rank5", v4);
            p.set_int("rank4", v3_);
            p.set_int("rank3", pts);
        }
        if pts > r(p, 2) && pts < r(p, 1) {
            let (v4, v3_, v2) = (r(p, 4), r(p, 3), r(p, 2));
            p.set_int("rank5", v4);
            p.set_int("rank4", v3_);
            p.set_int("rank3", v2);
            p.set_int("rank2", pts);
        }
        if pts > r(p, 1) {
            let (v4, v3_, v2, v1) = (r(p, 4), r(p, 3), r(p, 2), r(p, 1));
            p.set_int("rank5", v4);
            p.set_int("rank4", v3_);
            p.set_int("rank3", v2);
            p.set_int("rank2", v1);
            p.set_int("rank1", pts);
            self.score.new_high = true;
        }
    }

    pub(super) fn hud_fixed(&mut self) {
        // LifeCount1..3: grow while lives >= 2 / >= 3 / == 4
        for k in 0..3 {
            let lives = self.cm.lives;
            let on = match k {
                0 => lives >= 2,
                1 => lives >= 3,
                _ => lives == 4,
            };
            let lc = &mut self.life_counts[k];
            lc.1 = if on { (lc.1 + 1).min(30) } else { (lc.1 - 1).max(10) };
            let t = lc.1 as f32 / 30.0;
            self.ctx.w.set_scale(lc.0, v3(0.0382353 * t, 0.05909091 * t, 1.0));
        }
        // TriangleEsthetic
        if self.ct.death == 0 {
            let turn = 1 + self.coins / 25;
            self.ctx.w.rotate(self.score.n.triangle_esthetic, 0.0, 0.0, turn as f32, Space::Self_);
        }
        // Rotation
        for i in 0..self.rotators.len() {
            self.ctx.w.rotate(self.rotators[i], 0.0, 0.0, 1.0, Space::World);
        }
    }
}

fn pause_music(s: MusicState) -> MusicState {
    if s == MusicState::Playing {
        MusicState::Paused
    } else {
        s
    }
}
