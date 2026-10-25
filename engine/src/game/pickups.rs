//! Coin, SuperCoin, Life, CoinDrop ("+100" popups), Barricade, Blockade, Trampoline.

use super::{Barricade, Blockade, Call, GameScene, PickupKind, Trampoline};
use crate::math::{v3, V3};
use crate::world::Space;

const YELLOW: [f32; 4] = [1.0, 0.85, 0.47, 1.0];

impl GameScene {
    pub(super) fn coindrop_start(&mut self, i: usize) {
        let node = self.coindrops[i].node;
        let d = &mut self.coindrops[i];
        d.timer = 0;
        d.initial_position = v3(-320.0, -30.0, 0.0);
        d.allow_movement = 0;
        self.ctx.w.nodes[node].color = YELLOW;
    }

    pub(super) fn pickups_start(&mut self) {
        for i in 0..self.pickups.len() {
            let speed = self.ctx.rng.range(1, 5) as f32;
            let p = &mut self.pickups[i];
            p.initial_position = v3(-350.0, -30.0, 0.0);
            p.time = 0.0;
            p.speed = speed;
            p.turnspeed = speed / 2.0;
            p.dead = 0;
            p.spin = 0;
        }
        self.ctx.invoke_repeating(Call::CoinAvailable, 1.0, 0.05);
        self.ctx.invoke_repeating(Call::AvoidObjects, 1.0, 0.1);

        for (k, &node) in self.barricade_nodes.clone().iter().enumerate() {
            let w = &self.ctx.w;
            let name = w.nodes[node].name.clone();
            let parts: [usize; 5] = ["Block1", "Block2", "Block3", "Spike1", "Spike2"].map(|s| w.find(&format!("{name}{s}")));
            let b = Barricade {
                node,
                dead: 0,
                gravity: 0.0,
                time: 0,
                out: 0,
                initial_position: w.position(node),
                initial_rotation: w.rotation(node),
                parts,
                part_pos: parts.map(|p| w.position(p)),
                part_rot: parts.map(|p| w.rotation(p)),
            };
            debug_assert_eq!(k, self.barricades.len());
            self.barricades.push(b);
        }
        for &node in self.blockade_nodes.clone().iter() {
            let w = &self.ctx.w;
            let name = w.nodes[node].name.clone();
            let spikes: [usize; 4] = [1, 2, 3, 4].map(|s| w.find(&format!("{name}{s}")));
            self.blockades.push(Blockade {
                node,
                dead: 0,
                gravity: 0,
                anim: 0,
                time: 0,
                out: 0,
                initial_position: w.position(node),
                spikes,
                spike_pos: spikes.map(|p| w.position(p)),
            });
        }
        for &node in self.trampoline_nodes.clone().iter() {
            let w = &self.ctx.w;
            let name = w.nodes[node].name.clone();
            let blocks: [usize; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9].map(|s| w.find(&format!("{name}{s}")));
            self.trampolines.push(Trampoline { node, time: 0, out: 0, old_movement: 0.0, tension: 0.25, blocks });
        }
    }

    pub(super) fn pickups_fixed(&mut self) {
        for i in 0..self.pickups.len() {
            self.pickup_fixed(i);
        }
        for i in 0..self.coindrops.len() {
            self.coindrop_fixed(i);
        }
        for i in 0..self.barricades.len() {
            self.barricade_fixed(i);
        }
        for i in 0..self.blockades.len() {
            self.blockade_fixed(i);
        }
        for i in 0..self.trampolines.len() {
            self.trampoline_fixed(i);
        }
    }

    fn pickup_fixed(&mut self, i: usize) {
        let node = self.pickups[i].node;
        let kind = self.pickups[i].kind;
        if self.pickups[i].dead == 0 {
            let c = self.ctx.w.position(self.cm.node);
            let p = self.ctx.w.position(node);
            let touch = c.y < p.y + 10.0 && c.y > p.y - 10.0 && c.x < p.x + 10.0 && c.x > p.x - 10.0;
            let bump = self.cm.bumping == 1
                && c.y < p.y - 20.0
                && c.y > p.y - 40.0
                && c.x < p.x + 10.0
                && c.x > p.x - 10.0;
            if touch {
                self.collect(i);
            }
            if bump {
                self.collect(i);
            }
            let pk = &mut self.pickups[i];
            pk.time += 0.05;
            let dy = 0.1 * (pk.speed * pk.time).sin();
            let turn = pk.turnspeed;
            self.ctx.w.translate(node, 0.0, dy, 0.0, Space::World);
            if kind != PickupKind::Life {
                self.ctx.w.rotate(node, 0.0, 0.0, turn, Space::Self_);
            }
            let pk = &mut self.pickups[i];
            if (pk.time as f64) > (pk.speed * 2.0) as f64 * 3.1415 {
                pk.time = 0.0;
            }
        }
        if self.pickups[i].dead == 1 && self.pickups[i].spin == 1 {
            self.ctx.w.translate(node, 0.0, 2.0, 0.0, Space::World);
            self.ctx.w.rotate(node, 0.0, 0.0, 20.0, Space::World);
            let s = self.ctx.w.scale(node);
            self.ctx.w.set_scale(node, s * (1.0 / 1.05));
        }
    }

    /// The body of each pickup's collision branch (GetCoin + score + sound).
    fn collect(&mut self, i: usize) {
        let node = self.pickups[i].node;
        match self.pickups[i].kind {
            PickupKind::Coin => {
                self.score.points += 100;
                self.coins += 1;
                self.spawn_text_mesh(node, "+100");
                self.ctx.play("Coin");
            }
            PickupKind::SuperCoin => {
                self.score.points += 1000;
                self.coins += 1;
                self.spawn_text_mesh(node, "+1000");
                self.ctx.play("Coin");
            }
            PickupKind::Life => {
                if self.cm.lives < 4 {
                    self.cm.lives += 1;
                } else {
                    self.score.points += 1000;
                    self.spawn_text_mesh(node, "+1000");
                }
                self.ctx.play("Bubble");
            }
        }
        // DeathAnim
        self.pickups[i].dead = 1;
        self.pickups[i].spin = 1;
        self.ctx.invoke(Call::PickupReturn(i as u16), 0.5);
    }

    pub(super) fn pickup_return(&mut self, i: usize) {
        let node = self.pickups[i].node;
        let p = &mut self.pickups[i];
        p.dead = 0;
        p.spin = 0;
        let s = if p.kind == PickupKind::SuperCoin { 900.0 } else { 650.0 };
        let pos = p.initial_position;
        self.ctx.w.set_position(node, pos);
        self.ctx.w.set_scale(node, v3(s, s, 1.0));
    }

    pub(super) fn avoid_objects(&mut self) {
        let barricades: Vec<V3> = self.barricade_nodes.iter().map(|&b| self.ctx.w.position(b)).collect();
        let life = self.ctx.w.position(self.st_life);
        let sup = self.ctx.w.position(self.super_coin);
        for i in 0..self.pickups.len() {
            let p = self.ctx.w.position(self.pickups[i].node);
            let near = |o: V3| o.y > p.y - 20.0 && o.y < p.y + 20.0 && o.x > p.x - 20.0 && o.x < p.x + 20.0;
            let mut hit = barricades.iter().any(|&b| near(b));
            if self.pickups[i].kind == PickupKind::Coin {
                hit |= near(life) || near(sup);
            }
            if hit {
                self.pickup_return(i);
            }
        }
    }

    pub(super) fn coin_is_available(&mut self) {
        for k in 0..5 {
            let d = &self.coindrops[k];
            self.coin_av[k] = self.ctx.w.position(d.node).approx_eq(d.initial_position) as i32;
        }
    }

    fn spawn_text_mesh(&mut self, from: usize, text: &str) {
        let Some(k) = (0..5).find(|&k| self.coin_av[k] == 1) else { return };
        let node = self.coindrops[k].node;
        let p = self.ctx.w.position(from) + v3(0.0, 25.0, 0.0);
        self.ctx.w.set_position(node, p);
        self.ctx.w.set_text(node, text);
        self.coindrops[k].immortal = 1;
    }

    fn coindrop_fixed(&mut self, i: usize) {
        let node = self.coindrops[i].node;
        if self.coindrops[i].immortal == 1 {
            self.coindrops[i].allow_movement = 1;
            self.coindrops[i].timer += 1;
            if self.coindrops[i].timer > 60 {
                let d = &mut self.coindrops[i];
                d.immortal = 0;
                d.timer = 0;
                d.allow_movement = 0;
                let p = d.initial_position;
                self.ctx.w.set_position(node, p);
                self.ctx.w.nodes[node].color = YELLOW;
            }
        }
        if self.coindrops[i].allow_movement == 1 {
            self.ctx.w.translate(node, 0.0, 0.1, 0.0, Space::World);
            self.ctx.w.nodes[node].color[3] -= 0.02;
        }
    }

    fn in_out_band(&self, y: f32, lo: f32, hi: f32) -> bool {
        y < self.cam.min_y_obstacle - lo && y > self.cam.min_y_obstacle - hi
    }

    fn barricade_fixed(&mut self, i: usize) {
        let node = self.barricades[i].node;
        if self.barricades[i].dead == 0 {
            let c = self.ctx.w.position(self.cm.node);
            let p = self.ctx.w.position(node);
            if c.y > p.y - 13.0 && c.y < p.y + 25.0 && c.x > p.x - 7.0 && c.x < p.x + 7.0 {
                self.cm.lives -= 1;
                if self.cm.lives == 0 {
                    self.death();
                }
                self.barricade_anim_death(i);
                self.ctx.play("Brick1");
                self.barricades[i].dead = 1;
            }
            if self.cm.bumping == 1 && c.y > p.y - 40.0 && c.y < p.y - 20.0 && c.x > p.x - 7.0 && c.x < p.x + 7.0 {
                self.barricade_anim_death(i);
                self.ctx.play("Brick1");
                self.barricades[i].dead = 1;
            }
        } else {
            let b = &mut self.barricades[i];
            b.time += 1;
            if b.gravity >= 0.0 {
                b.gravity = ((15 - b.time) / 2) as f32;
            } else {
                b.gravity = -b.time as f32;
            }
            let g = b.gravity;
            let parts = b.parts;
            let moves = [(1.0, 5.0), (-1.0, -3.0), (2.0, -4.0), (2.0, -4.0), (-3.0, 3.0)];
            for (k, &(dx, rot)) in moves.iter().enumerate() {
                self.ctx.w.translate(parts[k], dx, g, 0.0, Space::World);
                self.ctx.w.rotate(parts[k], 0.0, 0.0, rot, Space::World);
            }
        }
        let y = self.ctx.w.position(node).y;
        if self.in_out_band(y, 40.0, 80.0) && self.barricades[i].out == 0 {
            self.ctx.w.translate(node, -500.0, 0.0, 0.0, Space::World);
            self.barricades[i].out = 1;
        } else {
            self.barricades[i].out = 0;
        }
    }

    fn barricade_anim_death(&mut self, i: usize) {
        self.score.points += 300;
        let node = self.barricades[i].node;
        self.spawn_text_mesh(node, "+300");
        self.barricades[i].gravity = 1.0;
        self.ctx.invoke(Call::BarricadeDeath(i as u8), 3.0);
        self.ctx.invoke(Call::BarricadeFall(i as u8), 0.3);
    }

    pub(super) fn barricade_fall(&mut self, i: usize) {
        self.barricades[i].gravity = -1.0;
        self.barricades[i].time = 0;
    }

    pub(super) fn barricade_death(&mut self, i: usize) {
        let b = self.barricades[i].clone();
        let w = &mut self.ctx.w;
        w.set_position(b.node, b.initial_position);
        w.set_rotation(b.node, b.initial_rotation);
        for k in 0..5 {
            w.set_position(b.parts[k], b.part_pos[k]);
            w.set_rotation(b.parts[k], b.part_rot[k]);
        }
        let b = &mut self.barricades[i];
        b.dead = 0;
        b.time = 0;
        b.gravity = 0.0;
    }

    fn blockade_fixed(&mut self, i: usize) {
        let node = self.blockades[i].node;
        if self.blockades[i].dead == 0 {
            let c = self.ctx.w.position(self.cm.node);
            let p = self.ctx.w.position(node);
            let band = c.y > p.y - 3.0 && c.y < p.y + 5.0;
            if band && c.x > p.x + 10.0 && c.x < p.x + 13.0 {
                self.blockade_hurt(i);
            }
            if band && c.x > p.x - 13.0 && c.x < p.x - 10.0 {
                self.blockade_hurt(i);
            }
            if band && c.x > p.x - 10.0 && c.x < p.x + 10.0 {
                self.blockades[i].dead = 1;
                self.blockades[i].gravity = -1;
                self.blockade_death_animation1(i);
                self.ctx.play("Boings_new");
            }
            if c.y > p.y + 5.0 && c.y < p.y + 8.0 && c.x > p.x - 13.0 && c.x < p.x + 13.0 {
                self.blockades[i].dead = 1;
                self.blockades[i].gravity = -1;
                self.blockade_death_animation1(i);
                self.ctx.play("Boings_new");
            }
            if self.cm.bumping == 1 && c.y > p.y - 40.0 && c.y < p.y - 20.0 && c.x > p.x - 7.0 && c.x < p.x + 7.0 {
                self.blockades[i].dead = 1;
                self.blockade_death_animation2(i);
                self.ctx.play("Boings_new");
            }
        } else {
            let b = &mut self.blockades[i];
            b.time += 1;
            if b.anim != 1 {
                if b.gravity >= 0 {
                    b.gravity = 10 / b.time;
                } else {
                    b.gravity = -b.time / 3;
                }
                let g = b.gravity as f32;
                self.ctx.w.translate(node, 0.0, g, 0.0, Space::World);
            }
        }
        if self.blockades[i].anim == 1 {
            let t = self.blockades[i].time as f32;
            let s = self.blockades[i].spikes;
            let w = &mut self.ctx.w;
            w.translate(node, 0.0, -t, 0.0, Space::World);
            w.translate(s[0], 4.0, 0.5, 0.0, Space::World);
            w.translate(s[1], 3.0, 1.0, 0.0, Space::World);
            w.translate(s[2], -4.0, 0.5, 0.0, Space::World);
            w.translate(s[3], -3.0, 1.0, 0.0, Space::World);
        }
        let y = self.ctx.w.position(node).y;
        if self.in_out_band(y, 40.0, 80.0) && self.blockades[i].out == 0 {
            let dy = self.cam.min_y_obstacle + 40.0;
            self.ctx.w.translate(node, -500.0, dy, 0.0, Space::World);
            self.blockades[i].dead = 0;
            self.blockades[i].out = 1;
        } else {
            self.blockades[i].out = 0;
        }
    }

    fn blockade_hurt(&mut self, i: usize) {
        self.cm.lives -= 1;
        if self.cm.lives == 0 {
            self.death();
        }
        self.blockade_death_animation2(i);
        self.ctx.play("Boings_new");
        self.blockades[i].dead = 1;
    }

    fn blockade_death_animation1(&mut self, i: usize) {
        self.score.points += 200;
        let node = self.blockades[i].node;
        self.spawn_text_mesh(node, "+200");
        self.blockades[i].anim = 1;
        self.ctx.invoke(Call::BlockadeDeath(i as u8), 1.0);
    }

    fn blockade_death_animation2(&mut self, i: usize) {
        self.score.points += 200;
        let node = self.blockades[i].node;
        self.spawn_text_mesh(node, "+200");
        self.ctx.invoke(Call::BlockadeDeath(i as u8), 1.0);
        self.ctx.invoke(Call::BlockadeFall(i as u8), 0.3);
        self.blockades[i].anim = 0;
    }

    pub(super) fn blockade_death(&mut self, i: usize) {
        let b = self.blockades[i].clone();
        let w = &mut self.ctx.w;
        w.set_position(b.node, b.initial_position);
        for k in 0..4 {
            w.set_position(b.spikes[k], b.spike_pos[k]);
        }
        let b = &mut self.blockades[i];
        b.gravity = 0;
        b.time = 0;
        b.dead = 0;
        b.anim = 0;
    }

    fn trampoline_fixed(&mut self, i: usize) {
        let node = self.trampolines[i].node;
        let c = self.ctx.w.position(self.cm.node);
        let p = self.ctx.w.position(node);
        if c.y > p.y + 10.0 && c.y < p.y + 20.0 && c.x > p.x - 40.0 && c.x < p.x + 40.0 {
            if self.trampolines[i].time == 0 {
                self.trampolines[i].old_movement = self.cm.movement;
            }
            self.cm.movement = 0.0;
            self.cm.fall_time = 0;
            self.trampolines[i].time += 1;
            self.trampoline_anim(i, 1);
        }
        if self.trampolines[i].time > 0 {
            self.cm.movement = self.trampolines[i].old_movement;
            self.cm.super_jump = 1.5;
            self.cm.jump = 1;
            self.trampolines[i].time = 0;
        }
        if self.in_out_band(p.y, 80.0, 120.0) && self.trampolines[i].out == 0 {
            self.ctx.w.translate(node, -500.0, 0.0, 0.0, Space::World);
            self.trampolines[i].out = 1;
        } else {
            self.trampolines[i].out = 0;
        }
    }

    /// Animation1..4: alternate tension, nudging the 9 trampoline blocks.
    pub(super) fn trampoline_anim(&mut self, i: usize, stage: u8) {
        let tension = match stage {
            1 => -0.25,
            2 => 0.25,
            3 => 0.125,
            _ => -0.125,
        };
        self.trampolines[i].tension = tension;
        let div = [4.0, 3.0, 2.0, 1.2, 1.0, 1.2, 2.0, 3.0, 4.0];
        for k in 0..9 {
            let b = self.trampolines[i].blocks[k];
            self.ctx.w.translate(b, 0.0, 24.0 * tension / div[k], 0.0, Space::World);
        }
        if stage < 4 {
            self.ctx.invoke(Call::TrampAnim(i as u8, stage + 1), 0.1);
        }
    }
}
