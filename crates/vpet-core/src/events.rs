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

use crate::pet::{attention, DeathCause, Stage};
use crate::time::NEVER;
use crate::timers::EventKind;
use crate::ui::{BusyKind, Ui};
use crate::{actions, assets::generated::GAME, Cart, CartState};

impl Cart {
    pub(crate) fn set_attention(&mut self, bit: u8) {
        if self.pet.attention & bit == 0 {
            self.pet.attention |= bit;
            self.attention_since[attention::index_of(bit)] = self.sim_now;
            self.reschedule_care_miss();
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

        if self.pet.sleeping {
            // No more poops while asleep; fire_wake re-arms this timer. Must still clear it
            // here (it's one-shot) or `advance_to` would refire it forever at this instant.
            self.timers.clear(EventKind::Poop);
        } else {
            let rules = self.stage_rules();
            let gap =
                self.next_rand_range(rules.poop_interval_min_secs, rules.poop_interval_max_secs);
            self.timers.set(EventKind::Poop, now.saturating_add(gap));
        }
    }

    pub(crate) fn fire_tantrum(&mut self) {
        // Only ever scheduled once Evolve actually promotes a pet to Child (Phase 4); the
        // handler is implemented now so nothing changes shape when that scheduling hook lands.
        self.set_attention(attention::TANTRUM);
        let gap = self.next_rand_range(
            GAME.tantrum_interval_min_secs,
            GAME.tantrum_interval_max_secs,
        );
        self.timers
            .set(EventKind::Tantrum, self.sim_now.saturating_add(gap));
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
