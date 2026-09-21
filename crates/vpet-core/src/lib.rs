//! vpet-core: the cartridge. Owns the simulation, the rules, the compiled-in assets, and the
//! 64x32 one-bit framebuffer. No allocator, no clock, no I/O — see docs/ARCHITECTURE.md and
//! docs/DETERMINISM.md. `std` is enabled only under `cfg(test)` (property tests need it) and
//! will later be enabled under the `dev-overrides` feature (docs/CONTENT.md "Hot preview").
#![cfg_attr(not(any(test, feature = "dev-overrides")), no_std)]
#![deny(clippy::float_arithmetic)]

mod actions;
pub mod anim;
pub mod assets;
#[cfg(feature = "dev-overrides")]
pub mod dev;
mod events;
pub mod meter;
pub mod pet;
pub mod render;
pub mod rng;
pub mod save;
pub mod time;
pub mod timers;
pub mod ui;

use anim::{AnimState, ClipId};
use assets::{generated, SpeciesDef, StageRules, StageSet};
use meter::Meter;
use pet::{Pet, Stage};
use render::fb::FRAME_LEN;
use render::Fb;
use rng::Rng;
use save::v1::SaveV1;
use save::LoadError;
use time::{Sec, NEVER};
use timers::{EventKind, Timers};
use ui::{BusyKind, Ui};

/// Bumps on any change to exports, buffer sizes, frame format, flag bits, error codes, or
/// `Inspect` (docs/HOST_ABI.md "Versioning"). 2: the frame became 64x32 / 256 bytes
/// (docs/adr/0016-screen-64x32.md).
pub const ABI_VERSION: u32 = 2;

pub mod buttons {
    pub const A: u8 = 1 << 0;
    pub const B: u8 = 1 << 1;
    pub const C: u8 = 1 << 2;
}

pub mod flags {
    pub const FRAME_CHANGED: u32 = 1 << 0;
    pub const SAVE_NEEDED: u32 = 1 << 1;
    pub const BEEP: u32 = 1 << 2;
    pub const ATTENTION: u32 = 1 << 3;
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartState {
    Uninit = 0,
    Alive = 1,
    Dead = 2,
}

impl CartState {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(CartState::Uninit),
            1 => Some(CartState::Alive),
            2 => Some(CartState::Dead),
            _ => None,
        }
    }
}

/// Mirrors docs/HOST_ABI.md's `Inspect` layout field-for-field (the `#[repr(C)]` packed version
/// lives in `vpet-abi`; this is the native equivalent used by the CLI and tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inspect {
    pub abi_version: u32,
    pub save_version: u16,
    pub content_hash: u32,
    pub state: u8,
    pub species: u8,
    pub stage: u8,
    pub hunger: u8,
    pub happiness: u8,
    pub discipline: u8,
    pub health: u8,
    pub weight: u16,
    pub age_secs: u32,
    pub flags: u8,
    pub attention: u8,
    pub poops: u8,
    pub care_mistakes: u8,
    pub sim_now: u32,
    pub next_event_at: u32,
}

pub struct Cart {
    pub state: CartState,
    pub sim_now: Sec,
    rng: [u32; 4],
    pub pet: Pet,
    pub timers: Timers,
    pub ui: Ui,
    pub attention_since: [Sec; 6],

    // Not persisted (docs/STATE_MODEL.md).
    prev_buttons: u8,
    anim: AnimState,
    prev_frame: [u8; FRAME_LEN],
    /// `flags::*` bits raised by actions/events since the last `update` returned
    /// (`BEEP`, `SAVE_NEEDED`); drained into that update's return value.
    pending_flags: u32,
    /// `now_ms` at which an A+C hold on the tombstone began (docs/HOST_ABI.md "Buttons":
    /// "A+C held for 1000 ms on the tombstone restarts"). `None` while not holding.
    hold_since_ms: Option<u64>,
}

impl Cart {
    pub const fn new_uninit() -> Self {
        Cart {
            state: CartState::Uninit,
            sim_now: 0,
            rng: [0; 4],
            pet: Pet {
                species: 0,
                stage: Stage::Egg,
                hatched_at: NEVER,
                stage_since: 0,
                hunger: Meter::new(0, 0, 0, 1),
                happy: Meter::new(100, 0, 0, -1),
                discipline: 0,
                health: 100,
                weight: 1,
                poops: 0,
                poop_since: NEVER,
                sick: false,
                sleeping: false,
                lights_off: false,
                attention: 0,
                care_mistakes: 0,
                sick_count: 0,
                snacks_since_wake: 0,
                wins: 0,
                losses: 0,
                last_battle_at: NEVER,
                death_cause: 0,
            },
            timers: Timers::new(),
            ui: Ui::Idle,
            attention_since: [NEVER; 6],
            prev_buttons: 0,
            anim: AnimState::new(),
            prev_frame: [0u8; FRAME_LEN],
            pending_flags: 0,
            hold_since_ms: None,
        }
    }

    // --- host-facing API (mirrors docs/HOST_ABI.md's exports) ---------------------------------

    /// Start a new egg at `now_ms`. `seed` is host entropy. Species is the registry default
    /// (the first entry of `generated::SPECIES`); a reset-with-species variant is future ABI
    /// work (docs/HOST_ABI.md).
    pub fn reset(&mut self, now_ms: u64, seed: u64) {
        let now = time::ms_to_sec(now_ms);
        self.state = CartState::Alive;
        self.sim_now = now;
        self.rng = Rng::seed_from_u64(seed).state();
        self.pet = Pet::new_egg(generated::SPECIES[0].id, now);
        self.timers = Timers::new();
        let hatch_secs = self.species_def().hatch_secs;
        self.timers
            .set(EventKind::Hatch, now.saturating_add(hatch_secs));
        self.ui = Ui::Idle;
        self.attention_since = [NEVER; 6];
        self.prev_buttons = 0;
        self.anim = AnimState::new();
        self.pending_flags = 0;
        self.hold_since_ms = None;
        let tick = time::tick(now_ms);
        self.anim
            .on_state_change(ClipId::Egg, tick, now, self.rng[0]);
        self.prev_frame = *self.render_frame(tick).as_bytes();
    }

    /// Advance to `now_ms`, apply input, render. Returns flags (`flags::*`).
    pub fn update(&mut self, now_ms: u64, buttons: u8) -> u32 {
        let frame_before = self.prev_frame;
        let target = time::ms_to_sec(now_ms);
        let event_fired = self.advance_to(target);

        let restarted = self.check_restart_hold(now_ms, buttons);
        if !restarted {
            let rising = buttons & !self.prev_buttons;
            self.prev_buttons = buttons;
            self.apply_input(rising);
        }

        let tick = time::tick(now_ms);
        self.retarget_clip(tick);

        let fb = self.render_frame(tick);
        let frame_changed = fb.as_bytes() != &frame_before;
        self.prev_frame = *fb.as_bytes();

        let mut out = self.pending_flags;
        self.pending_flags = 0;
        if frame_changed {
            out |= flags::FRAME_CHANGED;
        }
        if event_fired || restarted {
            out |= flags::SAVE_NEEDED;
        }
        if self.state == CartState::Alive && self.pet.attention != 0 {
            out |= flags::ATTENTION;
        }
        out
    }

    /// docs/HOST_ABI.md "Buttons": A+C held for 1000 ms on the tombstone starts a new egg.
    /// Returns `true` if the restart happened in this call. Sub-second time is allowed for
    /// exactly this (docs/DETERMINISM.md rule 2); the new egg itself is keyed to
    /// `floor(now_ms / 1000)` through `reset`. The seed is drawn from the dead pet's RNG
    /// stream so a replay of the same history restarts into the same egg.
    fn check_restart_hold(&mut self, now_ms: u64, buttons: u8) -> bool {
        if self.state != CartState::Dead || buttons != buttons::A | buttons::C {
            self.hold_since_ms = None;
            return false;
        }
        let since = *self.hold_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < 1000 {
            return false;
        }
        let mut rng = Rng::from_state(self.rng);
        let seed = ((rng.next_u32() as u64) << 32) | rng.next_u32() as u64;
        self.reset(now_ms, seed);
        // The buttons are still down; they must not read as a fresh press on the new egg.
        self.prev_buttons = buttons;
        self.pending_flags |= flags::BEEP;
        true
    }

    /// Raises the host flags an action's outcome implies (docs/HOST_ABI.md "Update flags":
    /// `BEEP` on every applied or refused action, `SAVE_NEEDED` when something was applied).
    pub(crate) fn note_outcome(&mut self, outcome: actions::Outcome) {
        self.pending_flags |= flags::BEEP;
        if outcome == actions::Outcome::Applied {
            self.pending_flags |= flags::SAVE_NEEDED;
        }
    }

    pub fn frame(&self) -> &[u8; FRAME_LEN] {
        &self.prev_frame
    }

    /// Encode the current state into `out`, using `scratch` as postcard's working buffer.
    /// Returns the blob length, or `Some(0)` while `Uninit` (docs/HOST_ABI.md: "0 means
    /// Uninit").
    pub fn save(&self, scratch: &mut [u8], out: &mut [u8]) -> Option<usize> {
        if self.state == CartState::Uninit {
            return Some(0);
        }
        let save = SaveV1 {
            state: self.state as u8,
            sim_now: self.sim_now,
            rng: self.rng,
            pet: self.pet.clone(),
            timers: self.timers,
            ui: self.ui,
            attention_since: self.attention_since,
        };
        save::v1::encode(&save, generated::CONTENT_HASH, scratch, out)
    }

    /// Replace the current state with `blob`. On any error the state is left unchanged
    /// (docs/HOST_ABI.md "Load errors"). Does not re-render; call `update` next so the frame
    /// reflects the loaded state (this matches every host's documented flow: load, then loop
    /// calling `update`).
    pub fn load(&mut self, blob: &[u8]) -> Result<(), LoadError> {
        let (header, payload) = save::read_header_and_payload(blob)?;
        if header.save_version > save::v1::SAVE_VERSION {
            return Err(LoadError::VersionTooNew);
        }
        // Only v1 exists so far; a future v2 will branch on header.save_version here and
        // convert forward through `From<SaveV1> for SaveV2` (docs/SAVE_FORMAT.md).
        let decoded = save::v1::decode(payload)?;
        if !generated::SPECIES
            .iter()
            .any(|s| s.id == decoded.pet.species)
        {
            return Err(LoadError::BadContentRef);
        }
        let state = CartState::from_u8(decoded.state).ok_or(save::LoadError::Decode)?;

        self.state = state;
        self.sim_now = decoded.sim_now;
        self.rng = decoded.rng;
        self.pet = decoded.pet;
        self.timers = decoded.timers;
        self.ui = decoded.ui;
        self.attention_since = decoded.attention_since;
        self.prev_buttons = 0;
        self.anim = AnimState::new();
        self.pending_flags = 0;
        self.hold_since_ms = None;
        Ok(())
    }

    pub fn inspect(&self) -> Inspect {
        let hunger = self.pet.hunger.value_at(self.sim_now);
        let happiness = self.pet.happy.value_at(self.sim_now);
        let age_secs = if self.pet.hatched_at == NEVER {
            0
        } else {
            self.sim_now.saturating_sub(self.pet.hatched_at)
        };
        let mut iflags = 0u8;
        if self.pet.sick {
            iflags |= 1 << 0;
        }
        if self.pet.sleeping {
            iflags |= 1 << 1;
        }
        if self.pet.lights_off {
            iflags |= 1 << 2;
        }
        if self.pet.attention != 0 {
            iflags |= 1 << 3;
        }
        let next_event_at = self.timers.earliest().map(|(t, _)| t).unwrap_or(NEVER);
        Inspect {
            abi_version: ABI_VERSION,
            save_version: save::v1::SAVE_VERSION,
            content_hash: generated::CONTENT_HASH,
            state: self.state as u8,
            species: self.pet.species,
            stage: self.pet.stage.as_u8(),
            hunger,
            happiness,
            discipline: self.pet.discipline,
            health: self.pet.health,
            weight: self.pet.weight,
            age_secs,
            flags: iflags,
            attention: self.pet.attention,
            poops: self.pet.poops,
            care_mistakes: self.pet.care_mistakes,
            sim_now: self.sim_now,
            next_event_at,
        }
    }

    // --- simulation core ----------------------------------------------------------------------

    /// docs/STATE_MODEL.md "Fast-forward". Event-stepped: jumps between scheduled timers, never
    /// one second at a time. `Dead`/`Uninit` cost O(1) regardless of the gap; time never moves
    /// backwards (rule 4: a `target` behind `sim_now` is a no-op here, though input still
    /// applies at the current `sim_now` in `update`).
    /// Returns whether any timer actually fired (used for `SAVE_NEEDED`; docs/HOST_ABI.md:
    /// "not on every animation tick" — simulated time simply moving forward, with nothing
    /// scheduled in the gap, does not count).
    fn advance_to(&mut self, target: Sec) -> bool {
        if self.state != CartState::Alive {
            if target > self.sim_now {
                self.sim_now = target;
            }
            return false;
        }
        if target <= self.sim_now {
            return false;
        }
        let mut fired = false;
        loop {
            match self.timers.earliest() {
                Some((t, kind)) if t <= target => {
                    self.sim_now = t;
                    self.fire(kind);
                    fired = true;
                }
                _ => break,
            }
        }
        self.sim_now = target;
        fired
    }

    fn fire(&mut self, kind: EventKind) {
        match kind {
            EventKind::Hatch => self.fire_hatch(),
            EventKind::MenuTimeout => {
                self.ui = Ui::Idle;
                self.timers.clear(kind);
            }
            EventKind::UiBusyEnd => self.fire_ui_busy_end(),
            EventKind::Sleep => self.fire_sleep(),
            EventKind::Wake => self.fire_wake(),
            EventKind::Poop => self.fire_poop(),
            EventKind::Tantrum => self.fire_tantrum(),
            EventKind::SickOnset => self.fire_sick_onset(),
            EventKind::SickDamage => self.fire_sick_damage(),
            EventKind::HungerEmpty => self.fire_hunger_empty(),
            EventKind::HappyEmpty => self.fire_happy_empty(),
            EventKind::CareMiss => self.fire_care_miss(),
            EventKind::Starve => self.fire_starve(),
            EventKind::Evolve => self.fire_evolve(),
            EventKind::OldAge => self.fire_old_age(),
        }
    }

    fn fire_hatch(&mut self) {
        let now = self.sim_now;
        self.pet.stage = Stage::Baby;
        self.pet.hatched_at = now;
        self.pet.stage_since = now;

        let species = self.species_def();
        let awake_secs = species.schedule.0;
        let rules = species.baby.rules;

        self.pet
            .hunger
            .reanchor_with_step(now, 0, rules.hunger_step_secs);
        self.pet
            .happy
            .reanchor_with_step(now, 100, rules.happy_step_secs);

        self.timers.clear(EventKind::Hatch);
        self.timers
            .set(EventKind::Evolve, now.saturating_add(rules.stage_secs));
        self.timers
            .set(EventKind::Sleep, now.saturating_add(awake_secs));
        let poop_gap =
            self.next_rand_range(rules.poop_interval_min_secs, rules.poop_interval_max_secs);
        self.timers
            .set(EventKind::Poop, now.saturating_add(poop_gap));
        self.timers.set(
            EventKind::HungerEmpty,
            self.pet.hunger.reaches(generated::GAME.hungry_threshold),
        );
        self.timers.set(
            EventKind::HappyEmpty,
            self.pet.happy.reaches(generated::GAME.sad_threshold),
        );
    }

    fn next_rand_range(&mut self, lo: u32, hi: u32) -> u32 {
        let mut rng = Rng::from_state(self.rng);
        let v = rng.range(lo, hi.max(lo));
        self.rng = rng.state();
        v
    }

    fn apply_input(&mut self, rising: u8) {
        if self.state != CartState::Alive || self.pet.stage == Stage::Egg {
            return; // docs/GAME_DESIGN.md: "Egg: ... Buttons are ignored."
        }
        match self.ui {
            Ui::Idle => {
                if rising & buttons::A != 0 {
                    self.open_menu(0);
                }
            }
            Ui::Menu { cursor } => {
                if rising & buttons::A != 0 {
                    let next = (cursor + 1) % generated::icon::COUNT as u8;
                    self.open_menu(next);
                } else if rising & buttons::B != 0 {
                    self.timers.clear(EventKind::MenuTimeout);
                    actions::select_menu_icon(self, cursor);
                } else if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                    self.timers.clear(EventKind::MenuTimeout);
                }
            }
            Ui::FeedSub { snack } => {
                if rising & buttons::A != 0 {
                    self.ui = Ui::FeedSub { snack: !snack };
                } else if rising & buttons::B != 0 {
                    let outcome = if snack {
                        actions::feed_snack(self)
                    } else {
                        actions::feed_meal(self)
                    };
                    self.note_outcome(outcome);
                } else if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                }
            }
            Ui::Status { page } => {
                if rising & buttons::A != 0 {
                    self.ui = Ui::Status {
                        page: (page + 1) % 5,
                    };
                } else if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                }
            }
            Ui::Busy {
                kind: BusyKind::Playing { .. },
            } => {
                if rising & buttons::A != 0 {
                    actions::play_guess(self, false);
                } else if rising & buttons::B != 0 {
                    actions::play_guess(self, true);
                } else if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                    self.timers.clear(EventKind::UiBusyEnd);
                }
            }
            Ui::Busy { .. } => {
                // Eating/Refuse/Discipline/Result: a fixed-length animation, no input accepted
                // except an early C to skip it (docs/GAME_DESIGN.md doesn't specify this; it's a
                // reasonable UX default and never changes simulation state beyond what the
                // action already applied).
                if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                    self.timers.clear(EventKind::UiBusyEnd);
                }
            }
            Ui::Battle { .. } => {} // Phase 6
        }
    }

    fn open_menu(&mut self, cursor: u8) {
        self.ui = Ui::Menu { cursor };
        self.timers.set(
            EventKind::MenuTimeout,
            self.sim_now
                .saturating_add(generated::GAME.menu_timeout_secs),
        );
    }

    // --- rendering ------------------------------------------------------------------------

    fn desired_clip(&self) -> ClipId {
        if self.state == CartState::Dead {
            return ClipId::Dead;
        }
        if self.state != CartState::Alive {
            return ClipId::Main; // Uninit: never actually rendered (render_frame short-circuits)
        }
        if self.pet.stage == Stage::Egg {
            return ClipId::Egg;
        }
        match self.ui {
            Ui::Busy {
                kind: BusyKind::Eating { .. },
            } => ClipId::Eating,
            Ui::Busy {
                kind: BusyKind::Refuse,
            } => ClipId::Refuse,
            Ui::Busy {
                kind: BusyKind::Discipline,
            } => ClipId::DisciplineBusy,
            Ui::Busy {
                kind: BusyKind::Result { .. },
            } => ClipId::Result,
            Ui::Busy {
                kind: BusyKind::Playing { .. },
            } => ClipId::Playing,
            Ui::Busy {
                kind: BusyKind::Evolving,
            } => ClipId::Evolving,
            Ui::Menu { .. } => ClipId::Menu,
            Ui::FeedSub { .. } => ClipId::FeedSub,
            Ui::Status { .. } => ClipId::Status,
            Ui::Battle { .. } => ClipId::Main, // Phase 6
            Ui::Idle => {
                if self.pet.sick {
                    ClipId::Sick
                } else if self.pet.sleeping {
                    ClipId::Sleeping
                } else {
                    ClipId::Main
                }
            }
        }
    }

    fn retarget_clip(&mut self, tick: u32) {
        let desired = self.desired_clip();
        if self.anim.clip != desired {
            let rng0 = self.rng[0];
            self.anim.on_state_change(desired, tick, self.sim_now, rng0);
        }
    }

    fn age_days(&self) -> u32 {
        if self.pet.hatched_at == NEVER {
            0
        } else {
            self.sim_now.saturating_sub(self.pet.hatched_at) / 86_400
        }
    }

    fn render_frame(&self, tick: u32) -> Fb {
        use render::compose;

        if self.state == CartState::Dead {
            return compose::render_dead();
        }
        if self.state != CartState::Alive {
            return Fb::new();
        }
        if self.pet.stage == Stage::Egg {
            let species = self.species_def();
            let egg_sprites = species.egg.as_ref().unwrap_or(&generated::EGG);
            let remaining = self
                .timers
                .get(EventKind::Hatch)
                .saturating_sub(self.sim_now);
            return compose::render_egg(egg_sprites, remaining, tick, &self.anim);
        }

        let species = self.species_def();
        let stage_set = self.stage_set(species);

        // The player's own UI is never darkened; every scene that shows the pet in its room
        // is, while the lights are off (docs/art/SCREEN_LAYOUT.md "Lights off while awake").
        // The Sleeping scene inverts itself, so it is excluded here to avoid a double flip.
        let room_scene = !matches!(
            self.ui,
            Ui::Menu { .. } | Ui::FeedSub { .. } | Ui::Status { .. }
        );
        let mut fb = self.render_scene(species, stage_set, tick);
        if room_scene && self.pet.lights_off && !self.pet.sleeping {
            fb.invert();
        }
        fb
    }

    fn render_scene(
        &self,
        species: &'static SpeciesDef,
        stage_set: &'static StageSet,
        tick: u32,
    ) -> Fb {
        use render::compose;

        match self.ui {
            Ui::Menu { cursor } => compose::render_menu(&generated::ICONS, cursor),
            Ui::FeedSub { snack } => compose::render_feed_sub(snack),
            Ui::Status { page } => compose::render_status(
                page,
                self.pet.hunger.value_at(self.sim_now),
                self.pet.happy.value_at(self.sim_now),
                self.pet.discipline,
                self.pet.health,
                self.pet.sick,
                self.age_days(),
                self.pet.weight,
            ),
            Ui::Busy {
                kind: BusyKind::Evolving,
            } => {
                let old_set = stage_set_of(species, previous_stage(self.pet.stage));
                compose::render_evolving(old_set, stage_set, tick, &self.anim)
            }
            Ui::Busy {
                kind: BusyKind::Eating { snack },
            } => compose::render_eating(stage_set, snack, tick, &self.anim),
            Ui::Busy {
                kind: BusyKind::Refuse,
            } => compose::render_refuse(stage_set, tick, &self.anim),
            Ui::Busy {
                kind: BusyKind::Discipline,
            } => compose::render_discipline_busy(stage_set, tick, &self.anim),
            Ui::Busy {
                kind: BusyKind::Result { won },
            } => compose::render_result(stage_set, won, tick, &self.anim),
            Ui::Busy {
                kind: BusyKind::Playing { seq, round, .. },
            } => compose::render_playing(stage_set, seq, round, tick, &self.anim),
            _ => {
                // Ui::Idle (sick/sleeping/plain), and Busy{Evolving}/Battle which are Phase 4/6
                // stand-ins for Main until those systems exist.
                if self.pet.sick {
                    compose::render_sick(stage_set, tick)
                } else if self.pet.sleeping {
                    compose::render_sleeping(
                        stage_set,
                        self.pet.lights_off,
                        self.pet.attention,
                        tick,
                    )
                } else {
                    compose::render_main(
                        stage_set,
                        self.pet.poops,
                        self.pet.attention,
                        tick,
                        &self.anim,
                    )
                }
            }
        }
    }

    fn species_def(&self) -> &'static SpeciesDef {
        generated::SPECIES
            .iter()
            .find(|s| s.id == self.pet.species)
            .unwrap_or(&generated::SPECIES[0])
    }

    fn stage_set(&self, species: &'static SpeciesDef) -> &'static StageSet {
        stage_set_of(species, self.pet.stage)
    }

    pub(crate) fn stage_rules(&self) -> StageRules {
        let species = self.species_def();
        self.stage_set(species).rules
    }
}

/// The pose set a species uses at `stage`. `Egg` maps to `baby` (only the egg sprites are
/// drawn then, but the rules table is what `Hatch` anchors to); `AdultAlt` falls back to
/// `adult` for a species without an alternate adult.
pub(crate) fn stage_set_of(species: &'static SpeciesDef, stage: Stage) -> &'static StageSet {
    match stage {
        Stage::Egg | Stage::Baby => &species.baby,
        Stage::Child => &species.child,
        Stage::Adult => &species.adult,
        Stage::AdultAlt => species.adult_alt.as_ref().unwrap_or(&species.adult),
    }
}

/// The stage a pet evolved *from* to reach `stage` (what the Evolving clip shows first).
pub(crate) fn previous_stage(stage: Stage) -> Stage {
    match stage {
        Stage::Egg | Stage::Baby => Stage::Egg,
        Stage::Child => Stage::Baby,
        Stage::Adult | Stage::AdultAlt => Stage::Child,
    }
}

impl Default for Cart {
    fn default() -> Self {
        Self::new_uninit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_starts_an_egg() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        assert_eq!(cart.state, CartState::Alive);
        assert_eq!(cart.pet.stage, Stage::Egg);
        let insp = cart.inspect();
        assert_eq!(insp.stage, Stage::Egg.as_u8());
        assert_ne!(insp.next_event_at, NEVER);
    }

    #[test]
    fn hatches_after_hatch_secs() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        // hatch_secs = 300 for the placeholder species; jump well past it in one call.
        cart.update(1_700_000_000_000 + 300_000, 0);
        assert_eq!(cart.pet.stage, Stage::Baby);
        let insp = cart.inspect();
        assert_ne!(insp.next_event_at, NEVER); // Evolve/Sleep/Poop/... now pending
    }

    #[test]
    fn buttons_ignored_on_an_egg() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        cart.update(1_700_000_001_000, buttons::A);
        assert_eq!(cart.ui, Ui::Idle);
    }

    #[test]
    fn a_opens_and_advances_the_menu_after_hatch() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        cart.update(1_700_000_000_000 + 300_000, 0); // hatch
        cart.update(1_700_000_000_000 + 300_100, buttons::A);
        assert_eq!(cart.ui, Ui::Menu { cursor: 0 });
        // Release then press again: A held-mask rising-edge model requires 0 in between.
        cart.update(1_700_000_000_000 + 300_200, 0);
        cart.update(1_700_000_000_000 + 300_300, buttons::A);
        assert_eq!(cart.ui, Ui::Menu { cursor: 1 });
    }

    #[test]
    fn c_closes_the_menu() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        cart.update(1_700_000_000_000 + 300_000, 0);
        cart.update(1_700_000_000_000 + 300_100, buttons::A);
        cart.update(1_700_000_000_000 + 300_200, 0);
        cart.update(1_700_000_000_000 + 300_300, buttons::C);
        assert_eq!(cart.ui, Ui::Idle);
    }

    #[test]
    fn save_load_round_trip_preserves_state() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        cart.update(1_700_000_000_000 + 300_000, 0);
        let mut scratch = [0u8; 512];
        let mut blob = [0u8; 512];
        let len = cart.save(&mut scratch, &mut blob).unwrap();

        let mut cart2 = Cart::new_uninit();
        cart2.load(&blob[0..len]).unwrap();
        assert_eq!(cart2.state, cart.state);
        assert_eq!(cart2.sim_now, cart.sim_now);
        assert_eq!(cart2.pet, cart.pet);
        assert_eq!(cart2.timers, cart.timers);
        assert_eq!(cart2.ui, cart.ui);
    }

    #[test]
    fn save_of_uninit_is_zero_length() {
        let cart = Cart::new_uninit();
        let mut scratch = [0u8; 512];
        let mut blob = [0u8; 512];
        assert_eq!(cart.save(&mut scratch, &mut blob), Some(0));
    }

    #[test]
    fn load_error_leaves_state_unchanged() {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        let before = cart.pet.clone();
        let err = cart.load(b"not a save blob at all").unwrap_err();
        assert_eq!(err, LoadError::BadMagic);
        assert_eq!(cart.pet, before);
    }

    // --- Phase 3: the care loop --------------------------------------------------------------

    const START_MS: u64 = 1_700_000_000_000;

    /// A freshly hatched baby, and the `now_ms` at the moment it hatched.
    fn hatched(seed: u64) -> (Cart, u64) {
        let mut cart = Cart::new_uninit();
        cart.reset(START_MS, seed);
        let hatch_ms = START_MS + 300_000;
        cart.update(hatch_ms, 0);
        assert_eq!(cart.pet.stage, Stage::Baby);
        (cart, hatch_ms)
    }

    /// A brief press-and-release of `mask`, advancing `now_ms` by 200ms.
    fn press(cart: &mut Cart, now_ms: &mut u64, mask: u8) {
        *now_ms += 100;
        cart.update(*now_ms, mask);
        *now_ms += 100;
        cart.update(*now_ms, 0);
    }

    /// Opens the menu and moves the cursor to `icon_index`.
    fn open_menu_at(cart: &mut Cart, now_ms: &mut u64, icon_index: u8) {
        press(cart, now_ms, buttons::A);
        for _ in 0..icon_index {
            press(cart, now_ms, buttons::A);
        }
    }

    #[test]
    fn idle_pet_animates_at_real_epoch_timestamps() {
        // docs/art/ANIMATION.md "Idle": idle_a/idle_b toggle every 2 ticks plus a walk, so
        // frames sampled every 2 ticks (500 ms) must not all be identical. Regression test for
        // `time::tick` saturating at u32::MAX for any real-world `now_ms` (~1.7e12 ms / 250 >
        // u32::MAX), which froze every host's animation while small-timestamp tests passed.
        // `START_MS` is a real 2023 epoch timestamp for exactly that reason.
        let (mut cart, mut now) = hatched(42);
        let first = *cart.frame();
        let mut changes = 0;
        let mut differs_from_first = false;
        for _ in 0..12 {
            now += 500;
            let flags = cart.update(now, 0);
            if flags & flags::FRAME_CHANGED != 0 {
                changes += 1;
            }
            if *cart.frame() != first {
                differs_from_first = true;
            }
        }
        assert!(
            differs_from_first,
            "an idle pet never left its first frame in 6 s"
        );
        assert!(
            changes >= 4,
            "an idle pet changed its frame only {changes} times in 6 s"
        );
    }

    #[test]
    fn feed_meal_refused_when_not_hungry_yet() {
        let (mut cart, mut now) = hatched(1);
        open_menu_at(&mut cart, &mut now, generated::icon::FEED as u8);
        press(&mut cart, &mut now, buttons::B); // Feed -> FeedSub{snack: false}
        assert_eq!(cart.ui, Ui::FeedSub { snack: false });
        let hunger_before = cart.pet.hunger.value_at(cart.sim_now);
        press(&mut cart, &mut now, buttons::B); // apply the meal
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Refuse
            }
        ));
        assert_eq!(cart.pet.hunger.value_at(cart.sim_now), hunger_before); // unchanged
    }

    #[test]
    fn feed_meal_applied_once_hungry_enough() {
        let (mut cart, mut now) = hatched(1);
        // hunger_step_secs = 180 for baby; wait past the full_threshold (10).
        now += 1_900_000; // ~31.7 min
        cart.update(now, 0);
        let hunger_before = cart.pet.hunger.value_at(cart.sim_now);
        assert!(hunger_before >= 10);

        open_menu_at(&mut cart, &mut now, generated::icon::FEED as u8);
        press(&mut cart, &mut now, buttons::B); // FeedSub{snack:false}
        press(&mut cart, &mut now, buttons::B); // apply
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Eating { .. }
            }
        ));
        assert!(cart.pet.hunger.value_at(cart.sim_now) < hunger_before);
        assert_eq!(cart.pet.weight, 1 + generated::GAME.meal_weight);
    }

    #[test]
    fn feed_snack_raises_happiness_and_weight() {
        let (mut cart, mut now) = hatched(2);
        let happy_before = cart.pet.happy.value_at(cart.sim_now);
        open_menu_at(&mut cart, &mut now, generated::icon::FEED as u8);
        press(&mut cart, &mut now, buttons::B); // FeedSub{snack:false}
        press(&mut cart, &mut now, buttons::A); // toggle to snack
        assert_eq!(cart.ui, Ui::FeedSub { snack: true });
        press(&mut cart, &mut now, buttons::B); // apply the snack
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Eating { .. }
            }
        ));
        assert!(cart.pet.happy.value_at(cart.sim_now) >= happy_before);
        assert_eq!(cart.pet.weight, 1 + generated::GAME.snack_weight);
        assert_eq!(cart.pet.snacks_since_wake, 1);
    }

    #[test]
    fn clean_refused_without_poop_applied_with_poop() {
        let (mut cart, mut now) = hatched(3);
        open_menu_at(&mut cart, &mut now, generated::icon::CLEAN as u8);
        press(&mut cart, &mut now, buttons::B);
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Refuse
            }
        ));
        // Let the refuse animation finish before navigating the menu again.
        now += 2_000;
        cart.update(now, 0);
        assert_eq!(cart.ui, Ui::Idle);

        cart.pet.poops = 2; // simulate a poop having landed
        open_menu_at(&mut cart, &mut now, generated::icon::CLEAN as u8);
        press(&mut cart, &mut now, buttons::B);
        assert_eq!(cart.pet.poops, 0);
        assert_eq!(cart.ui, Ui::Idle);
    }

    #[test]
    fn medicine_cures_sickness_and_penalizes_discipline_when_not_sick() {
        let (mut cart, mut now) = hatched(4);
        open_menu_at(&mut cart, &mut now, generated::icon::MEDICINE as u8);
        press(&mut cart, &mut now, buttons::B);
        assert_eq!(cart.pet.discipline, 0); // saturating_sub from 0 stays 0, but exercised
        assert!(!cart.pet.sick);
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Discipline
            }
        )); // the penalty is shown, not silent
        now += 3_000;
        cart.update(now, 0); // let that animation end before using the menu again

        cart.pet.sick = true;
        cart.pet.health = 50;
        open_menu_at(&mut cart, &mut now, generated::icon::MEDICINE as u8);
        press(&mut cart, &mut now, buttons::B);
        assert!(!cart.pet.sick);
        assert_eq!(cart.pet.health, 50 + generated::GAME.medicine_health);
        assert_eq!(cart.pet.sick_count, 1);
    }

    #[test]
    fn lights_off_while_awake_inverts_the_room_but_not_the_menu() {
        let (mut cart, mut now) = hatched(13);
        now += 2_000;
        cart.update(now, 0);
        let before = *cart.frame();
        open_menu_at(&mut cart, &mut now, generated::icon::LIGHTS as u8);
        press(&mut cart, &mut now, buttons::B); // lights off
        assert!(cart.pet.lights_off);
        assert_eq!(cart.ui, Ui::Idle);
        let after = *cart.frame();
        // A mostly-dark frame became a mostly-lit one.
        let lit = |f: &[u8; FRAME_LEN]| f.iter().map(|b| b.count_ones()).sum::<u32>();
        assert!(lit(&before) < FRAME_LEN as u32 * 4);
        assert!(lit(&after) > FRAME_LEN as u32 * 4);

        // The menu is drawn normally on top of a dark room.
        press(&mut cart, &mut now, buttons::A);
        assert!(matches!(cart.ui, Ui::Menu { .. }));
        assert!(lit(cart.frame()) < FRAME_LEN as u32 * 4);
    }

    #[test]
    fn discipline_without_a_tantrum_lowers_happiness() {
        let (mut cart, mut now) = hatched(5);
        let happy_before = cart.pet.happy.value_at(cart.sim_now);
        open_menu_at(&mut cart, &mut now, generated::icon::DISCIPLINE as u8);
        press(&mut cart, &mut now, buttons::B);
        assert!(cart.pet.happy.value_at(cart.sim_now) < happy_before);
    }

    #[test]
    fn play_session_scores_five_rounds_and_returns_to_idle() {
        let (mut cart, mut now) = hatched(6);
        open_menu_at(&mut cart, &mut now, generated::icon::PLAY as u8);
        press(&mut cart, &mut now, buttons::B); // start playing
        let seq = match cart.ui {
            Ui::Busy {
                kind: BusyKind::Playing { seq, .. },
            } => seq,
            other => panic!("expected Playing, got {other:?}"),
        };
        // Always guess correctly by reading the hidden sequence back.
        for round in 0..5u8 {
            let actual_right = (seq >> round) & 1 != 0;
            press(
                &mut cart,
                &mut now,
                if actual_right { buttons::B } else { buttons::A },
            );
        }
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Result { .. }
            }
        ));
        // Let the result animation finish.
        now += 3_000;
        cart.update(now, 0);
        assert_eq!(cart.ui, Ui::Idle);
    }

    #[test]
    fn lights_off_during_sleep_pauses_meters_and_wake_gives_a_health_bonus() {
        let (mut cart, mut now) = hatched(7);
        // Force Sleep to fire very soon rather than waiting out the full awake_secs (14h): a
        // wait that long would also run into hunger/poop/sickness timers from the same neglect,
        // which is exactly what tests::neglect_leads_to_starvation exercises on its own. This
        // test wants Sleep/Wake/lights in isolation.
        cart.timers
            .set(EventKind::Sleep, cart.sim_now.saturating_add(5));
        now += 6_000;
        cart.update(now, 0);
        assert!(cart.pet.sleeping);
        assert_ne!(cart.pet.attention & pet::attention::SLEEPY, 0);

        let hunger_at_sleep = cart.pet.hunger.value_at(cart.sim_now);
        open_menu_at(&mut cart, &mut now, generated::icon::LIGHTS as u8);
        press(&mut cart, &mut now, buttons::B);
        assert!(cart.pet.lights_off);
        assert_eq!(cart.pet.attention & pet::attention::SLEEPY, 0);

        // Advance a while; a paused meter must not have moved.
        now += 3_600_000;
        cart.update(now, 0);
        assert_eq!(cart.pet.hunger.value_at(cart.sim_now), hunger_at_sleep);

        // Force Wake to fire soon too (sleep_secs is 36000s) and check the wake bonus.
        cart.timers
            .set(EventKind::Wake, cart.sim_now.saturating_add(5));
        let health_before = cart.pet.health;
        now += 6_000;
        cart.update(now, 0);
        assert!(!cart.pet.sleeping);
        assert_eq!(
            cart.pet.health,
            (health_before + generated::GAME.wake_health_bonus).min(100)
        );
    }

    #[test]
    fn a_missed_attention_call_becomes_a_care_mistake() {
        let (mut cart, mut now) = hatched(8);
        assert_eq!(cart.pet.care_mistakes, 0);
        // Isolate the hunger call from poop/happiness (which would otherwise also cross their
        // own thresholds somewhere in a 15300s gap and add their own, correct, care mistakes).
        cart.timers.clear(EventKind::Poop);
        cart.timers.clear(EventKind::HappyEmpty);
        cart.timers
            .set(EventKind::HungerEmpty, cart.sim_now.saturating_add(5));
        // The resulting CareMiss timer fires care_miss_secs (900s) later if never fed.
        now += 5_000 + 900_000 + 1_000;
        cart.update(now, 0);
        assert_eq!(cart.pet.care_mistakes, 1);
        assert_eq!(cart.pet.attention & pet::attention::HUNGRY, 0); // the call gave up
    }

    #[test]
    fn neglect_leads_to_death_within_a_day() {
        let (mut cart, mut now) = hatched(9);
        // A neglected pet evolves to Child after 1 h, so poop-driven sickness applies and
        // usually kills it (SickDamage every 30 min from 100 health) hours before the Starve
        // timer (hunger at 100 for 12 h) would. Either way: dead, cause recorded, no timers.
        now += 18_000_000 + 43_200_000 + 60_000;
        cart.update(now, 0);
        assert_eq!(cart.state, CartState::Dead);
        assert!(matches!(
            cart.pet.death_cause,
            c if c == pet::DeathCause::Starvation as u8 || c == pet::DeathCause::Sickness as u8
        ));
        assert_eq!(cart.timers.earliest(), None);
    }

    #[test]
    fn a_baby_that_never_evolves_starves() {
        // Pin the starvation path on its own by keeping the pet a baby (no poop sickness):
        // clear the Evolve timer right after hatch.
        let (mut cart, mut now) = hatched(9);
        cart.timers.clear(EventKind::Evolve);
        now += 18_000_000 + 43_200_000 + 60_000;
        cart.update(now, 0);
        assert_eq!(cart.state, CartState::Dead);
        assert_eq!(cart.pet.death_cause, pet::DeathCause::Starvation as u8);
    }

    #[test]
    fn a_dead_pet_ignores_further_input() {
        let (mut cart, mut now) = hatched(10);
        now += 18_000_000 + 43_200_000 + 60_000;
        cart.update(now, 0);
        assert_eq!(cart.state, CartState::Dead);
        press(&mut cart, &mut now, buttons::A);
        assert_eq!(cart.ui, Ui::Idle);
    }
}
