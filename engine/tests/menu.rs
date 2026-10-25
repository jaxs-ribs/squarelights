//! Menu flow tests: tutorial on first launch, Play -> game with the fake
//! loading screen, option toggles persisted to PlayerPrefs.

mod common;
use common::*;

fn tap(e: &mut squarelights::Engine, x: f32, y: f32) {
    e.pointer(1, x, y, 0);
    e.frame();
    e.pointer(1, x, y, 2);
    e.frame();
}

fn run(e: &mut squarelights::Engine, n: usize) {
    for _ in 0..n {
        e.frame();
    }
}

#[test]
fn first_launch_opens_the_tutorial_slides() {
    let mut e = engine("menu", "{}", 1);
    e.frame();
    let s = state(&e);
    assert_eq!(s["scene"], "menu");
    assert_eq!(s["camera"][0], 200.0);
    assert_eq!(s["camera"][1], -200.0);
    let prefs: serde_json::Value = serde_json::from_str(&e.take_prefs()).unwrap();
    assert_eq!(prefs["firstTime"], 1);
    assert_eq!(prefs["sound"], 1);
    // "Next" button lives bottom-right: inset (160,-230) from screen center
    let (nx, ny) = (400.0 + 160.0 + 100.0, 240.0 - 230.0 + 50.0);
    for slide in 0..4 {
        tap(&mut e, nx, ny);
        run(&mut e, 30);
        let s = state(&e);
        assert_eq!(f(&s, "camera.0"), 200.0 - 200.0 * (slide + 1) as f64, "slide {slide}");
    }
    // the last Next closes the tutorial back to the main panel at (0,0)
    tap(&mut e, nx, ny);
    run(&mut e, 30);
    let s = state(&e);
    assert_eq!((f(&s, "camera.0"), f(&s, "camera.1")), (0.0, 0.0));
}

#[test]
fn play_starts_the_game_without_the_fake_loading_screen() {
    let mut e = engine("menu", r#"{"firstTime":1,"sound":1,"music":1}"#, 1);
    run(&mut e, 5);
    assert_eq!(f(&state(&e), "camera.0"), 0.0);
    // Play: inset (-210,65,460,150) around the screen center
    tap(&mut e, 400.0, 240.0 + 65.0 + 75.0);
    run(&mut e, 30);
    let s = state(&e);
    assert_eq!(s["scene"], "game");
    assert_eq!(s["loading"], 0);
    // 30 ticks after the tap the scene is a few frames old; the original
    // 36-tick intro animation still plays, then the run starts (~0.7 s, not 6.7 s)
    run(&mut e, 40);
    assert_eq!(f(&state(&e), "movement"), -125.0, "running right after the intro");
}

#[test]
fn options_toggle_sound_and_music() {
    let mut e = engine("menu", r#"{"firstTime":1,"sound":1,"music":1}"#, 1);
    run(&mut e, 5);
    // Options: inset (-200,-30,420,90)
    tap(&mut e, 400.0, 240.0 - 30.0 + 45.0);
    run(&mut e, 30);
    assert_eq!(f(&state(&e), "camera.1"), -100.0);
    // Sound toggle: inset (-250,105,550,100); Music: (-250,3,550,100)
    tap(&mut e, 400.0, 240.0 + 105.0 + 50.0);
    tap(&mut e, 400.0, 240.0 + 3.0 + 50.0);
    let s = state(&e);
    assert_eq!(s["sound"], 0);
    assert_eq!(s["music"], 0);
    let m: serde_json::Value = serde_json::from_str(&e.music()).unwrap();
    assert_eq!(m["mute"], true);
    // exit options: inset (225,-225,150,80)
    tap(&mut e, 400.0 + 225.0 + 75.0, 240.0 - 225.0 + 40.0);
    run(&mut e, 30);
    assert_eq!(f(&state(&e), "camera.1"), 0.0);
}
