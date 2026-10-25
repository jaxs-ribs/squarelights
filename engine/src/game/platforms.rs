//! PlatformGenerator, BlockController(Initial), Block and prefabController.
//! The original spelled out every chunk as a switch case; the cases are
//! transcribed verbatim into the SINGLES / DOUBLES tables below.

use super::{Call, GameScene};
use crate::math::{v3, V3};
use crate::world::Space;

struct Chunk {
    x: f32,
    startloc: i32,
    num: i32,
    startloc2: i32,
    num2: i32,
    super_x: Option<f32>,
    heart_x: Option<f32>,
    /// (chosenObstacle 1..10, x)
    obstacles: &'static [(i32, f32)],
    x_range: (f32, f32, f32, f32),
}

const fn single(x: f32, startloc: i32, num: i32, sc: f32, ht: f32, obs: &'static [(i32, f32)], r: (f32, f32)) -> Chunk {
    Chunk { x, startloc, num, startloc2: 0, num2: 0, super_x: Some(sc), heart_x: Some(ht), obstacles: obs, x_range: (r.0, r.1, r.0, r.1) }
}

#[allow(clippy::too_many_arguments)]
const fn double(startloc: i32, num: i32, startloc2: i32, num2: i32, sc: f32, ht: f32, obs: &'static [(i32, f32)], r: (f32, f32, f32, f32)) -> Chunk {
    Chunk { x: 0.0, startloc, num, startloc2, num2, super_x: Some(sc), heart_x: Some(ht), obstacles: obs, x_range: r }
}

const SINGLES: [Chunk; 10] = [
    single(-22.0, -100, 3, 20.0, -60.0, &[(1, -78.0)], (-125.0, 50.0)),
    single(15.0, -45, 3, 25.0, -50.0, &[(6, 46.0)], (-85.0, 100.0)),
    single(3.0, -80, 3, 10.0, -60.0, &[(2, 8.7)], (-100.0, 75.0)),
    single(40.0, -20, 3, 20.0, -60.0, &[(7, -41.7)], (-60.0, 115.0)),
    single(-9.0, -100, 3, -60.0, 20.0, &[(3, 8.5)], (-112.0, 63.0)),
    single(3.0, -80, 3, -60.0, 20.0, &[(8, -54.0)], (-95.0, 50.0)),
    single(40.0, -50, 3, -60.0, 20.0, &[(4, 71.3)], (-60.0, 90.0)),
    single(3.0, -80, 3, -60.0, 20.0, &[(9, -3.5)], (-100.0, 50.0)),
    single(15.0, -50, 2, -60.0, 20.0, &[(5, -66.5)], (-85.0, 65.0)),
    Chunk { x: -10.0, startloc: -100, num: 3, startloc2: 0, num2: 0, super_x: None, heart_x: None, obstacles: &[], x_range: (-100.0, 75.0, -100.0, 75.0) },
];

const DOUBLES: [Chunk; 10] = [
    double(-160, 2, 80, 2, -120.0, 100.0, &[(1, -28.0), (1, 84.5)], (-180.0, -10.0, 50.0, 180.0)),
    double(-165, 3, 80, 2, -120.0, 100.0, &[(6, -28.0), (6, 109.0)], (-180.0, -10.0, 65.0, 180.0)),
    double(-140, 3, 105, 2, -120.0, 100.0, &[(2, -66.6), (2, 83.5)], (-180.0, -10.0, 65.0, 180.0)),
    double(-130, 2, 40, 3, -100.0, 120.0, &[(7, -90.5), (7, 21.0)], (-180.0, -60.0, 0.0, 180.0)),
    double(-100, 1, 45, 3, -90.0, 130.0, &[(3, -116.0), (3, 21.0)], (-180.0, -85.0, 0.0, 180.0)),
    double(-150, 2, 65, 3, 100.0, -120.0, &[(8, -78.0), (8, 46.5)], (-180.0, -60.0, 15.0, 180.0)),
    double(-140, 3, 90, 2, 100.0, -120.0, &[(4, 71.8), (4, -40.5)], (-180.0, -10.0, 65.0, 180.0)),
    double(-140, 3, 120, 2, 100.0, -120.0, &[(9, -53.0), (9, 97.0)], (-180.0, 0.0, 68.0, 180.0)),
    double(-140, 3, 120, 2, 100.0, -120.0, &[(4, -41.0), (4, 97.0)], (-180.0, 15.0, 68.0, 180.0)),
    Chunk { x: 0.0, startloc: -140, num: 3, startloc2: 80, num2: 2, super_x: None, heart_x: None, obstacles: &[], x_range: (-180.0, -10.0, 65.0, 180.0) },
];

impl GameScene {
    pub(super) fn platforms_start(&mut self) {
        let w = &self.ctx.w;
        for k in 0..10 {
            self.pg.singles[k] = w.find(&format!("singlechunk{}", k + 1));
            self.pg.doubles[k] = w.find(&format!("doublechunk{}", k + 1));
        }
        // The original spawns singles at z=100 and doubles at z=128. One prefab
        // (singlechunk8) has its platform ~200 units deeper than the others, which
        // puts its blocks behind the background plane: collidable but invisible.
        // Spawn such chunks so their blocks land on the same plane as the rest.
        let bg_z = self.ctx.w.position(self.ctx.w.find("Background")).z;
        for (kind, base) in [(0usize, 100.0f32), (1, 128.0)] {
            for k in 0..10 {
                let chunk = if kind == 0 { self.pg.singles[k] } else { self.pg.doubles[k] };
                let w = &self.ctx.w;
                let cz = w.position(chunk).z;
                let mut nodes = Vec::new();
                w.descendants(chunk, &mut nodes);
                let dz: Vec<f32> = nodes
                    .iter()
                    .filter(|&&n| w.nodes[n].scripts.iter().any(|s| s == "Block"))
                    .map(|&n| w.position(n).z - cz)
                    .collect();
                let deepest = dz.iter().cloned().fold(f32::MIN, f32::max);
                let mut z = base;
                if base + deepest > bg_z - 20.0 {
                    let mean = dz.iter().sum::<f32>() / dz.len() as f32;
                    z = 100.0 - mean;
                }
                self.pg.spawn_z[kind][k] = z;
            }
        }
        let pg = &mut self.pg;
        pg.chunk_type_for_coin = 0;
        pg.height = 0.0;
        pg.startloc = 0;
        pg.num = 0;
        pg.startloc2 = 0;
        pg.num2 = 0;
        pg.a = 0;
        pg.b = 0;
        pg.e = 0;
        pg.switcher = 1;
        pg.spawn_intel_divider = 1;
        pg.zoner = self.ctx.rng.range(5, 10) + 10000;
        pg.phase = 0;
        pg.chosen_triangle = 0;
        self.ctx.invoke_repeating(Call::PgIsInside, 1.0, 0.3);
        self.ctx.invoke_repeating(Call::PgProcess, 0.5, 0.05);

        for i in 0..self.controllers.len() {
            let node = self.controllers[i].node;
            let initial_pos = self.ctx.w.nodes[node].lpos;
            let r = if self.controllers[i].initial {
                self.ctx.rng.range(50, 150) + 1000
            } else {
                self.ctx.rng.range(0, 50) + 1000 + self.controllers[i].increaser
            };
            let bc = &mut self.controllers[i];
            bc.initial_pos = initial_pos;
            bc.difficulty = 0.0;
            bc.timer = 0;
            bc.evaluated = 0;
            bc.out = 0;
            bc.rand_time = r as f32 - bc.difficulty;
        }
        for b in &mut self.blocks {
            b.bumped = 0;
            b.bump_return = 5;
            b.bump_count = 0;
        }
    }

    pub(super) fn platforms_fixed(&mut self) {
        for i in 0..self.controllers.len() {
            self.block_controller_fixed(i);
        }
        for i in 0..self.blocks.len() {
            self.block_fixed(i);
        }
    }

    fn inside(&self, node: usize, lo: f32, hi: f32) -> i32 {
        let y = self.ctx.w.position(node).y;
        (y > lo && y < hi) as i32
    }

    pub(super) fn pg_is_inside(&mut self) {
        let (lo, hi) = (self.cam.min_y, self.cam.max_y);
        for k in 0..10 {
            self.pg.inside_single[k] = self.inside(self.pg.singles[k], lo, hi);
            self.pg.inside_double[k] = self.inside(self.pg.doubles[k], lo, hi);
        }
    }

    fn evaluate_spawn_number(&mut self) {
        let k = self.pg.platform_number;
        if (1..=10).contains(&k) {
            let k = (k - 1) as usize;
            if self.pg.switcher == 0 {
                self.pg.global_inside = self.pg.inside_single[k];
            }
            if self.pg.switcher == 1 {
                self.pg.global_inside = self.pg.inside_double[k];
            }
        }
    }

    pub(super) fn pg_process(&mut self) {
        let height = self.cm.height;
        self.pg.spawn_intel = ((height - 1) / self.pg.spawn_intel_divider) as f32;
        if self.pg.spawn_intel != 1.0 {
            return;
        }
        self.pg.platform_number = self.ctx.rng.range(1, 10);
        self.evaluate_spawn_number();
        if self.pg.global_inside != 0 {
            return;
        }
        self.pg.spawn_intel_divider += 1;
        self.cm.height_max = self.pg.spawn_intel_divider + 3;
        if self.pg.phase == 0 {
            self.spawn_zone1();
            if self.pg.zoner == self.pg.spawn_intel_divider && self.pg.phase == 0 {
                self.pg.phase = 1;
                self.pg.zoner += self.ctx.rng.range(0, 10) + 10000;
            }
        } else if self.pg.phase == 1 && self.pg.zoner == self.pg.spawn_intel_divider {
            self.pg.phase = 0;
            self.pg.zoner += self.ctx.rng.range(0, 10) + 10000;
        }
    }

    fn row_y(&self, base: f32) -> f32 {
        // the C# used a never-assigned field `i` (0) in "base + 40*i + 40*(div-1)"
        base + (40 * (self.pg.spawn_intel_divider - 1)) as f32
    }

    fn spawn_zone1(&mut self) {
        let single = self.pg.switcher == 0;
        let k = (self.pg.platform_number - 1) as usize;
        let spec = if single { &SINGLES[k] } else { &DOUBLES[k] };
        self.pg.height = self.row_y(195.0);
        let (chunk, z) = if single {
            (self.pg.singles[k], self.pg.spawn_z[0][k])
        } else {
            (self.pg.doubles[k], self.pg.spawn_z[1][k])
        };
        let y = self.pg.height;
        self.ctx.w.set_position(chunk, v3(spec.x, y, z));
        self.jump_for_it(chunk);
        self.pg.startloc = spec.startloc;
        self.pg.num = spec.num;
        if !single {
            self.pg.startloc2 = spec.startloc2;
            self.pg.num2 = spec.num2;
        }
        self.pg.chunk_type_for_coin = if single { 1 } else { 2 };
        self.coin_spawn();
        if let Some(sx) = spec.super_x {
            if self.ctx.rng.range(1, 100) < 3 {
                self.pg.super_coin_inside = self.inside(self.super_coin, self.cam.min_y_obstacle, self.cam.max_y_obstacle);
                if self.pg.super_coin_inside == 0 {
                    let p = v3(sx, self.row_y(216.0), 100.0);
                    self.ctx.w.set_position(self.super_coin, p);
                }
            }
        }
        if let Some(hx) = spec.heart_x {
            if self.ctx.rng.range(1, 100) < 5 {
                self.pg.heart_inside = self.inside(self.st_life, self.cam.min_y_obstacle, self.cam.max_y_obstacle);
                if self.pg.heart_inside == 0 {
                    let p = v3(hx, self.row_y(216.0), 100.0);
                    self.ctx.w.set_position(self.st_life, p);
                }
            }
        }
        for &(chosen, ox) in spec.obstacles {
            if self.ctx.rng.range(1, 100) < 50 {
                self.pg.chosen_obstacle = chosen;
                let node = self.obstacle_node(chosen);
                self.pg.global_obstacle_inside = self.inside(node, self.cam.min_y_obstacle, self.cam.max_y_obstacle);
                if self.pg.global_obstacle_inside == 0 {
                    let p = v3(ox, self.row_y(210.5), 100.0);
                    self.ctx.w.set_position(node, p);
                }
            }
        }
        let r = spec.x_range;
        self.pg.x_min = r.0;
        self.pg.x_max = r.1;
        self.pg.x_min2 = r.2;
        self.pg.x_max2 = r.3;
        self.platform_collisions();
        self.pg.switcher = if single { 1 } else { 0 };
    }

    fn obstacle_node(&self, chosen: i32) -> usize {
        if chosen <= 5 {
            self.barricade_nodes[(chosen - 1) as usize]
        } else {
            self.blockade_nodes[(chosen - 6) as usize]
        }
    }

    /// prefabController.JumpForIt: BroadcastMessage("goForIt") to the chunk.
    fn jump_for_it(&mut self, chunk: usize) {
        let mut nodes = Vec::new();
        self.ctx.w.descendants(chunk, &mut nodes);
        for n in nodes {
            if let Some(i) = self.controllers.iter().position(|c| c.node == n) {
                self.go_for_it(i);
            }
        }
    }

    fn go_for_it(&mut self, i: usize) {
        let r = if self.controllers[i].initial {
            self.ctx.rng.range(0, 50) + 250
        } else {
            self.ctx.rng.range(0, 50) + 1000 + self.controllers[i].increaser
        };
        let node = self.controllers[i].node;
        let bc = &mut self.controllers[i];
        bc.get_started = 1;
        if !bc.initial {
            self.ctx.w.nodes[node].lpos = bc.initial_pos;
            bc.ypos = 0.0;
            bc.out = 0;
        }
        self.ctx.w.set_scale(node, V3::ONE);
        bc.timer = 0;
        bc.evaluated = 0;
        bc.rand_time = r as f32 - bc.difficulty;
    }

    fn platform_collisions(&mut self) {
        let c = &mut self.cm;
        for k in 0..5 {
            c.x_min[k] = c.x_min[k + 1];
            c.x_max[k] = c.x_max[k + 1];
            c.x_min2[k] = c.x_min2[k + 1];
            c.x_max2[k] = c.x_max2[k + 1];
        }
        c.x_min[5] = self.pg.x_min;
        c.x_max[5] = self.pg.x_max;
        c.x_min2[5] = self.pg.x_min2;
        c.x_max2[5] = self.pg.x_max2;
        for k in 0..6 {
            c.y_min[k] += 40.0;
            c.y_max[k] += 40.0;
        }
    }

    fn coin_inside(&self, chosen: i32) -> i32 {
        if (1..=20).contains(&chosen) {
            self.inside(self.triangles[(chosen - 1) as usize], self.cam.min_y_coin, self.cam.max_y_coin)
        } else {
            // EvaluateCoinNumber leaves GlobalTriangleInside unchanged for 0 / 21
            self.pg.global_triangle_inside
        }
    }

    fn place_triangle(&mut self, x: i32) {
        let c = self.pg.chosen_triangle;
        if (1..=20).contains(&c) {
            let p = v3(x as f32, 20.0 + self.pg.height, 0.0);
            self.ctx.w.set_position(self.triangles[(c - 1) as usize], p);
        }
    }

    fn coin_spawn(&mut self) {
        self.pg.chosen_triangle += 1;
        self.pg.global_triangle_inside = self.coin_inside(self.pg.chosen_triangle);
        if self.pg.global_triangle_inside == 0 {
            if self.pg.chunk_type_for_coin == 1 {
                self.pg.a = 0;
                self.pg.b = 0;
                self.pg.e = 0;
                for _ in 0..self.pg.num {
                    if self.pg.a != 0 {
                        self.pg.chosen_triangle += 1;
                    }
                    self.place_triangle(self.pg.startloc + 60 * self.pg.a);
                    self.pg.global_triangle_inside = self.coin_inside(self.pg.chosen_triangle);
                    if self.pg.global_triangle_inside == 1 {
                        break;
                    }
                    self.pg.a += 1;
                    if self.pg.a > 3 {
                        self.pg.a = 0;
                    }
                }
            }
            if self.pg.chunk_type_for_coin == 2 {
                for _ in 0..self.pg.num2 {
                    if self.pg.b != 0 {
                        self.pg.chosen_triangle += 1;
                    }
                    self.place_triangle(self.pg.startloc2 + 60 * self.pg.b);
                    self.pg.global_triangle_inside = self.coin_inside(self.pg.chosen_triangle);
                    if self.pg.global_triangle_inside == 1 {
                        break;
                    }
                    self.pg.b += 1;
                    if self.pg.b > 3 {
                        self.pg.b = 0;
                    }
                }
                for _ in 0..self.pg.num {
                    if self.pg.e != 0 {
                        self.pg.chosen_triangle += 1;
                    }
                    self.place_triangle(self.pg.startloc + 60 * self.pg.e);
                    self.pg.global_triangle_inside = self.coin_inside(self.pg.chosen_triangle);
                    if self.pg.global_triangle_inside == 1 {
                        break;
                    }
                    self.pg.e += 1;
                    if self.pg.e > 3 {
                        self.pg.e = 0;
                    }
                }
            }
        }
        if self.pg.chosen_triangle > 20 {
            self.pg.chosen_triangle = 0;
        }
    }

    fn set_increaser(&mut self, i: usize) {
        let y = self.ctx.w.position(self.controllers[i].node).y;
        let bc = &mut self.controllers[i];
        bc.height = y;
        let h = y;
        let set = |bc: &mut super::BlockController, inc: i32| {
            bc.increaser = inc;
            bc.rand_time = (1000 + inc) as f32 - bc.difficulty;
        };
        if h >= 610.0 && bc.max_height_bool == 0 {
            set(bc, -700);
            bc.max_height_bool = 1;
            return;
        }
        if h >= 610.0 {
            return;
        }
        set(bc, -680);
        for (limit, inc) in [(570.0, -580), (532.0, -480), (492.0, -380), (452.0, -280), (410.0, -180), (371.0, -80), (328.0, 20)] {
            if h >= limit {
                return;
            }
            set(bc, inc);
        }
        if h < 291.0 {
            set(bc, 120);
            if h < 251.0 {
                set(bc, 60);
            }
        }
    }

    fn block_controller_fixed(&mut self, i: usize) {
        if !self.controllers[i].initial {
            self.set_increaser(i);
        }
        let bc = &mut self.controllers[i];
        bc.timer += 1;
        if bc.difficulty > 99.0 {
            bc.difficulty = 100.0;
        }
        bc.difficulty += if bc.initial { 0.001 } else { 0.0025 };
        if bc.get_started == 1 {
            bc.get_started = 0;
        }
        if bc.timer as f32 > bc.rand_time {
            self.vibration(i);
        }
        let node = self.controllers[i].node;
        let limit = self.cam.min_y_obstacle - 20.0;
        if self.ctx.w.position(node).y < limit && self.controllers[i].out == 0 {
            let dx = if self.controllers[i].initial { -200.0 } else { -500.0 };
            self.ctx.w.translate(node, dx, 0.0, 0.0, Space::World);
            self.controllers[i].out = 1;
            self.controllers[i].timer = 0;
        }
    }

    fn vibration(&mut self, i: usize) {
        let node = self.controllers[i].node;
        let initial = self.controllers[i].initial;
        let t = self.controllers[i].timer as f32;
        let r = self.controllers[i].rand_time;
        if t > 140.0 + r && self.controllers[i].evaluated == 0 {
            self.controllers[i].ypos = self.ctx.w.position(node).y;
            self.evaluate_platform_falling(i);
            self.controllers[i].evaluated = 1;
        }
        let w = &mut self.ctx.w;
        if t > r && t < 100.0 + r {
            w.add_scale(node, v3(0.0, -0.002, 0.0));
        }
        if t > 100.0 + r && t < 150.0 + r {
            w.add_scale(node, v3(-0.004, 0.0, 0.0));
        }
        if t > 150.0 + r && t < 166.0 + r {
            w.add_scale(node, v3(-0.05, -0.03, 0.0));
        }
        if t > 160.0 + r && t < 166.0 + r {
            w.translate(node, 0.0, if initial { -10.0 } else { -7.0 }, 0.0, Space::World);
        }
        if t > 166.0 + r && t < 168.0 + r {
            w.translate(node, if initial { -200.0 } else { -500.0 }, 0.0, 0.0, Space::World);
        }
        // shake: alternate -1/+1 every 5 ticks over (r, r+150)
        let mut s = 0.0;
        while s < 150.0 {
            if t > s + r && t < s + 5.0 + r {
                let dir = if (s as i32 / 5) % 2 == 0 { -1.0 } else { 1.0 };
                w.translate(node, dir, 0.0, 0.0, Space::World);
            }
            s += 5.0;
        }
    }

    fn evaluate_platform_falling(&mut self, i: usize) {
        let y = self.controllers[i].ypos;
        let c = &mut self.cm;
        for k in 0..6 {
            if y < c.y_max[k] && y > c.y_min[k] - 20.0 {
                c.x_max[k] = 0.0;
                c.x_min[k] = 0.0;
                c.x_max2[k] = 0.0;
                c.x_min2[k] = 0.0;
                return;
            }
        }
    }

    fn block_fixed(&mut self, i: usize) {
        let node = self.blocks[i].node;
        let pos = self.ctx.w.position(self.cm.node);
        let bp = self.ctx.w.position(node);
        if pos.y < bp.y - 2.0 && pos.y > bp.y - 20.0 && pos.x < bp.x + 7.0 && pos.x > bp.x - 7.0 {
            self.blocks[i].bumped = 1;
            self.cm.jump = 0;
        }
        let b = &mut self.blocks[i];
        if b.bumped == 1 && b.bump_count <= 5 {
            self.cm.bumping = 1;
            self.ctx.w.translate(node, 0.0, 2.0, 0.0, Space::World);
            b.bump_count += 1;
        }
        if b.bump_count > 5 && b.bump_return >= 0 {
            self.ctx.w.translate(node, 0.0, -2.0, 0.0, Space::World);
            b.bump_return -= 1;
        }
        if b.bump_return < 0 {
            self.cm.bumping = 0;
            b.bumped = 0;
            b.bump_return = 5;
            b.bump_count = 0;
        }
    }
}

impl GameScene {
    /// Test hook: run one spawn with the current switcher/platform_number.
    #[doc(hidden)]
    pub fn debug_spawn(&mut self) {
        self.spawn_zone1();
    }
}
