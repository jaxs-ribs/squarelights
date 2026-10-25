use squarelights::Engine;

pub fn assets_dir() -> String {
    format!("{}/../web/assets", env!("CARGO_MANIFEST_DIR"))
}

pub fn engine(start: &str, prefs: &str, seed: u64) -> Engine {
    let d = assets_dir();
    let read = |f: &str| std::fs::read_to_string(format!("{d}/{f}")).unwrap();
    Engine::new(read("scene_menu.json"), read("scene_game.json"), read("assets.json"), prefs, seed, start)
}

pub fn state(e: &Engine) -> serde_json::Value {
    serde_json::from_str(&e.state()).unwrap()
}

pub fn f(v: &serde_json::Value, path: &str) -> f64 {
    let mut cur = v;
    for p in path.split('.') {
        cur = match p.parse::<usize>() {
            Ok(i) => &cur[i],
            Err(_) => &cur[p],
        };
    }
    cur.as_f64().unwrap_or_else(|| panic!("{path} not a number in {v}"))
}
