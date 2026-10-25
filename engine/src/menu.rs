//! The "Menu" scene: MenuManager (camera panning between panels, sound/music
//! toggles, tutorial slides) plus the decorative FloatingText/Fragments,
//! ScaleUpDown, Rotation and PlatformVibrateLoop scripts.

use crate::input::{Key, Phase};
use crate::math::v3;
use crate::runtime::{Ctx, MusicState, SceneId};
use crate::world::Space;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCall {
    Shrink(Btn),
    ShrinkTutorialExit,
    PlayGame,
    OpenCredits,
    OpenOptions,
    CloseOptions,
    CloseCredits,
    CloseTutorial,
    NextSlide,
    OpenTutorial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Btn {
    Play,
    Options,
    Tutorial,
    OptionsExit,
    CreditsExit,
    Next,
    Sound,
    Music,
    Credits,
}

const BUTTONS: [(Btn, &str); 9] = [
    (Btn::Play, "GUIPlay"),
    (Btn::Options, "GUIOptions"),
    (Btn::Tutorial, "GUITutorial"),
    (Btn::OptionsExit, "GUIOptionsExit"),
    (Btn::CreditsExit, "GUICreditsExit"),
    (Btn::Next, "GUINext"),
    (Btn::Sound, "GUISONOFF"),
    (Btn::Music, "GUIMONOFF"),
    (Btn::Credits, "GUICredits"),
];

pub type MenuCtx = Ctx<MenuCall>;

struct Floater {
    node: usize,
    timer: f32,
    kind: u8, // 0 text (world y), 1 fragments (self y), 2 horizontal (world x)
}

pub struct MenuScene {
    pub ctx: MenuCtx,
    cam: usize,
    action: i32,
    slide_count: i32,
    gui: [(usize, i32, i32); 9], // (node, hit, option)
    off_music: usize,
    off_sound: usize,
    on_music: usize,
    on_sound: usize,
    logo: [usize; 9],
    next_extra: [usize; 3],
    tutorial_exit_logo: usize,
    floaters: Vec<Floater>,
    scalers: Vec<(usize, i32)>,
    rotators: Vec<usize>,
    vibrators: Vec<(usize, i32)>,
}

const HIDDEN: crate::math::V3 = v3(-100.0, -100.0, 0.0);
const SOUND_POS: crate::math::V3 = v3(-33.0, -75.0, 0.0);
const MUSIC_POS: crate::math::V3 = v3(-33.0, -92.0, 0.0);

impl MenuScene {
    pub fn new(ctx: MenuCtx) -> MenuScene {
        let w = &ctx.w;
        let f = |n: &str| w.find(n);
        let gui = BUTTONS.map(|(_, name)| (f(name), 0, 0));
        // logos sized by each button, in BUTTONS order (Sound/Music have none)
        let logo = [
            f("PlayLogo"),
            f("OptionsLogo"),
            f("TutorialLogo"),
            f("OptionsExitLogo"),
            f("CreditsExitLogo"),
            f("NextLogo"),
            usize::MAX,
            usize::MAX,
            f("CreditsLogo"),
        ];
        let next_extra = [f("NextLogo1"), f("NextLogo2"), f("thankslogo")];
        let mut floaters = Vec::new();
        for (script, kind) in [("FloatingText", 0u8), ("FloatingFragments", 1), ("FloatingTextHorizontal", 2)] {
            for node in w.with_script(script) {
                floaters.push(Floater { node, timer: 0.0, kind });
            }
        }
        let scalers = w.with_script("ScaleUpDown").into_iter().map(|n| (n, 0)).collect();
        let rotators = w.with_script("Rotation");
        let vibrators = w.with_script("PlatformVibrateLoop").into_iter().map(|n| (n, 0)).collect();
        let yellow = w.with_script("YellowColor");
        let mut m = MenuScene {
            cam: f("MainCamera"),
            action: 0,
            slide_count: -1,
            gui,
            off_music: f("OffMusic"),
            off_sound: f("OffSound"),
            on_music: f("OnMusic"),
            on_sound: f("OnSound"),
            logo,
            next_extra,
            tutorial_exit_logo: f("TutorialExitLogo"),
            floaters,
            scalers,
            rotators,
            vibrators,
            ctx,
        };
        for n in yellow {
            m.ctx.w.nodes[n].color = [1.0, 0.85, 0.47, 1.0];
        }
        m.start();
        m
    }

    fn option(&mut self, b: Btn, v: i32) {
        self.gui[b as usize].2 = v;
    }

    fn hit(&self, b: Btn) -> bool {
        self.gui[b as usize].1 == 1
    }

    fn opt(&self, b: Btn) -> bool {
        self.gui[b as usize].2 == 1
    }

    fn start(&mut self) {
        self.ctx.music.state = MusicState::Playing;
        for f in &mut self.floaters {
            f.timer = self.ctx.rng.range(1, 90) as f32;
        }
        self.slide_count = -1;
        self.action = 0;
        self.option(Btn::Play, 1);
        self.option(Btn::Options, 1);
        self.option(Btn::Tutorial, 1);
        let w = &mut self.ctx.w;
        w.set_position(self.off_music, HIDDEN);
        w.set_position(self.off_sound, HIDDEN);
        w.set_position(self.on_music, MUSIC_POS);
        w.set_position(self.on_sound, SOUND_POS);
        self.ctx.prefs.set_int("loading", 1);
        if !self.ctx.prefs.has_key("firstTime") {
            self.ctx.w.set_position(self.cam, v3(200.0, -200.0, 100.0));
            self.slide_count = -1;
            self.action = 0;
            for (b, v) in [
                (Btn::Next, 1),
                (Btn::Play, 0),
                (Btn::Options, 0),
                (Btn::Tutorial, 0),
                (Btn::Sound, 0),
                (Btn::Music, 0),
                (Btn::Credits, 0),
                (Btn::CreditsExit, 0),
            ] {
                self.option(b, v);
            }
            self.ctx.prefs.set_int("firstTime", 1);
        }
        if !self.ctx.prefs.has_key("music") {
            self.ctx.prefs.set_int("music", 1);
        }
        if !self.ctx.prefs.has_key("sound") {
            self.ctx.prefs.set_int("sound", 1);
        }
        self.sync_toggles();
    }

    fn sync_toggles(&mut self) {
        let music = self.ctx.prefs.get_int("music") == 1;
        let sound = self.ctx.prefs.get_int("sound") == 1;
        let w = &mut self.ctx.w;
        w.set_position(self.off_music, if music { HIDDEN } else { MUSIC_POS });
        w.set_position(self.on_music, if music { MUSIC_POS } else { HIDDEN });
        w.set_position(self.off_sound, if sound { HIDDEN } else { SOUND_POS });
        w.set_position(self.on_sound, if sound { SOUND_POS } else { HIDDEN });
    }

    pub fn frame(&mut self) {
        self.ctx.frame += 1;
        self.ctx.time += crate::runtime::FIXED_DT as f64;
        self.fixed_update();
        self.update();
        for call in self.ctx.due_invokes() {
            self.dispatch(call);
        }
    }

    fn fixed_update(&mut self) {
        let w = &mut self.ctx.w;
        for f in &mut self.floaters {
            f.timer += 1.0;
            let t = f.timer;
            let amp = match f.kind {
                0 => 0.05,
                1 => 0.01,
                _ => 0.1,
            };
            let d = if t < 45.0 {
                amp * (45.0 / (t + 11.0)).sqrt().sqrt()
            } else if t > 45.0 && t < 90.0 {
                -amp * (45.0 / (t - 34.0)).sqrt().sqrt()
            } else {
                0.0
            };
            if d != 0.0 {
                match f.kind {
                    0 => w.translate(f.node, 0.0, d, 0.0, Space::World),
                    1 => w.translate(f.node, 0.0, d, 0.0, Space::Self_),
                    _ => w.translate(f.node, d, 0.0, 0.0, Space::World),
                }
            }
            if t > 90.0 {
                f.timer = 0.0;
            }
        }
        for s in &mut self.scalers {
            s.1 += 1;
            if s.1 < 45 {
                w.add_scale(s.0, v3(0.001, 0.001, 0.001));
            }
            if s.1 < 90 && s.1 >= 45 {
                w.add_scale(s.0, v3(-0.001, -0.001, -0.001));
            }
            if s.1 >= 90 {
                s.1 = 0;
            }
        }
        for &r in &self.rotators {
            w.rotate(r, 0.0, 0.0, 1.0, Space::World);
        }
        for v in &mut self.vibrators {
            v.1 += 1;
            let t = v.1;
            if t > 160 {
                w.set_scale(v.0, v3(0.4, 0.4, 1.0));
                w.set_position(v.0, v3(-456.0, -221.0, -38.0));
                v.1 = 0;
            }
            let t = v.1;
            if t <= 120 {
                let mut s = 20;
                while s < 120 {
                    if t > s && t < s + 5 {
                        let dir = if ((s - 20) / 5) % 2 == 0 { 1.0 } else { -1.0 };
                        w.translate(v.0, dir, 0.0, 0.0, Space::World);
                    }
                    s += 5;
                }
                if t > 100 && t < 158 {
                    w.translate(v.0, 0.0, -7.0, 0.0, Space::World);
                }
            }
            if t > 0 && t < 80 {
                w.add_scale(v.0, v3(0.0, -0.002, 0.0));
            }
            if t > 80 && t < 150 {
                w.add_scale(v.0, v3(-0.004, 0.0, 0.0));
            }
            if t > 150 && t < 160 {
                w.add_scale(v.0, v3(-0.05, -0.03, 0.0));
            }
        }
    }

    fn update(&mut self) {
        // GUIConditioned
        let touches = self.ctx.input.touches.clone();
        for g in &mut self.gui {
            if touches.is_empty() || g.2 != 1 {
                continue;
            }
            for t in &touches {
                g.1 = 0;
                if self.ctx.w.gui_hit(g.0, t.x, t.y) {
                    if t.phase == Phase::Began {
                        g.1 = 1;
                    }
                    if t.phase == Phase::Ended {
                        g.1 = 0;
                    }
                }
            }
        }
        // MusicController
        self.ctx.music.mute = self.ctx.prefs.get_int("music") == 0;

        let key = |s: &Self, k: Key, b: Btn| s.ctx.input.get_key_down(k) && s.opt(b);
        if (self.hit(Btn::Sound) || key(self, Key::S, Btn::Sound)) && self.action == 0 {
            self.ctx.play("Bubble");
            let on = self.ctx.prefs.get_int("sound") == 1;
            self.ctx.prefs.set_int("sound", if on { 0 } else { 1 });
            self.sync_toggles();
            self.slide_count = -1;
        }
        if (self.hit(Btn::Music) || key(self, Key::M, Btn::Music)) && self.action == 0 {
            self.ctx.play("Bubble");
            let on = self.ctx.prefs.get_int("music") == 1;
            self.ctx.prefs.set_int("music", if on { 0 } else { 1 });
            self.sync_toggles();
            self.slide_count = -1;
        }
        let simple: [(Btn, Key, MenuCall); 6] = [
            (Btn::Credits, Key::C, MenuCall::OpenCredits),
            (Btn::Play, Key::P, MenuCall::PlayGame),
            (Btn::Options, Key::O, MenuCall::OpenOptions),
            (Btn::Tutorial, Key::T, MenuCall::OpenTutorial),
            (Btn::OptionsExit, Key::E, MenuCall::CloseOptions),
            (Btn::CreditsExit, Key::E, MenuCall::CloseCredits),
        ];
        for (b, k, call) in simple {
            if (self.hit(b) || key(self, k, b)) && self.action == 0 {
                self.ctx.play("Bubble");
                self.action = 1;
                self.grow(b, 0.02);
                self.ctx.invoke(MenuCall::Shrink(b), 0.3);
                self.ctx.invoke(call, 0.5);
                if matches!(b, Btn::Credits | Btn::Play) {
                    self.slide_count = -1;
                }
            }
        }
        let next = self.hit(Btn::Next) || key(self, Key::N, Btn::Next);
        if next && self.action == 0 && self.slide_count < 3 {
            self.ctx.play("Bubble");
            self.action = 1;
            self.grow(Btn::Next, 0.02);
            self.ctx.invoke(MenuCall::Shrink(Btn::Next), 0.3);
            self.ctx.invoke(MenuCall::NextSlide, 0.5);
            self.slide_count += 1;
        } else if next && self.action == 0 {
            self.ctx.play("Bubble");
            self.action = 1;
            for _ in 0..5 {
                self.ctx.w.add_scale(self.tutorial_exit_logo, v3(0.02, 0.02, 0.0));
            }
            self.ctx.invoke(MenuCall::ShrinkTutorialExit, 0.3);
            self.ctx.invoke(MenuCall::CloseTutorial, 0.5);
            self.slide_count = 0;
        }
    }

    fn grow(&mut self, b: Btn, d: f32) {
        let mut nodes = vec![self.logo[b as usize]];
        if b == Btn::Next {
            nodes.extend(self.next_extra);
        }
        for n in nodes {
            if n != usize::MAX {
                for _ in 0..5 {
                    self.ctx.w.add_scale(n, v3(d, d, 0.0));
                }
            }
        }
    }

    fn set_options(&mut self, opts: [(Btn, i32); 9]) {
        for (b, v) in opts {
            self.option(b, v);
        }
    }

    fn dispatch(&mut self, call: MenuCall) {
        use Btn::*;
        let cam = self.cam;
        match call {
            MenuCall::Shrink(b) => self.grow(b, -0.02),
            MenuCall::ShrinkTutorialExit => {
                for _ in 0..5 {
                    self.ctx.w.add_scale(self.tutorial_exit_logo, v3(-0.02, -0.02, 0.0));
                }
            }
            MenuCall::PlayGame => {
                self.action = 0;
                self.ctx.load_request = Some(SceneId::Game);
            }
            MenuCall::OpenCredits => {
                self.action = 0;
                self.set_options([
                    (Next, 0), (OptionsExit, 0), (Play, 0), (Options, 0), (Tutorial, 0),
                    (Sound, 0), (Music, 0), (Credits, 0), (CreditsExit, 1),
                ]);
                self.ctx.w.translate(cam, -200.0, 0.0, 0.0, Space::World);
            }
            MenuCall::OpenOptions => {
                self.action = 0;
                self.set_options([
                    (Next, 0), (OptionsExit, 1), (Play, 0), (Options, 0), (Tutorial, 0),
                    (Sound, 1), (Music, 1), (Credits, 1), (CreditsExit, 0),
                ]);
                self.ctx.w.translate(cam, 0.0, -100.0, 0.0, Space::World);
            }
            MenuCall::CloseOptions | MenuCall::CloseCredits | MenuCall::CloseTutorial => {
                self.action = 0;
                let keep = if call == MenuCall::CloseTutorial { self.gui[OptionsExit as usize].2 } else { 0 };
                self.set_options([
                    (Next, 0), (OptionsExit, keep), (Play, 1), (Options, 1), (Tutorial, 1),
                    (Sound, 0), (Music, 0), (Credits, 0), (CreditsExit, 0),
                ]);
                let d = match call {
                    MenuCall::CloseOptions => (0.0, 100.0),
                    MenuCall::CloseCredits => (200.0, 100.0),
                    _ => (600.0, 200.0),
                };
                self.ctx.w.translate(cam, d.0, d.1, 0.0, Space::World);
            }
            MenuCall::NextSlide | MenuCall::OpenTutorial => {
                if call == MenuCall::OpenTutorial {
                    self.slide_count = 0;
                }
                self.action = 0;
                self.set_options([
                    (Next, 1), (OptionsExit, self.gui[OptionsExit as usize].2), (Play, 0), (Options, 0), (Tutorial, 0),
                    (Sound, 0), (Music, 0), (Credits, 0), (CreditsExit, 0),
                ]);
                let d = if call == MenuCall::NextSlide { (-200.0, 0.0) } else { (0.0, -200.0) };
                self.ctx.w.translate(cam, d.0, d.1, 0.0, Space::World);
            }
        }
    }

    pub fn debug_state(&self) -> serde_json::Value {
        let c = self.ctx.w.position(self.cam);
        serde_json::json!({
            "scene": "menu",
            "frame": self.ctx.frame,
            "camera": [c.x, c.y],
            "action": self.action,
            "slide": self.slide_count,
            "options": self.gui.iter().map(|g| g.2).collect::<Vec<_>>(),
            "sound": self.ctx.prefs.get_int("sound"),
            "music": self.ctx.prefs.get_int("music"),
        })
    }
}
