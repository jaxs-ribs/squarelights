# Square Lights — browser port

Square Lights (Three Lights Left, Unity 4.3.4, Android, 2014) rebuilt outside Unity:
the simulation is Rust compiled to WebAssembly, and rendering is WebGPU.

```
tools/unity4/   reader for Unity 4.3 serialized files (no UnityPy): scenes, meshes,
                textures (RGBA4444 / ETC1 / RGB24), materials, audio, fonts
engine/         Rust crate: scene graph + line-by-line port of every MonoBehaviour
  src/game/     the SquareLights scene (player, platforms, pickups, HUD)
  src/menu.rs   the Menu scene (panels, options, tutorial slides)
  tests/        headless gameplay tests driving the same API the browser uses
web/            index.html + main.js (WebGPU renderer, input, WebAudio) + extracted assets
```

## Run

```bash
./build.sh
```

```bash
python3 -m http.server 8642 --bind 127.0.0.1 --directory web
```

Then open http://127.0.0.1:8642/ in a WebGPU browser (Chrome/Edge 113+, Safari 26+).

Controls: the original touch controls (swipe left/right anywhere to change direction,
tap the bottom-right area to jump) work with mouse or touch. Keyboard: ←/→ change
direction, ↑ / Space jump, P pause, R retry, E exit to menu.

URL flags: `?debug` live state overlay, `?start=game` skip the menu, `?seed=N`
deterministic run, `?autoplay` hold jump.

## Test

```bash
cargo test --release --manifest-path engine/Cargo.toml
```

`window.__sq` in the page exposes the live game state (`__sq.state()`), fps, draw calls
and errors for browser automation.

## Fidelity notes

- Game logic runs at Unity's fixed 50 Hz step; Update logic runs once per tick and
  rendering interpolates between ticks.
- Unity's per-object script order is unspecified; this port picks one fixed order.
- `Random` is a seeded xorshift, not Unity's generator, so layouts differ run to run
  but follow the same rules and odds.
- AdMob banners and the (disabled) screenshot recorder are dropped.
