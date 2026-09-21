//! `vpet-cli autoplay`: plays the pet by policy and writes the input log it produced as an
//! ordinary `.vlog` (docs/TESTING.md). Long goldens (both evolution branches, a full life to
//! old age) need a caretaker that reacts to state — a blind, fixed-cadence script can't answer
//! a tantrum or know when a poop landed — so this drives a `Cart`, reads `inspect()` after
//! every step, and records exactly the `t` lines it issued. Deterministic per seed; the output
//! replays through `vpet-cli replay` with no new grammar.
//!
//! Nothing here touches simulation logic: it is a host, like the browser, that happens to
//! press buttons on a schedule (CLAUDE.md "Hosts are dumb blitters").

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::ValueEnum;
use vpet_core::{buttons, Cart, Inspect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Policy {
    /// Visits right after every scheduled event (via `inspect().next_event_at`, capped by
    /// `--every-secs`) and answers everything: medicine, clean, discipline on a tantrum, a meal
    /// at hunger >= 40, play every visit, lights on `sleepy`. Reaches the well-cared-for adult.
    Caretaker,
    /// The same actions but only on a fixed `--every-secs` cadence, so 15-minute calls
    /// (tantrum, sleepy) are usually missed: care mistakes pile up, discipline stays low,
    /// and the child becomes the alternate adult -- but it is fed and cleaned enough to live.
    Sloppy,
    /// Never presses a button.
    Neglect,
}

#[derive(Debug, Clone)]
pub struct Opts {
    pub seed: u64,
    pub start_ms: u64,
    pub until_secs: u64,
    pub policy: Policy,
    pub every_secs: u64,
    /// Stop (after the snap) once the pet reaches any adult stage.
    pub stop_at_adult: bool,
}

// Mirrors `vpet_core::assets::generated::icon::*` order (docs/GAME_DESIGN.md's 4x2 menu).
const ICON_FEED: u8 = 0;
const ICON_LIGHTS: u8 = 1;
const ICON_PLAY: u8 = 2;
const ICON_MEDICINE: u8 = 3;
const ICON_CLEAN: u8 = 4;
const ICON_DISCIPLINE: u8 = 6;

// Mirrors `vpet_core::pet::attention::*` and the `Inspect.flags` bits (docs/HOST_ABI.md).
const ATT_TANTRUM: u8 = 1 << 5;
const ATT_SLEEPY: u8 = 1 << 4;
const FLAG_SICK: u8 = 1 << 0;
const FLAG_SLEEPING: u8 = 1 << 1;

const STATE_DEAD: u8 = 2;
const STAGE_EGG: u8 = 0;
const STAGE_NAMES: [&str; 5] = ["egg", "baby", "child", "adult", "adult_alt"];

struct Driver {
    cart: Cart,
    now_ms: u64,
    lines: Vec<String>,
}

fn buttons_token(mask: u8) -> String {
    if mask == 0 {
        return "-".to_string();
    }
    let mut s = String::new();
    if mask & buttons::A != 0 {
        s.push('A');
    }
    if mask & buttons::B != 0 {
        s.push('B');
    }
    if mask & buttons::C != 0 {
        s.push('C');
    }
    s
}

impl Driver {
    fn t(&mut self, mask: u8) {
        self.cart.update(self.now_ms, mask);
        self.lines
            .push(format!("t {} {}", self.now_ms, buttons_token(mask)));
    }

    fn press(&mut self, mask: u8) {
        self.now_ms += 100;
        self.t(mask);
        self.now_ms += 100;
        self.t(0);
    }

    fn wait_secs(&mut self, secs: u64) {
        self.now_ms += secs * 1000;
        self.t(0);
    }

    fn snap(&mut self, name: &str) {
        self.lines.push(format!("snap {name}"));
    }

    fn comment(&mut self, text: &str) {
        self.lines.push(format!("# {text}"));
    }

    fn open_menu_at(&mut self, icon: u8) {
        self.press(buttons::A);
        for _ in 0..icon {
            self.press(buttons::A);
        }
    }

    /// Select `icon`, press any `extra` buttons the action needs, then wait out the longest
    /// busy animation (eating: 5 s) plus a margin.
    fn action(&mut self, icon: u8, extra: &[u8]) {
        self.open_menu_at(icon);
        self.press(buttons::B);
        for &b in extra {
            self.press(b);
        }
        self.wait_secs(6);
    }

    fn inspect(&self) -> Inspect {
        self.cart.inspect()
    }

    /// One visit's worth of care, for the caretaker and sloppy policies.
    fn care(&mut self) {
        let insp = self.inspect();
        let sick = insp.flags & FLAG_SICK != 0;
        let sleeping = insp.flags & FLAG_SLEEPING != 0;
        if sick {
            self.action(ICON_MEDICINE, &[]);
        }
        if insp.poops > 0 {
            self.action(ICON_CLEAN, &[]);
        }
        if sleeping {
            if insp.attention & ATT_SLEEPY != 0 {
                self.action(ICON_LIGHTS, &[]);
            }
            return; // everything else is refused while asleep
        }
        if insp.attention & ATT_TANTRUM != 0 {
            self.action(ICON_DISCIPLINE, &[]);
        }
        if insp.hunger >= 40 {
            self.action(ICON_FEED, &[buttons::B]); // FeedSub -> meal
        }
        // Play every visit: the only way to lose weight (docs/GAME_DESIGN.md), and it keeps
        // happiness up. Five rounds, always guessing left.
        self.action(
            ICON_PLAY,
            &[buttons::A, buttons::A, buttons::A, buttons::A, buttons::A],
        );
    }
}

pub fn run(opts: &Opts, out: &Path) -> Result<()> {
    let mut d = Driver {
        cart: Cart::new_uninit(),
        now_ms: opts.start_ms,
        lines: Vec::new(),
    };
    let policy_name = format!("{:?}", opts.policy).to_lowercase();
    d.comment(&format!(
        "{}: generated by `vpet-cli autoplay --seed {} --start-ms {} --until-secs {} \
         --policy {} --every-secs {}{}`; re-run that to regenerate.",
        out.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("autoplay.vlog"),
        opts.seed,
        opts.start_ms,
        opts.until_secs,
        policy_name,
        opts.every_secs,
        if opts.stop_at_adult {
            " --stop-at-adult"
        } else {
            ""
        }
    ));
    d.lines
        .push(format!("reset {} seed={}", opts.start_ms, opts.seed));
    d.cart.reset(opts.start_ms, opts.seed);

    let end_ms = opts.start_ms + opts.until_secs * 1000;
    let mut last_stage = STAGE_EGG;

    loop {
        // Where to look next.
        let insp = d.inspect();
        let next_ms = match opts.policy {
            Policy::Caretaker if insp.next_event_at != u32::MAX => {
                let event_ms = (insp.next_event_at as u64 + 1) * 1000;
                event_ms
                    .max(d.now_ms + 60_000)
                    .min(d.now_ms + opts.every_secs * 1000)
            }
            _ => d.now_ms + opts.every_secs * 1000,
        };
        if next_ms > end_ms {
            break;
        }
        d.now_ms = next_ms;
        d.t(0);

        let insp = d.inspect();
        if insp.state == STATE_DEAD {
            d.snap("dead");
            break;
        }
        if insp.stage != last_stage {
            let name = STAGE_NAMES[insp.stage as usize];
            if last_stage != STAGE_EGG {
                // The Evolving clip is on screen right now; keep one frame of it.
                d.snap(&format!("{name}_evolving"));
                d.wait_secs(5);
            }
            d.snap(if insp.stage == 1 { "hatched" } else { name });
            last_stage = insp.stage;
            if opts.stop_at_adult && insp.stage >= 3 {
                break;
            }
        }

        match opts.policy {
            Policy::Caretaker | Policy::Sloppy => d.care(),
            Policy::Neglect => {}
        }
    }

    d.snap("end");
    let text = d.lines.join("\n") + "\n";
    fs::write(out, text).with_context(|| format!("writing {out:?}"))?;
    let final_insp = d.inspect();
    eprintln!(
        "autoplay: {} lines, ended at stage {} state {} (care_mistakes {}, discipline {}) -> {}",
        d.lines.len(),
        STAGE_NAMES[final_insp.stage as usize],
        final_insp.state,
        final_insp.care_mistakes,
        final_insp.discipline,
        out.display()
    );
    Ok(())
}
