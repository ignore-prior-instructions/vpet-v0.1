//! vpet-core: the cartridge. Owns the simulation, the rules, the compiled-in assets, and the
//! 32x16 one-bit framebuffer. No allocator, no clock, no I/O — see docs/ARCHITECTURE.md and
//! docs/DETERMINISM.md. `std` is enabled only under `cfg(test)` (property tests need it) and
//! will later be enabled under the `dev-overrides` feature (docs/CONTENT.md "Hot preview").
#![cfg_attr(not(test), no_std)]
#![deny(clippy::float_arithmetic)]

pub mod anim;
pub mod assets;
pub mod meter;
pub mod pet;
pub mod render;
pub mod rng;
pub mod save;
pub mod time;
pub mod timers;
pub mod ui;

use anim::{AnimState, ClipId};
use assets::{generated, SpeciesDef, StageSet};
use meter::Meter;
use pet::{Pet, Stage};
use render::Fb;
use rng::Rng;
use save::v1::SaveV1;
use save::LoadError;
use time::{Sec, NEVER};
use timers::{EventKind, Timers};
use ui::Ui;

/// Bumps on any change to exports, buffer sizes, frame format, flag bits, error codes, or
/// `Inspect` (docs/HOST_ABI.md "Versioning").
pub const ABI_VERSION: u32 = 1;

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
    prev_frame: [u8; 64],
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
            prev_frame: [0u8; 64],
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
        let tick = time::tick(now_ms);
        self.anim
            .on_state_change(ClipId::Egg, tick, now, self.rng[0]);
        self.prev_frame = *self.render_frame(tick).as_bytes();
    }

    /// Advance to `now_ms`, apply input, render. Returns flags (`flags::*`).
    pub fn update(&mut self, now_ms: u64, buttons: u8) -> u32 {
        let target = time::ms_to_sec(now_ms);
        let event_fired = self.advance_to(target);

        let rising = buttons & !self.prev_buttons;
        self.prev_buttons = buttons;
        self.apply_input(rising);

        let tick = time::tick(now_ms);
        self.retarget_clip(tick);

        let fb = self.render_frame(tick);
        let frame_changed = fb.as_bytes() != &self.prev_frame;
        self.prev_frame = *fb.as_bytes();

        let mut out = 0u32;
        if frame_changed {
            out |= flags::FRAME_CHANGED;
        }
        if event_fired {
            out |= flags::SAVE_NEEDED;
        }
        out
    }

    pub fn frame(&self) -> &[u8; 64] {
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
            EventKind::UiBusyEnd | EventKind::MenuTimeout => {
                self.ui = Ui::Idle;
                self.timers.clear(kind);
            }
            // Phase 3/4 implement these handlers (evolution, sleep, poop, sickness, care
            // misses, starvation, old age). For now they just clear so `advance_to`'s loop
            // always terminates; they were scheduled by `fire_hatch` below purely so
            // `inspect().next_event_at` is meaningful before those systems exist.
            EventKind::Wake
            | EventKind::Sleep
            | EventKind::Evolve
            | EventKind::OldAge
            | EventKind::Poop
            | EventKind::Tantrum
            | EventKind::SickOnset
            | EventKind::HungerEmpty
            | EventKind::HappyEmpty
            | EventKind::CareMiss
            | EventKind::SickDamage
            | EventKind::Starve => {
                self.timers.clear(kind);
            }
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
                } else if rising & buttons::C != 0 {
                    self.ui = Ui::Idle;
                    self.timers.clear(EventKind::MenuTimeout);
                }
                // B (select an action) is Phase 3: no actions exist yet to select.
            }
            _ => {} // other Ui variants are Phase 3+
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
        if self.state != CartState::Alive {
            return ClipId::Idle; // Dead/Uninit: Phase 3 gives these their own clips
        }
        if self.pet.stage == Stage::Egg {
            return ClipId::Egg;
        }
        match self.ui {
            Ui::Menu { .. } => ClipId::Menu,
            _ => ClipId::Idle,
        }
    }

    fn retarget_clip(&mut self, tick: u32) {
        let desired = self.desired_clip();
        if self.anim.clip != desired {
            let rng0 = self.rng[0];
            self.anim.on_state_change(desired, tick, self.sim_now, rng0);
        }
    }

    fn render_frame(&self, tick: u32) -> Fb {
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
            return render::compose::render_egg(egg_sprites, remaining, tick, &self.anim);
        }
        match self.ui {
            Ui::Menu { cursor } => render::compose::render_menu(&generated::ICONS, cursor),
            _ => {
                let species = self.species_def();
                let stage_set = self.stage_set(species);
                render::compose::render_idle(stage_set, tick, &self.anim)
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
        match self.pet.stage {
            Stage::Egg | Stage::Baby => &species.baby,
            Stage::Child => &species.child,
            Stage::Adult => &species.adult,
            Stage::AdultAlt => species.adult_alt.as_ref().unwrap_or(&species.adult),
        }
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
}
