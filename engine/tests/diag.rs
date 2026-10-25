mod common;
use squarelights::{build_scene, runtime::*, Scene};

fn game(seed: u64) -> squarelights::game::GameScene {
    let d = common::assets_dir();
    let r = |f: &str| std::fs::read_to_string(format!("{d}/{f}")).unwrap();
    let mut p = Prefs::default();
    p.set_int("loading", 0); p.set_int("sound", 0);
    match build_scene(&r("scene_menu.json"), &r("scene_game.json"), &r("assets.json"), Rng::new(seed), SceneId::Game, p) {
        Scene::Game(g) => g, _ => unreachable!(),
    }
}

/// For every landing row inside the camera, are there visible blocks under it?
#[test]
#[ignore]
fn rows_vs_blocks() {
    let mut g = game(7);
    g.ctx.input.held[2] = true;
    let mut bad = 0;
    for t in 0..3000 {
        g.frame();
        let w = &g.ctx.w;
        let cam = w.position(g.cam.cam).y;
        let blocks: Vec<_> = g.blocks.iter().map(|b| w.position(b.node)).filter(|p| (p.y - cam).abs() < 110.0 && p.x.abs() < 200.0).collect();
        let c = &g.cm;
        for k in 0..6 {
            let land = c.y_min[k] + 10.0;
            if (land - cam).abs() > 95.0 || c.y_max[k] - c.y_min[k] < 1.0 { continue; }
            for (lo, hi) in [(c.x_min[k], c.x_max[k]), (c.x_min2[k], c.x_max2[k])] {
                if hi - lo < 1.0 { continue; }
                let mid = (lo + hi) / 2.0;
                let near = blocks.iter().filter(|b| (b.y - (land - 8.0)).abs() < 6.0 && b.x > lo - 10.0 && b.x < hi + 10.0).count();
                if near == 0 && bad < 25 {
                    bad += 1;
                    println!("t={t} row{k} land={land} x[{lo},{hi}] mid {mid}: no blocks. cam={cam} blocks at that y: {:?}",
                        blocks.iter().filter(|b| (b.y-(land-8.0)).abs()<15.0).map(|b|(b.x as i32,b.y as i32)).collect::<Vec<_>>());
                }
            }
        }
    }
    println!("bad={bad}");
}

#[test]
#[ignore]
fn blocks_vs_records() {
    use squarelights::render::*;
    let mut g = game(7);
    g.ctx.input.held[2] = true;
    let mut interp = Interp::new();
    let mut out = Vec::new();
    let mut shown = 0;
    for t in 0..3000 {
        g.frame();
        interp.capture(&g.ctx.w);
        let w = &g.ctx.w;
        let cam = w.position(g.cam.cam);
        let view = View { center: cam, half_w: 100.0 * 800.0 / 480.0, half_h: 100.0, forward: squarelights::math::v3(0.0, 0.0, 1.0) };
        build(w, &view, &interp, 1.0, &mut out);
        let drawn: std::collections::HashSet<(i32, i32)> = out.chunks(28).filter(|r| r[0] == 0.0).map(|r| (r[16] as i32, r[17] as i32)).collect();
        for b in &g.blocks {
            let n = &w.nodes[b.node];
            let p = w.position(b.node);
            if (p.y - cam.y).abs() < 90.0 && p.x.abs() < 160.0 && !drawn.contains(&(p.x as i32, p.y as i32)) && shown < 20 {
                shown += 1;
                let par = n.parent.map(|q| (w.nodes[q].name.clone(), w.nodes[q].visible, w.scale(q)));
                println!("t={t} {} at ({:.1},{:.1}) visible={} mesh={:?} mat={:?} parent={:?} scale={:?}", n.name, p.x, p.y, n.visible, n.mesh, n.material, par, w.scale(b.node));
            }
        }
    }
    println!("missing={shown}");
}

#[test]
#[ignore]
fn row_coverage() {
    for seed in [1u64, 7, 42] {
    let mut g = game(seed);
    g.ctx.input.held[2] = true;
    let mut gaps = 0; let mut total = 0;
    for t in 0..4000 {
        g.frame();
        if t % 10 != 0 { continue; }
        let w = &g.ctx.w;
        let cam = w.position(g.cam.cam).y;
        let blocks: Vec<_> = g.blocks.iter().map(|b| w.position(b.node)).collect();
        let c = &g.cm;
        for k in 0..6 {
            let land = c.y_min[k] + 10.0;
            if (land - cam).abs() > 95.0 { continue; }
            for (lo, hi) in [(c.x_min[k], c.x_max[k]), (c.x_min2[k], c.x_max2[k])] {
                if hi - lo < 1.0 { continue; }
                let mut x = lo + 3.0;
                while x < hi - 3.0 {
                    let wx = if x > 170.0 { x - 337.0 } else if x < -175.0 { x + 337.0 } else { x };
                    total += 1;
                    let ok = blocks.iter().any(|b| (b.y - (land - 8.0)).abs() < 8.0 && (b.x - wx).abs() < 9.0);
                    if !ok { gaps += 1; if gaps < 8 { println!("seed {seed} t={t} row{k} land={land} x={wx:.0} range[{lo},{hi}] no block"); } }
                    x += 6.0;
                }
            }
        }
    }
    println!("seed {seed}: gaps={gaps}/{total}");
    }
}

#[test]
#[ignore]
fn dump_row_blocks() {
    let mut g = game(1);
    for t in 0..=40 {
        g.frame();
        if t == 0 || t == 40 {
            let w = &g.ctx.w;
            let mut v: Vec<_> = g.blocks.iter().map(|b| (w.position(b.node), b.node)).filter(|(p, _)| (p.y - 82.0).abs() < 12.0 && p.x.abs() < 300.0).map(|(p, n)| (p.x as i32, p.y as i32, w.nodes[w.nodes[n].parent.unwrap()].name.clone())).collect();
            v.sort();
            println!("t={t} {:?}", v);
        }
    }
}

/// A bot that jumps only when standing under a gap of the row above, logging
/// how often the row above has no platform at all (nothing to climb to).
#[test]
#[ignore]
fn smart_bot() {
    for seed in [1u64, 2, 3, 4, 5] {
        let mut g = game(seed);
        let mut best = 0; let mut empty_above = 0; let mut t_end = 0; let mut first_empty = None;
        for t in 0..20000 {
            let w = &g.ctx.w;
            let p = w.position(g.cm.node);
            let c = &g.cm;
            let standing = c.jump == 0 && c.fall_time == 0;
            let land = 50.0 + 40.0 * c.height as f32;
            let above = land + 40.0;
            // blocks of the row above
            let row: Vec<f32> = g.blocks.iter().map(|b| w.position(b.node)).filter(|b| (b.y - (above - 8.0)).abs() < 5.0 && b.x.abs() < 200.0).map(|b| b.x).collect();
            if row.is_empty() && standing { empty_above += 1; if first_empty.is_none() { first_empty = Some((t, c.height)); } }
            let dir = if c.movement > 0.0 { -1.0 } else { 1.0 };
            let ahead = p.x + dir * 6.0;
            let clear = row.iter().all(|&bx| (bx - p.x).abs() > 12.0 && (bx - ahead).abs() > 12.0);
            let target = p.x + dir * 35.0;
            let wrap = |x: f32| if x > 170.0 { x - 337.0 } else if x < -175.0 { x + 337.0 } else { x };
            let lands = (0..6).any(|k| (c.y_min[k] + 10.0 - above).abs() < 1.0 && {
                let t = wrap(target);
                (t > c.x_min[k] + 4.0 && t < c.x_max[k] - 4.0) || (t > c.x_min2[k] + 4.0 && t < c.x_max2[k] - 4.0)
            });
            g.ctx.input.held[2] = standing && clear && lands;
            g.frame();
            best = best.max(g.cm.height);
            if g.ct.death == 1 {
                t_end = t;
                let w = &g.ctx.w;
                let p = w.position(g.cm.node);
                let cam = w.position(g.cam.cam).y;
                let near: Vec<_> = g.barricade_nodes.iter().chain(g.blockade_nodes.iter()).map(|&n| (w.nodes[n].name.clone(), w.position(n))).filter(|(_, q)| (q.y - p.y).abs() < 40.0 && (q.x - p.x).abs() < 30.0).map(|(n, q)| (n, q.x as i32, q.y as i32)).collect();
                println!("  death: player ({:.1},{:.1}) h={} cam={cam:.1} minYObst={:.1} rows={:?} near={near:?}", p.x, p.y, g.cm.height, g.cam.min_y_obstacle, (0..6).map(|k| (g.cm.y_min[k], g.cm.x_min[k], g.cm.x_max[k], g.cm.x_min2[k], g.cm.x_max2[k])).collect::<Vec<_>>());
                break;
            }
        }
        println!("seed {seed}: best height {best}, died at {t_end}, ticks standing with no row above {empty_above}, first {:?}, pts {}", first_empty, g.score.points);
    }
}

fn bot_input(g: &mut squarelights::game::GameScene) {
    let w = &g.ctx.w;
    let p = w.position(g.cm.node);
    let c = &g.cm;
    let standing = c.jump == 0 && c.fall_time == 0;
    let above = 50.0 + 40.0 * c.height as f32 + 40.0;
    let row: Vec<f32> = g.blocks.iter().map(|b| w.position(b.node)).filter(|b| (b.y - (above - 8.0)).abs() < 5.0 && b.x.abs() < 200.0).map(|b| b.x).collect();
    let dir = if c.movement > 0.0 { -1.0 } else { 1.0 };
    let clear = row.iter().all(|&bx| (bx - p.x).abs() > 12.0 && (bx - (p.x + dir * 6.0)).abs() > 12.0);
    let wrap = |x: f32| if x > 170.0 { x - 337.0 } else if x < -175.0 { x + 337.0 } else { x };
    let t = wrap(p.x + dir * 35.0);
    let lands = (0..6).any(|k| (c.y_min[k] + 10.0 - above).abs() < 1.0 && ((t > c.x_min[k] + 4.0 && t < c.x_max[k] - 4.0) || (t > c.x_min2[k] + 4.0 && t < c.x_max2[k] - 4.0)));
    g.ctx.input.held[2] = standing && clear && lands;
}

#[test]
#[ignore]
fn invisible_platforms() {
    let mut events = 0;
    for seed in 1..=20u64 {
        let mut g = game(seed);
        for t in 0..6000 {
            bot_input(&mut g);
            g.frame();
            if g.ct.death == 1 { break; }
            let w = &g.ctx.w;
            let cam = w.position(g.cam.cam).y;
            let c = &g.cm;
            for k in 0..6 {
                let land = c.y_min[k] + 10.0;
                if (land - cam).abs() > 95.0 { continue; }
                for (lo, hi) in [(c.x_min[k], c.x_max[k]), (c.x_min2[k], c.x_max2[k])] {
                    if hi - lo < 1.0 { continue; }
                    let any = g.blocks.iter().any(|b| { let q = w.position(b.node); (q.y - (land - 8.0)).abs() < 16.0 && q.x > lo - 12.0 && q.x < hi + 12.0 });
                    if !any { events += 1; if events < 10 { println!("seed {seed} t={t} row land={land} [{lo},{hi}] has NO blocks (cam {cam:.0}, h {})", c.height); } }
                }
            }
        }
    }
    println!("invisible-platform ticks: {events}");
}

#[test]
#[ignore]
fn chunk_depths() {
    let g = game(1);
    let w = &g.ctx.w;
    let bg = w.position(w.find("Background")).z;
    let cam = w.position(g.cam.cam).z;
    println!("camera z {cam}, background z {bg}");
    for (k, (&s, &d)) in g.pg.singles.iter().zip(g.pg.doubles.iter()).enumerate() {
        for (node, spawn_z) in [(s, 100.0f32), (d, 128.0)] {
            let cz = w.position(node).z;
            let mut nodes = Vec::new();
            w.descendants(node, &mut nodes);
            let zs: Vec<f32> = nodes.iter().filter(|&&n| w.nodes[n].scripts.iter().any(|x| x == "Block")).map(|&n| w.position(n).z - cz + spawn_z).collect();
            let (lo, hi) = zs.iter().fold((f32::MAX, f32::MIN), |a, &z| (a.0.min(z), a.1.max(z)));
            println!("{} #{}: {} blocks, z after spawn {lo:.1}..{hi:.1}{}", w.nodes[node].name, k + 1, zs.len(), if hi >= bg { "  <-- BEHIND BACKGROUND" } else { "" });
        }
    }
}

#[test]
#[ignore]
fn all_depths() {
    use std::collections::BTreeMap;
    let mut offenders: BTreeMap<String, (usize, f32)> = BTreeMap::new();
    for seed in 1..=10u64 {
        let mut g = game(seed);
        for t in 0..4000 {
            bot_input(&mut g);
            g.frame();
            if g.ct.death == 1 { break; }
            if t % 7 != 0 { continue; }
            let w = &g.ctx.w;
            let cam = w.position(g.cam.cam);
            let bgz = w.position(w.find("Background")).z;
            for (i, n) in w.nodes.iter().enumerate() {
                if n.mesh.is_none() || !n.visible || n.name == "Background" { continue; }
                let p = w.position(i);
                if (p.y - cam.y).abs() > 100.0 || p.x.abs() > 170.0 { continue; }
                if p.z >= bgz - 0.5 || p.z <= cam.z + 0.3 {
                    let e = offenders.entry(format!("{} (parent {:?})", n.name, n.parent.map(|q| w.nodes[q].name.clone()))).or_insert((0, p.z));
                    e.0 += 1; e.1 = p.z;
                }
            }
        }
    }
    for (k, v) in &offenders { println!("{k}: {} samples, z={:.1}", v.0, v.1); }
    println!("offender kinds: {}", offenders.len());
}

#[test]
#[ignore]
fn chunk_extents_vs_ranges() {
    let mut g = game(1);
    // force each chunk through spawn_zone1 via the public generator state
    for single in [true, false] {
        for k in 1..=10 {
            g.pg.switcher = if single { 0 } else { 1 };
            g.pg.platform_number = k;
            g.debug_spawn();
            let w = &g.ctx.w;
            let chunk = if single { g.pg.singles[k as usize - 1] } else { g.pg.doubles[k as usize - 1] };
            let mut nodes = Vec::new();
            w.descendants(chunk, &mut nodes);
            let mut xs: Vec<f32> = nodes.iter().filter(|&&n| w.nodes[n].scripts.iter().any(|s| s == "Block")).map(|&n| w.position(n).x).collect();
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            // split into contiguous runs (blocks are 12.5 apart)
            let mut runs: Vec<(f32, f32)> = Vec::new();
            for x in xs { match runs.last_mut() { Some(r) if x - r.1 < 14.0 => r.1 = x, _ => runs.push((x, x)) } }
            let c = &g.cm;
            println!("{} {k}: blocks {:?} | land [{},{}] [{},{}]", if single {"single"} else {"double"},
                runs.iter().map(|r| ((r.0 - 6.0) as i32, (r.1 + 6.0) as i32)).collect::<Vec<_>>(), c.x_min[5], c.x_max[5], c.x_min2[5], c.x_max2[5]);
        }
    }
}
