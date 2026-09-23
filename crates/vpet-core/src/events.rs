//! Event handlers: what happens when a scheduled timer fires. See docs/STATE_MODEL.md's "Event
//! handlers" table and docs/GAME_DESIGN.md. `Cart::fire` (in `lib.rs`) dispatches here.
//!
//! A few places in docs/GAME_DESIGN.md describe sickness triggers ("a poop has been on screen
//! for 1h", "hunger has reached 100", "the 4th snack since wake", "weight >= 40") that all feed
//! a single `SickOnset` timer slot (docs/STATE_MODEL.md's fixed 15-timer table has room for
//! exactly one pending onset, not one per trigger). Two simplifications follow directly from
//! that fixed budget, called out where they apply below:
//! - Multiple simultaneous triggers can only make `SickOnset` happen *sooner*, never later
//!   (`schedule_sick_onset_no_later_than`): a later trigger never overwrites an earlier pending
//!   one.
//! - An action that resolves one trigger (feeding clears the hunger trigger, cleaning clears
//!   the poop trigger) clears the pending `SickOnset` unconditionally. If a second, unrelated
//!   trigger was also contributing, this cancels its onset too rather than tracking multiple
//!   contributors — a hobby-project simplification, not a determinism issue.

use crate::assets::{generated, Branch};
use crate::pet::{attention, DeathCause, Stage};
use crate::time::NEVER;
use crate::timers::EventKind;
use crate::ui::{BusyKind, Ui};
use crate::{actions, assets::generated::GAME, flags, Cart, CartState};

impl Cart {
    pub(crate) fn set_attention(&mut self, bit: u8) {
        if self.pet.attention & bit == 0 {
            self.pet.attention |= bit;
            self.attention_since[attention::index_of(bit)] = self.sim_now;
            self.reschedule_care_miss();
            // docs/HOST_ABI.md: the pet "calls" by showing `!` and beeping.
            self.pending_flags |= flags::BEEP;
        }
    }

    pub(crate) fn clear_attention(&mut self, bit: u8) {
        if self.pet.attention & bit != 0 {
            self.pet.attention &= !bit;
            self.attention_since[attention::index_of(bit)] = NEVER;
            self.reschedule_care_miss();
        }
    }

    fn reschedule_care_miss(&mut self) {
        let mut earliest = NEVER;
        for (i, &bit) in attention::ALL.iter().enumerate() {
            if self.pet.attention & bit != 0 && self.attention_since[i] != NEVER {
                let deadline = self.attention_since[i].saturating_add(GAME.care_miss_secs);
                if deadline < earliest {
                    earliest = deadline;
                }
            }
        }
        if earliest == NEVER {
            self.timers.clear(EventKind::CareMiss);
        } else {
            self.timers.set(EventKind::CareMiss, earliest);
        }
    }

    /// Only moves `SickOnset` earlier, never later or from unset to a later time than an
    /// existing pending one. See the module doc for why (one timer slot, many triggers).
    pub(crate) fn schedule_sick_onset_no_later_than(&mut self, at: crate::time::Sec) {
        let current = self.timers.get(EventKind::SickOnset);
        if current == NEVER || at < current {
            self.timers.set(EventKind::SickOnset, at);
        }
    }

    pub(crate) fn recompute_hunger_happy_timers(&mut self) {
        let now = self.sim_now;
        self.timers.set(
            EventKind::HungerEmpty,
            self.pet.hunger.reaches(GAME.hungry_threshold),
        );
        self.timers.set(
            EventKind::HappyEmpty,
            self.pet.happy.reaches(GAME.sad_threshold),
        );
        if self.pet.hunger.value_at(now) < 100 {
            self.timers.clear(EventKind::Starve);
        }
    }

    pub(crate) fn die(&mut self, cause: DeathCause) {
        self.state = CartState::Dead;
        self.pet.death_cause = cause as u8;
        self.timers.clear_all();
        self.ui = Ui::Idle;
    }

    pub(crate) fn fire_sleep(&mut self) {
        let now = self.sim_now;
        self.pet.sleeping = true;
        if self.pet.lights_off {
            // Already off at the start of the window: a clean night from the first tick.
            self.pet.hunger.pause(now);
            self.pet.happy.pause(now);
            self.timers.clear(EventKind::HungerEmpty);
            self.timers.clear(EventKind::HappyEmpty);
        } else {
            // The `sleepy` call's own CareMiss timer (docs/GAME_DESIGN.md's 15 min) *is* the
            // "lights off within 15 min" grace period: the `lights` action clears this bit and
            // pauses the meters if the player answers in time; otherwise CareMiss fires and the
            // night was not clean.
            self.set_attention(attention::SLEEPY);
        }
        // Sleep's own timer is one-shot: `fire_wake` is what schedules the *next* Sleep, when
        // it fires. Leaving this timer at its own fire time would spin `advance_to`'s loop
        // forever refiring it at that same instant.
        self.timers.clear(EventKind::Sleep);
        let sleep_secs = self.species_def().schedule.1;
        self.timers
            .set(EventKind::Wake, now.saturating_add(sleep_secs));
    }

    pub(crate) fn fire_wake(&mut self) {
        let now = self.sim_now;
        self.timers.clear(EventKind::Wake); // one-shot; fire_sleep schedules the next Wake
        self.pet.sleeping = false;
        self.pet.lights_off = false;
        if self.pet.hunger.is_paused() {
            self.pet.hunger.resume(now);
        }
        if self.pet.happy.is_paused() {
            self.pet.happy.resume(now);
        }
        self.recompute_hunger_happy_timers();
        self.pet.snacks_since_wake = 0;
        self.clear_attention(attention::SLEEPY);
        if !self.pet.sick {
            self.pet.health = self
                .pet
                .health
                .saturating_add(GAME.wake_health_bonus)
                .min(100);
        }
        let awake_secs = self.species_def().schedule.0;
        self.timers
            .set(EventKind::Sleep, now.saturating_add(awake_secs));

        // Poop stops rescheduling itself while asleep (fire_poop); re-arm it here.
        let rules = self.stage_rules();
        let gap = self.next_rand_range(rules.poop_interval_min_secs, rules.poop_interval_max_secs);
        self.timers.set(EventKind::Poop, now.saturating_add(gap));
    }

    pub(crate) fn fire_poop(&mut self) {
        let now = self.sim_now;
        if self.pet.sleeping {
            // docs/GAME_DESIGN.md: poops appear "while awake". A timer that lands inside the
            // sleep window is dropped; `fire_wake` re-arms it. (One-shot: must clear, or
            // `advance_to` would refire it forever at this instant.)
            self.timers.clear(EventKind::Poop);
            return;
        }
        let was_empty = self.pet.poops == 0;
        self.pet.poops = (self.pet.poops + 1).min(GAME.max_poops);
        if self.pet.poop_since == NEVER {
            self.pet.poop_since = now;
        }
        self.set_attention(attention::POOP);

        if self.pet.stage != Stage::Baby {
            if self.pet.poops >= GAME.max_poops {
                let delay =
                    self.next_rand_range(GAME.sick_delay_min_secs, GAME.sick_delay_max_secs);
                self.schedule_sick_onset_no_later_than(now.saturating_add(delay));
            } else if was_empty {
                // "a poop has been on screen for 1h" trigger, folded into one scheduling step
                // with the usual onset delay (see module doc).
                let delay =
                    self.next_rand_range(GAME.sick_delay_min_secs, GAME.sick_delay_max_secs);
                let at = now
                    .saturating_add(GAME.poop_sick_after_secs)
                    .saturating_add(delay);
                self.schedule_sick_onset_no_later_than(at);
            }
        }

        let rules = self.stage_rules();
        let gap = self.next_rand_range(rules.poop_interval_min_secs, rules.poop_interval_max_secs);
        self.timers.set(EventKind::Poop, now.saturating_add(gap));
    }

    /// Scheduled from the Baby -> Child evolution onward (docs/GAME_DESIGN.md: tantrums are
    /// "Child+ only", every 4 to 8 h "awake"). A firing that lands inside the sleep window is
    /// simply pushed out by another interval rather than waking a false call the player
    /// couldn't answer (Discipline is refused while asleep).
    pub(crate) fn fire_tantrum(&mut self) {
        if !self.pet.sleeping {
            self.set_attention(attention::TANTRUM);
        }
        let gap = self.next_rand_range(
            GAME.tantrum_interval_min_secs,
            GAME.tantrum_interval_max_secs,
        );
        self.timers
            .set(EventKind::Tantrum, self.sim_now.saturating_add(gap));
    }

    /// docs/STATE_MODEL.md's Evolve row and docs/GAME_DESIGN.md "Evolution". One-shot: the
    /// Baby -> Child transition re-arms it for the Child -> Adult one; the adult transition
    /// arms `OldAge` instead.
    pub(crate) fn fire_evolve(&mut self) {
        let now = self.sim_now;
        self.timers.clear(EventKind::Evolve);
        match self.pet.stage {
            Stage::Baby => {
                let species = self.pet.species;
                self.enter_stage(species, Stage::Child);
                let rules = self.stage_rules();
                self.timers
                    .set(EventKind::Evolve, now.saturating_add(rules.stage_secs));
                let gap = self.next_rand_range(
                    GAME.tantrum_interval_min_secs,
                    GAME.tantrum_interval_max_secs,
                );
                self.timers.set(EventKind::Tantrum, now.saturating_add(gap));
            }
            Stage::Child => {
                let (species, stage) = self.choose_adult_branch();
                self.enter_stage(species, stage);
                let rules = self.stage_rules();
                // docs/GAME_DESIGN.md "Death": uniform in the stage's lifespan window, minus
                // 6 h per care mistake, floored. Exactly one RNG draw, here.
                let span = self.next_rand_range(rules.lifespan_min_secs, rules.lifespan_max_secs);
                let penalty = (self.pet.care_mistakes as u32)
                    .saturating_mul(GAME.lifespan_penalty_per_mistake_secs);
                let span = span.saturating_sub(penalty).max(GAME.lifespan_floor_secs);
                self.timers.set(EventKind::OldAge, now.saturating_add(span));
            }
            // Egg never has this timer; adults end by OldAge, not Evolve.
            Stage::Egg | Stage::Adult | Stage::AdultAlt => {}
        }
    }

    pub(crate) fn fire_old_age(&mut self) {
        self.die(DeathCause::OldAge);
    }

    /// The stage change itself: species/stage/stage_since, meters re-anchored to the new
    /// stage's step values *preserving their current values and paused-ness* (an evolution
    /// during a clean night must not silently resume the meters), thresholds recomputed, and
    /// the Evolving animation unless asleep (the Sleeping clip owns the screen then;
    /// docs/art/ANIMATION.md).
    fn enter_stage(&mut self, species: u8, stage: Stage) {
        let now = self.sim_now;
        self.pet.species = species;
        self.pet.stage = stage;
        self.pet.stage_since = now;
        let rules = self.stage_rules();

        let hunger_paused = self.pet.hunger.is_paused();
        let h = self.pet.hunger.value_at(now);
        self.pet
            .hunger
            .reanchor_with_step(now, h, rules.hunger_step_secs);
        if hunger_paused {
            self.pet.hunger.pause(now);
        }
        let happy_paused = self.pet.happy.is_paused();
        let p = self.pet.happy.value_at(now);
        self.pet
            .happy
            .reanchor_with_step(now, p, rules.happy_step_secs);
        if happy_paused {
            self.pet.happy.pause(now);
        }
        self.recompute_hunger_happy_timers();

        if !self.pet.sleeping {
            self.ui = Ui::Busy {
                kind: BusyKind::Evolving,
            };
            // 12 ticks (docs/art/ANIMATION.md "Evolving") at ANIM_HZ = 4.
            self.timers.set(EventKind::UiBusyEnd, now.saturating_add(3));
        }
        self.pending_flags |= flags::BEEP;
    }

    fn branch_matches(&self, b: &Branch) -> bool {
        b.max_care_mistakes
            .is_none_or(|m| self.pet.care_mistakes <= m)
            && b.min_discipline.is_none_or(|d| self.pet.discipline >= d)
            && b.max_weight.is_none_or(|w| self.pet.weight <= w)
    }

    /// docs/GAME_DESIGN.md "Evolution": branches are evaluated in order and the first whose
    /// conditions all hold wins. `weight > 0` marks a randomised branch: the first match's
    /// weight defines the set, every later matching branch with that same weight joins it, and
    /// the RNG picks uniformly — only when the set has more than one member (docs/DETERMINISM.md
    /// rule 6: no draw when nothing is random). `to_species` jumps lineages if that id is in
    /// this build's registry; a target without an alternate adult falls back to `Adult`.
    fn choose_adult_branch(&mut self) -> (u8, Stage) {
        let species = self.species_def();
        let branches = species.child.evolve;

        let first = branches.iter().position(|b| self.branch_matches(b));
        let Some(first) = first else {
            return (self.pet.species, Stage::Adult);
        };
        let weight = branches[first].weight;
        let chosen: &Branch = if weight == 0 {
            &branches[first]
        } else {
            let candidates = branches
                .iter()
                .skip(first)
                .filter(|b| b.weight == weight && self.branch_matches(b))
                .count() as u32;
            let pick = if candidates > 1 {
                self.next_rand_range(0, candidates - 1) as usize
            } else {
                0
            };
            branches
                .iter()
                .skip(first)
                .filter(|b| b.weight == weight && self.branch_matches(b))
                .nth(pick)
                .unwrap_or(&branches[first])
        };

        let target_species = match chosen.to_species {
            Some(id) if generated::SPECIES.iter().any(|s| s.id == id) => id,
            _ => self.pet.species,
        };
        let target = generated::SPECIES
            .iter()
            .find(|s| s.id == target_species)
            .unwrap_or(species);
        let stage = match chosen.to {
            Stage::AdultAlt if target.adult_alt.is_some() => Stage::AdultAlt,
            _ => Stage::Adult,
        };
        (target_species, stage)
    }

    pub(crate) fn fire_sick_onset(&mut self) {
        self.timers.clear(EventKind::SickOnset); // one-shot; medicine/SickDamage take it from here
        self.pet.sick = true;
        self.set_attention(attention::SICK);
        self.timers.set(
            EventKind::SickDamage,
            self.sim_now.saturating_add(GAME.sick_damage_secs),
        );
    }

    pub(crate) fn fire_sick_damage(&mut self) {
        self.pet.health = self.pet.health.saturating_sub(GAME.sick_damage_hp);
        if self.pet.health == 0 {
            self.die(DeathCause::Sickness);
        } else {
            self.timers.set(
                EventKind::SickDamage,
                self.sim_now.saturating_add(GAME.sick_damage_secs),
            );
        }
    }

    /// One-shot: fires once when hunger crosses the threshold, then clears its own timer.
    /// Re-armed only by an action that moves the meter back down and recomputes it
    /// (`feed_meal`) — nothing else would naturally cross the threshold again, since hunger
    /// only rises and clamps at 100. Leaving the timer set to its own fire time here would spin
    /// `advance_to`'s loop forever refiring it at that same instant.
    pub(crate) fn fire_hunger_empty(&mut self) {
        self.timers.clear(EventKind::HungerEmpty);
        self.set_attention(attention::HUNGRY);
        let reaches_100 = self.pet.hunger.reaches(100);
        if reaches_100 != NEVER {
            self.timers.set(
                EventKind::Starve,
                reaches_100.saturating_add(GAME.starve_secs),
            );
            if self.pet.stage != Stage::Baby {
                let delay =
                    self.next_rand_range(GAME.sick_delay_min_secs, GAME.sick_delay_max_secs);
                self.schedule_sick_onset_no_later_than(reaches_100.saturating_add(delay));
            }
        }
    }

    /// One-shot, same reasoning as `fire_hunger_empty`.
    pub(crate) fn fire_happy_empty(&mut self) {
        self.timers.clear(EventKind::HappyEmpty);
        self.set_attention(attention::SAD);
    }

    pub(crate) fn fire_care_miss(&mut self) {
        let now = self.sim_now;
        let mut any_cleared = false;
        for (i, &bit) in attention::ALL.iter().enumerate() {
            if self.pet.attention & bit != 0 {
                let since = self.attention_since[i];
                if since != NEVER && now.saturating_sub(since) >= GAME.care_miss_secs {
                    self.pet.attention &= !bit;
                    self.attention_since[i] = NEVER;
                    any_cleared = true;
                }
            }
        }
        if any_cleared {
            self.pet.care_mistakes = self.pet.care_mistakes.saturating_add(1);
        }
        self.reschedule_care_miss();
    }

    pub(crate) fn fire_starve(&mut self) {
        self.die(DeathCause::Starvation);
    }

    pub(crate) fn fire_ui_busy_end(&mut self) {
        self.timers.clear(EventKind::UiBusyEnd);
        if let Ui::Busy {
            kind: BusyKind::Playing { correct, .. },
        } = self.ui
        {
            actions::finish_play(self, correct);
        } else {
            self.ui = Ui::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::generated::SPECIES;
    use crate::assets::SpeciesDef;
    use crate::buttons;
    use crate::time::Sec;

    const START_MS: u64 = 1_700_000_000_000;

    fn hatched(seed: u64) -> (Cart, u64) {
        let mut cart = Cart::new_uninit();
        cart.reset(START_MS, seed);
        let hatch_ms = START_MS + 300_000;
        cart.update(hatch_ms, 0);
        assert_eq!(cart.pet.stage, Stage::Baby);
        (cart, hatch_ms)
    }

    /// A child, straight after its (animated) evolution finished.
    fn child(seed: u64) -> (Cart, u64) {
        let (mut cart, mut now) = hatched(seed);
        now += 3_600_000 + 5_000; // baby stage_secs, plus the Evolving busy end
        cart.update(now, 0);
        assert_eq!(cart.pet.stage, Stage::Child);
        assert_eq!(cart.ui, Ui::Idle);
        (cart, now)
    }

    /// Forces the pending `Evolve` to fire 1 s from now (nothing else is due that soon).
    fn evolve_in_one_second(cart: &mut Cart, now: &mut u64) {
        let at = cart.sim_now + 1;
        cart.timers.set(EventKind::Evolve, at);
        *now += 1_000;
        cart.update(*now, 0);
    }

    #[test]
    fn baby_evolves_to_child_at_stage_secs_and_keeps_meter_values() {
        let (mut cart, hatch_ms) = hatched(1);
        let just_before = hatch_ms + 3_600_000 - 1_000;
        cart.update(just_before, 0);
        assert_eq!(cart.pet.stage, Stage::Baby);
        // The baby meters as they were; evaluated *at the evolve second* below, so a point
        // that legitimately ticks over in that last second doesn't read as a re-anchor bug.
        let hunger_before = cart.pet.hunger;
        let happy_before = cart.pet.happy;

        cart.update(hatch_ms + 3_600_000, 0);
        assert_eq!(cart.pet.stage, Stage::Child);
        assert_eq!(cart.pet.stage_since, cart.sim_now);
        let t = cart.sim_now;
        assert_eq!(cart.pet.hunger.value_at(t), hunger_before.value_at(t));
        assert_eq!(cart.pet.happy.value_at(t), happy_before.value_at(t));
        assert_eq!(
            cart.pet.hunger.step_secs(),
            SPECIES[0].child.rules.hunger_step_secs
        );
        assert!(matches!(
            cart.ui,
            Ui::Busy {
                kind: BusyKind::Evolving
            }
        ));
        assert_ne!(cart.timers.get(EventKind::UiBusyEnd), NEVER);
        assert_ne!(cart.timers.get(EventKind::Evolve), NEVER); // re-armed for Child -> Adult
        assert_ne!(cart.timers.get(EventKind::Tantrum), NEVER); // tantrums start at Child
    }

    /// lalafu's child has a three-way split (docs/GAME_DESIGN.md "Evolution", first match in
    /// file order): fed and disciplined -> ninjifu, fed but undisciplined -> lalafu's own adult
    /// (the ghost), neglected -> charamofu (docs/art/LINEAGE.md). All land on `Stage::Adult`;
    /// `Stage::AdultAlt` still exists for a single species that wants a bad-care alternate
    /// adult of its own, lalafu just doesn't use it.
    fn species_by_id(id: u8) -> &'static SpeciesDef {
        SPECIES.iter().find(|s| s.id == id).unwrap()
    }

    #[test]
    fn cared_for_child_becomes_ninjifu_and_gets_an_old_age_timer() {
        let (mut cart, mut now) = child(2);
        cart.pet.care_mistakes = 4;
        cart.pet.discipline = 40;
        evolve_in_one_second(&mut cart, &mut now);
        assert_eq!(cart.pet.stage, Stage::Adult);
        assert_eq!(cart.pet.species, 3); // ninjifu (registry.toml)
        assert_eq!(cart.timers.get(EventKind::Evolve), NEVER);

        let rules = species_by_id(3).adult.rules;
        let penalty = 4 * GAME.lifespan_penalty_per_mistake_secs;
        let at = cart.timers.get(EventKind::OldAge);
        assert!(at >= cart.sim_now + rules.lifespan_min_secs - penalty);
        assert!(at <= cart.sim_now + rules.lifespan_max_secs - penalty);
        assert_eq!(cart.pet.hunger.step_secs(), rules.hunger_step_secs);
    }

    #[test]
    fn neglected_child_becomes_charamofu() {
        let (mut cart, mut now) = child(3);
        cart.pet.care_mistakes = 5; // one over the ninjifu branch's max
        cart.pet.discipline = 100;
        evolve_in_one_second(&mut cart, &mut now);
        assert_eq!(cart.pet.stage, Stage::Adult);
        assert_eq!(cart.pet.species, 2); // charamofu (registry.toml)
        let rules = species_by_id(2).adult.rules;
        assert_eq!(cart.pet.hunger.step_secs(), rules.hunger_step_secs);
    }

    #[test]
    fn fed_but_undisciplined_child_becomes_the_ghost_adult() {
        let (mut cart, mut now) = child(4);
        cart.pet.care_mistakes = 0;
        cart.pet.discipline = 39; // one under the ninjifu branch's min
        evolve_in_one_second(&mut cart, &mut now);
        assert_eq!(cart.pet.stage, Stage::Adult);
        assert_eq!(cart.pet.species, 1); // lalafu's own adult, the ghost (registry.toml)
        let rules = species_by_id(1).adult.rules;
        assert_eq!(cart.pet.hunger.step_secs(), rules.hunger_step_secs);
    }

    #[test]
    fn lifespan_penalty_is_floored() {
        let (mut cart, mut now) = child(5);
        cart.pet.care_mistakes = 200; // 200 * 6 h dwarfs any lifespan window
        evolve_in_one_second(&mut cart, &mut now);
        assert_eq!(
            cart.timers.get(EventKind::OldAge),
            cart.sim_now + GAME.lifespan_floor_secs
        );
    }

    #[test]
    fn old_age_kills_and_clears_every_timer() {
        let (mut cart, mut now) = child(6);
        evolve_in_one_second(&mut cart, &mut now);
        assert!(matches!(cart.pet.stage, Stage::Adult | Stage::AdultAlt));
        cart.timers.set(EventKind::OldAge, cart.sim_now + 1);
        now += 1_000;
        cart.update(now, 0);
        assert_eq!(cart.state, CartState::Dead);
        assert_eq!(cart.pet.death_cause, DeathCause::OldAge as u8);
        assert_eq!(cart.timers.earliest(), None);
    }

    #[test]
    fn evolving_while_asleep_keeps_meters_paused_and_skips_the_animation() {
        let (mut cart, mut now) = hatched(7);
        // Fake a clean night in progress.
        let t: Sec = cart.sim_now;
        cart.pet.sleeping = true;
        cart.pet.lights_off = true;
        cart.pet.hunger.pause(t);
        cart.pet.happy.pause(t);
        cart.timers.clear(EventKind::HungerEmpty);
        cart.timers.clear(EventKind::HappyEmpty);

        evolve_in_one_second(&mut cart, &mut now);
        assert_eq!(cart.pet.stage, Stage::Child);
        assert!(cart.pet.hunger.is_paused());
        assert!(cart.pet.happy.is_paused());
        assert_eq!(cart.ui, Ui::Idle);
        assert_eq!(cart.timers.get(EventKind::UiBusyEnd), NEVER);
    }

    #[test]
    fn a_tantrum_due_while_asleep_is_deferred_not_raised() {
        let (mut cart, mut now) = child(8);
        cart.pet.sleeping = true;
        cart.timers.set(EventKind::Tantrum, cart.sim_now + 1);
        now += 1_000;
        cart.update(now, 0);
        assert_eq!(cart.pet.attention & attention::TANTRUM, 0);
        assert!(cart.timers.get(EventKind::Tantrum) > cart.sim_now);
    }

    #[test]
    fn a_poop_due_while_asleep_is_dropped_until_wake() {
        let (mut cart, mut now) = child(12);
        cart.pet.sleeping = true;
        cart.timers.set(EventKind::Poop, cart.sim_now + 1);
        now += 1_000;
        cart.update(now, 0);
        assert_eq!(cart.pet.poops, 0);
        assert_eq!(cart.timers.get(EventKind::Poop), NEVER); // fire_wake re-arms it
    }

    fn dead_by_neglect() -> (Cart, u64) {
        let (mut cart, mut now) = hatched(9);
        now += 90_000_000; // 25 h: well past starvation (tests/golden/neglect_24h.vlog)
        cart.update(now, 0);
        assert_eq!(cart.state, CartState::Dead);
        (cart, now)
    }

    #[test]
    fn holding_a_and_c_for_a_second_on_the_tombstone_restarts() {
        let (mut cart, mut now) = dead_by_neglect();
        let rng_before = cart.rng;
        now += 1_000;
        cart.update(now, buttons::A | buttons::C);
        assert_eq!(cart.state, CartState::Dead); // hold just started
        now += 1_000;
        let out = cart.update(now, buttons::A | buttons::C);
        assert_eq!(cart.state, CartState::Alive);
        assert_eq!(cart.pet.stage, Stage::Egg);
        assert_ne!(cart.rng, rng_before); // reseeded from a draw on the old stream
        assert_ne!(out & flags::SAVE_NEEDED, 0);
        assert_ne!(out & flags::FRAME_CHANGED, 0); // tombstone -> egg
                                                   // The still-held buttons are not a fresh press on the new egg.
        now += 100;
        cart.update(now, buttons::A | buttons::C);
        assert_eq!(cart.ui, Ui::Idle);
    }

    #[test]
    fn releasing_before_a_second_does_not_restart() {
        let (mut cart, mut now) = dead_by_neglect();
        now += 1_000;
        cart.update(now, buttons::A | buttons::C);
        now += 500;
        cart.update(now, 0);
        now += 1_000;
        cart.update(now, buttons::A | buttons::C); // a new hold, only 0 ms old
        assert_eq!(cart.state, CartState::Dead);
        now += 999;
        cart.update(now, buttons::A); // wrong mask: hold abandoned
        now += 1;
        cart.update(now, buttons::A | buttons::C);
        assert_eq!(cart.state, CartState::Dead);
    }

    #[test]
    fn restart_is_deterministic_for_the_same_history() {
        let run = || {
            let (mut cart, mut now) = dead_by_neglect();
            now += 1_000;
            cart.update(now, buttons::A | buttons::C);
            now += 1_000;
            cart.update(now, buttons::A | buttons::C);
            now += 400_000;
            cart.update(now, 0); // into the new egg's life a little
            let mut scratch = [0u8; 512];
            let mut out = [0u8; 512];
            let len = cart.save(&mut scratch, &mut out).unwrap();
            out[..len].to_vec()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn flags_beep_on_actions_and_attention() {
        let (mut cart, mut now) = hatched(10);
        // A refused meal (not hungry yet): BEEP, but nothing to save.
        let press = |cart: &mut Cart, now: &mut u64, mask: u8| -> u32 {
            *now += 100;
            let f = cart.update(*now, mask);
            *now += 100;
            cart.update(*now, 0);
            f
        };
        press(&mut cart, &mut now, buttons::A); // menu at Feed
        press(&mut cart, &mut now, buttons::B); // FeedSub
        let refused = press(&mut cart, &mut now, buttons::B);
        assert_ne!(refused & flags::BEEP, 0);
        assert_eq!(refused & flags::SAVE_NEEDED, 0);

        // Wait until hungry enough, then an applied meal: BEEP and SAVE_NEEDED.
        now += 2_000_000;
        cart.update(now, 0);
        now += 2_000; // let the refuse animation end
        cart.update(now, 0);
        press(&mut cart, &mut now, buttons::A);
        press(&mut cart, &mut now, buttons::B);
        let applied = press(&mut cart, &mut now, buttons::B);
        assert_ne!(applied & flags::BEEP, 0);
        assert_ne!(applied & flags::SAVE_NEEDED, 0);

        // A call: the update that raises the bit beeps, and ATTENTION stays up while pending.
        let (mut cart, mut now) = hatched(11);
        // Hunger reaches 80 at 21600 s: 20 points over the 3600 s baby stage (step 180),
        // then 60 more as a child (step 300).
        now += 21_600_000 + 1_000;
        let out = cart.update(now, 0);
        assert_ne!(cart.pet.attention & attention::HUNGRY, 0);
        assert_ne!(out & flags::BEEP, 0);
        assert_ne!(out & flags::ATTENTION, 0);
        now += 1_000;
        let later = cart.update(now, 0);
        assert_ne!(later & flags::ATTENTION, 0);
        assert_eq!(later & flags::BEEP, 0);
    }
}
