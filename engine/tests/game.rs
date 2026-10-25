//! Headless gameplay tests: drive the real Engine (same API the browser uses)
//! and assert on the observable state.

mod common;
use common::*;

const UP: usize = 2;
const P: usize = 3;
const R: usize = 4;
const PREFS: &str = r#"{"loading":0,"sound":1,"music":1,"firstTime":1}"#;

fn run(e: &mut squarelights::Engine, n: usize) {
    for _ in 0..n {
        e.frame();
    }
}

#[test]
fn intro_then_runs_at_125_units_per_second() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 30);
    let s = state(&e);
    assert_eq!(f(&s, "movement"), 0.0, "stands still during the intro animation");
    let x0 = f(&s, "player.0");
    run(&mut e, 40);
    let s = state(&e);
    assert_eq!(f(&s, "movement"), -125.0);
    // 125 u/s * 0.02 s = 2.5 u per tick; with MovTimer the first ticks may be skipped
    let x1 = f(&s, "player.0");
    assert!(x1 - x0 > 2.5 * 30.0, "moved {} units", x1 - x0);
    assert_eq!(f(&s, "player.1"), 50.0, "standing on the ground row (y = 50 + 40*h)");
}

#[test]
fn jump_impulse_follows_original_integer_formula() {
    let mut e = engine("game", PREFS, 1);
    // tick 77 puts the runner (x~20) under the gap of the double platform above;
    // jumping elsewhere bumps the head on a block (Block.cs zeroes `jump`).
    run(&mut e, 77);
    e.key(UP, true);
    let mut ys = vec![f(&state(&e), "player.1")];
    for _ in 0..4 {
        e.frame();
        ys.push(f(&state(&e), "player.1"));
    }
    let d: Vec<f64> = ys.windows(2).map(|w| w[1] - w[0]).collect();
    // JumpPower = (int)(17/(fallTime+1) + fallTime/5): 17, 8, 5, 4 ...
    // the first tick only latches the jump (landing check runs after movement)
    assert_eq!(&d[1..4], &[17.0, 8.0, 5.0], "per-tick rise {d:?}");
    assert_eq!(state(&e)["jump"], 1);
}

#[test]
fn holding_up_climbs_and_spawns_new_rows() {
    let mut e = engine("game", PREFS, 7);
    e.key(UP, true);
    let mut best = 0.0;
    for _ in 0..1300 {
        e.frame();
        let s = state(&e);
        best = f64::max(best, f(&s, "height"));
    }
    let s = state(&e);
    assert!(best >= 4.0, "reached height {best}");
    assert!(f(&s, "divider") >= 4.0, "generator advanced: {}", s["divider"]);
    assert!(f(&s, "coins") >= 1.0, "picked up coins on the way: {}", s["coins"]);
    assert!(f(&s, "points") > 1000.0);
}

#[test]
fn falling_kills_and_shows_end_screen_with_highscore() {
    let mut e = engine("game", PREFS, 3);
    // idle: the starting platforms crumble after ~21-23 s
    let mut died_at = None;
    for t in 0..2500 {
        e.frame();
        if died_at.is_none() && state(&e)["dead"] == 1 {
            died_at = Some(t);
        }
    }
    let t = died_at.expect("player should die when the initial platforms fall");
    assert!((1000..1400).contains(&t), "died at tick {t}");
    let s = state(&e);
    assert_eq!(s["lives"], 0);
    assert!(f(&s, "teleport") > 60.0, "end screen sequence ran");
    let points = f(&s, "points");
    let prefs: serde_json::Value = serde_json::from_str(&e.take_prefs()).unwrap();
    assert_eq!(prefs["rank1"].as_f64().unwrap(), points, "new highscore saved");
    assert_eq!(prefs["loading"], 0);
    let texts: serde_json::Value = serde_json::from_str(&e.texts()).unwrap();
    let all: Vec<String> = texts.as_object().unwrap().values().map(|v| v[0].as_str().unwrap().to_string()).collect();
    assert!(all.iter().any(|t| t == &format!("1.   {}", points as i64)), "rank1 shown: {all:?}");
    assert!(all.iter().any(|t| t == "New Highscore!!!"));
}

#[test]
fn retry_reloads_the_scene() {
    let mut e = engine("game", PREFS, 3);
    run(&mut e, 200);
    e.key(R, true);
    e.frame();
    e.key(R, false);
    run(&mut e, 25);
    let s = state(&e);
    assert!(f(&s, "frame") < 10.0, "scene restarted, frame={}", s["frame"]);
    assert_eq!(f(&s, "points"), 0.0);
}

#[test]
fn pause_freezes_the_simulation() {
    let mut e = engine("game", PREFS, 3);
    run(&mut e, 100);
    e.key(P, true);
    e.frame();
    e.key(P, false);
    let a = state(&e);
    assert_eq!(a["paused"], true);
    run(&mut e, 50);
    let b = state(&e);
    assert_eq!(a["player"], b["player"]);
    assert_eq!(a["time"], b["time"]);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&e.music()).unwrap()["state"], "paused");
    e.key(P, true);
    e.frame();
    e.key(P, false);
    run(&mut e, 5);
    let c = state(&e);
    assert_eq!(c["paused"], false);
    assert_ne!(b["player"], c["player"]);
}

#[test]
fn tap_in_jump_zone_jumps() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 77);
    // bottom-right quarter of the 800x480 screen is the jump button area
    e.pointer(1, 650.0, 100.0, 0);
    let y0 = f(&state(&e), "player.1");
    run(&mut e, 4);
    e.pointer(1, 650.0, 100.0, 2);
    run(&mut e, 4);
    let y1 = f(&state(&e), "player.1");
    assert!(y1 > y0 + 20.0, "jumped from {y0} to {y1}");
}

#[test]
fn swipe_turns_the_runner_around() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 60);
    assert_eq!(f(&state(&e), "movement"), -125.0);
    // swipe right-to-left on the upper half (outside the jump zone)
    e.pointer(2, 400.0, 300.0, 0);
    e.frame();
    for k in 1..5 {
        e.pointer(2, 400.0 - 15.0 * k as f32, 300.0, 1);
        e.frame();
    }
    e.pointer(2, 330.0, 300.0, 2);
    run(&mut e, 2);
    let m = f(&state(&e), "movement");
    assert!(m > 0.0, "movement flipped to {m}");
}

#[test]
fn keyboard_left_right_set_direction() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 60);
    let x0 = f(&state(&e), "player.0");
    e.key(0, true); // Left
    e.frame();
    e.key(0, false);
    run(&mut e, 10);
    let x1 = f(&state(&e), "player.0");
    assert!(x1 < x0, "LeftArrow moves left: {x0} -> {x1}");
    e.key(1, true); // Right
    e.frame();
    e.key(1, false);
    run(&mut e, 10);
    let x2 = f(&state(&e), "player.0");
    assert!(x2 > x1, "RightArrow moves right: {x1} -> {x2}");
}

#[test]
fn same_seed_same_game() {
    let play = |seed| {
        let mut e = engine("game", PREFS, seed);
        e.key(UP, true);
        run(&mut e, 1500);
        e.state()
    };
    assert_eq!(play(11), play(11));
}

#[test]
fn render_records_are_well_formed() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 80);
    let r = e.render_vec(0.5);
    assert_eq!(r.len() % 28, 0);
    let n = r.len() / 28;
    assert!(n > 50, "{n} instances visible");
    let tables: serde_json::Value = serde_json::from_str(&e.tables()).unwrap();
    let mats = tables["materials"].as_array().unwrap().len() as f32;
    let transparent: Vec<bool> = tables["transparent"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
    let mut seen_transparent = false;
    let mut last_depth = f32::INFINITY;
    let mut texts = 0;
    for i in 0..n {
        let rec = &r[i * 28..(i + 1) * 28];
        assert!(rec.iter().all(|v| v.is_finite()), "record {i} has NaN");
        assert!(rec[2] < mats);
        let is_transparent = rec[0] == 1.0 || transparent[rec[2] as usize];
        if is_transparent {
            seen_transparent = true;
            assert!(rec[3] <= last_depth + 1e-3, "transparent queue sorted far-to-near");
            last_depth = rec[3];
        } else {
            assert!(!seen_transparent, "opaque record {i} after the transparent queue");
        }
        texts += (rec[0] == 1.0) as usize;
    }
    assert!(texts >= 1, "score text is drawn");
    let s = state(&e);
    // the character sprite uses animated UV offsets from the 16x8 sheet
    let uv = &s["frameUV"];
    assert!(f(uv, "1") == 0.75 || f(uv, "1") == 0.875, "running row: {uv}");
}

#[test]
fn chaos_input_soak_never_breaks_invariants() {
    let mut e = engine("game", PREFS, 99);
    let mut rng = 12345u64;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for t in 0..20_000 {
        let r = next();
        match r % 40 {
            0 => e.key((r >> 8) as usize % 3, true),
            1 => e.key((r >> 8) as usize % 3, false),
            2 => e.pointer(1, (r >> 16) as f32 % 800.0, (r >> 32) as f32 % 480.0, 0),
            3 => e.pointer(1, (r >> 16) as f32 % 800.0, (r >> 32) as f32 % 480.0, 1),
            4 => e.pointer(1, 0.0, 0.0, 2),
            5 if t % 3000 == 0 => {
                e.key(R, true);
                e.frame();
                e.key(R, false);
            }
            _ => {}
        }
        e.frame();
        if t % 97 == 0 {
            let s = state(&e);
            if s["scene"] == "menu" {
                // a random tap hit "Exit" on the end screen; tap Play to go back in
                e.pointer(9, 400.0, 380.0, 0);
                e.frame();
                e.pointer(9, 400.0, 380.0, 2);
                continue;
            }
            let lives = f(&s, "lives");
            assert!((0.0..=4.0).contains(&lives), "lives {lives}");
            assert!(f(&s, "player.0").abs() <= 176.0, "x stays wrapped: {}", s["player"]);
            assert!(f(&s, "player.1").is_finite());
            let r = e.render_vec(1.0);
            assert!(r.iter().all(|v| v.is_finite()));
        }
    }
}

/// Regression: every mesh drawn in front of the camera must be nearer than the
/// opaque background plane, or it is hidden (singlechunk8 used to be).
#[test]
fn nothing_gameplay_relevant_hides_behind_the_background() {
    for seed in 1..=12u64 {
        let mut e = engine("game", PREFS, seed);
        e.key(UP, true);
        let mut bg_depth = None;
        for t in 0..2500 {
            e.frame();
            if t % 5 != 0 {
                continue;
            }
            let r = e.render_vec(1.0);
            let tables: serde_json::Value = serde_json::from_str(&e.tables()).unwrap();
            let mats: Vec<String> = tables["materials"].as_array().unwrap().iter().map(|m| m.as_str().unwrap().to_string()).collect();
            let assets: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!("{}/assets.json", assets_dir())).unwrap()).unwrap();
            let name = |m: f32| assets["materials"][&mats[m as usize]]["name"].as_str().unwrap().to_string();
            for rec in r.chunks(28) {
                if rec[0] == 0.0 && name(rec[2]) == "Background" {
                    bg_depth = Some(rec[3]);
                }
            }
            let bg = bg_depth.expect("background drawn");
            for rec in r.chunks(28) {
                if rec[0] == 0.0 && name(rec[2]) == "blockColored" {
                    assert!(rec[3] < bg - 1.0, "seed {seed} t={t}: block at depth {} behind background {bg} (pos {},{})", rec[3], rec[16], rec[17]);
                }
            }
        }
    }
}

/// Regression: after a swipe, the arrow keys must still change direction.
#[test]
fn arrow_keys_still_work_after_a_swipe() {
    let mut e = engine("game", PREFS, 1);
    run(&mut e, 60);
    e.pointer(2, 400.0, 300.0, 0);
    e.frame();
    for k in 1..5 {
        e.pointer(2, 400.0 - 15.0 * k as f32, 300.0, 1);
        e.frame();
    }
    e.pointer(2, 330.0, 300.0, 2);
    run(&mut e, 3);
    assert!(f(&state(&e), "movement") > 0.0);
    e.key(1, true); // Right
    e.frame();
    e.key(1, false);
    run(&mut e, 5);
    assert!(f(&state(&e), "movement") < 0.0, "Right arrow overrides the old swipe");
}
