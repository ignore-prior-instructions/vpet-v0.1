//! Actions: what a menu selection (B) does. See docs/GAME_DESIGN.md's action table and
//! docs/STATE_MODEL.md's `feed_meal` shape. Every action is applied at `sim_now` (after
//! `advance_to`), returns [`Outcome`], and recomputes the threshold timers of any meter it
//! touches.

use crate::assets::generated::{icon, GAME};
use crate::pet::{attention, Stage};
use crate::timers::EventKind;
use crate::ui::{BusyKind, Ui};
use crate::Cart;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Applied,
    Refused,
}

/// Ticks (`ANIM_HZ` = 4) rounded up to the next whole tick, matching STATE_MODEL.md's
/// `feed_meal` example ("18 anim ticks ~ 4.5 s, rounded up").
fn busy_ticks_to_secs(ticks: u32) -> u32 {
    ticks.div_ceil(crate::time::ANIM_HZ)
}

fn refuse(cart: &mut Cart) -> Outcome {
    cart.ui = Ui::Busy {
        kind: BusyKind::Refuse,
    };
    cart.timers.set(
        EventKind::UiBusyEnd,
        cart.sim_now.saturating_add(busy_ticks_to_secs(4)),
    );
    Outcome::Refused
}

fn maybe_schedule_overweight_sickness(cart: &mut Cart) {
    if cart.pet.stage != Stage::Baby && cart.pet.weight >= GAME.overweight_threshold {
        let delay = cart.next_rand_range(GAME.sick_delay_min_secs, GAME.sick_delay_max_secs);
        let at = cart.sim_now.saturating_add(delay);
        cart.schedule_sick_onset_no_later_than(at);
    }
}

/// Selects the icon under the menu cursor (B while `Ui::Menu`). `Feed` opens a submenu instead
/// of applying directly; everything else applies immediately.
pub fn select_menu_icon(cart: &mut Cart, cursor: u8) {
    match cursor as usize {
        icon::FEED => {
            cart.ui = Ui::FeedSub { snack: false };
        }
        icon::LIGHTS => {
            lights(cart);
        }
        icon::PLAY => {
            play_select(cart);
        }
        icon::MEDICINE => {
            medicine(cart);
        }
        icon::CLEAN => {
            clean(cart);
        }
        icon::STATUS => {
            cart.ui = Ui::Status { page: 0 };
        }
        icon::DISCIPLINE => {
            discipline(cart);
        }
        icon::BATTLE => {
            // Battle is Phase 6; refuse gracefully rather than doing nothing.
            refuse(cart);
        }
        _ => {}
    }
}

pub fn feed_meal(cart: &mut Cart) -> Outcome {
    let now = cart.sim_now;
    let h = cart.pet.hunger.value_at(now);
    if cart.pet.sleeping || h < GAME.full_threshold {
        return refuse(cart);
    }
    cart.pet
        .hunger
        .reanchor(now, h.saturating_sub(GAME.meal_hunger));
    cart.pet.weight = cart.pet.weight.saturating_add(GAME.meal_weight);
    cart.clear_attention(attention::HUNGRY);
    cart.timers.set(
        EventKind::HungerEmpty,
        cart.pet.hunger.reaches(GAME.hungry_threshold),
    );
    cart.timers.clear(EventKind::Starve);
    maybe_schedule_overweight_sickness(cart);

    cart.ui = Ui::Busy {
        kind: BusyKind::Eating { snack: false },
    };
    cart.timers.set(
        EventKind::UiBusyEnd,
        now.saturating_add(busy_ticks_to_secs(18)),
    );
    Outcome::Applied
}

pub fn feed_snack(cart: &mut Cart) -> Outcome {
    let now = cart.sim_now;
    if cart.pet.sleeping {
        return refuse(cart);
    }
    let h = cart.pet.happy.value_at(now);
    cart.pet
        .happy
        .reanchor(now, h.saturating_add(GAME.snack_happy).min(100));
    cart.pet.weight = cart.pet.weight.saturating_add(GAME.snack_weight);
    cart.clear_attention(attention::SAD);
    cart.timers.set(
        EventKind::HappyEmpty,
        cart.pet.happy.reaches(GAME.sad_threshold),
    );

    cart.pet.snacks_since_wake = cart.pet.snacks_since_wake.saturating_add(1);
    if cart.pet.stage != Stage::Baby && cart.pet.snacks_since_wake >= GAME.snacks_before_sick {
        let delay = cart.next_rand_range(GAME.sick_delay_min_secs, GAME.sick_delay_max_secs);
        let at = now.saturating_add(delay);
        cart.schedule_sick_onset_no_later_than(at);
    }
    maybe_schedule_overweight_sickness(cart);

    cart.ui = Ui::Busy {
        kind: BusyKind::Eating { snack: true },
    };
    cart.timers.set(
        EventKind::UiBusyEnd,
        now.saturating_add(busy_ticks_to_secs(18)),
    );
    Outcome::Applied
}

pub fn clean(cart: &mut Cart) -> Outcome {
    if cart.pet.poops == 0 {
        return refuse(cart);
    }
    cart.pet.poops = 0;
    cart.pet.poop_since = crate::time::NEVER;
    cart.clear_attention(attention::POOP);
    // The poop-driven SickOnset trigger is resolved; see events.rs's module doc on why this
    // clears the timer unconditionally rather than tracking multiple contributors.
    cart.timers.clear(EventKind::SickOnset);
    // No dedicated clip for Clean (docs/art/ANIMATION.md's clip table has no "Clean" row): the
    // menu just closes, like Lights and (the non-sick branch of) Medicine.
    cart.ui = Ui::Idle;
    Outcome::Applied
}

pub fn medicine(cart: &mut Cart) -> Outcome {
    if cart.pet.sick {
        cart.pet.sick = false;
        cart.pet.health = cart
            .pet
            .health
            .saturating_add(GAME.medicine_health)
            .min(100);
        cart.pet.sick_count = cart.pet.sick_count.saturating_add(1);
        cart.clear_attention(attention::SICK);
        cart.timers.clear(EventKind::SickDamage);
    } else {
        cart.pet.discipline = cart
            .pet
            .discipline
            .saturating_sub(GAME.medicine_discipline_penalty);
    }
    cart.ui = Ui::Idle;
    Outcome::Applied
}

/// Never refused. Toggles `lights_off`; if that starts or ends a clean night mid-sleep, pauses
/// or resumes the meters to match (see events.rs's `fire_sleep`).
pub fn lights(cart: &mut Cart) -> Outcome {
    let now = cart.sim_now;
    cart.pet.lights_off = !cart.pet.lights_off;
    if cart.pet.sleeping {
        if cart.pet.lights_off {
            cart.pet.hunger.pause(now);
            cart.pet.happy.pause(now);
            cart.timers.clear(EventKind::HungerEmpty);
            cart.timers.clear(EventKind::HappyEmpty);
            cart.clear_attention(attention::SLEEPY);
        } else {
            if cart.pet.hunger.is_paused() {
                cart.pet.hunger.resume(now);
            }
            if cart.pet.happy.is_paused() {
                cart.pet.happy.resume(now);
            }
            cart.recompute_hunger_happy_timers();
        }
    }
    cart.ui = Ui::Idle;
    Outcome::Applied
}

pub fn discipline(cart: &mut Cart) -> Outcome {
    if cart.pet.sleeping {
        return refuse(cart);
    }
    let now = cart.sim_now;
    if cart.pet.attention & attention::TANTRUM != 0 {
        cart.pet.discipline = cart
            .pet
            .discipline
            .saturating_add(GAME.discipline_gain)
            .min(100);
        cart.clear_attention(attention::TANTRUM);
    } else {
        let h = cart.pet.happy.value_at(now);
        cart.pet
            .happy
            .reanchor(now, h.saturating_sub(GAME.discipline_wrong_happy_penalty));
        cart.timers.set(
            EventKind::HappyEmpty,
            cart.pet.happy.reaches(GAME.sad_threshold),
        );
    }
    cart.ui = Ui::Busy {
        kind: BusyKind::Discipline,
    };
    cart.timers.set(
        EventKind::UiBusyEnd,
        now.saturating_add(busy_ticks_to_secs(6)),
    );
    Outcome::Applied
}

pub fn play_select(cart: &mut Cart) -> Outcome {
    if cart.pet.sleeping || cart.pet.sick {
        return refuse(cart);
    }
    // 5 rounds, one random direction bit each (docs/GAME_DESIGN.md: "drawn from the RNG when
    // Play is selected, so a replay is deterministic").
    let seq = cart.next_rand_range(0, 31) as u8;
    cart.ui = Ui::Busy {
        kind: BusyKind::Playing {
            seq,
            round: 0,
            correct: 0,
        },
    };
    cart.timers
        .set(EventKind::UiBusyEnd, cart.sim_now.saturating_add(30));
    Outcome::Applied
}

/// `guessed_right`: the player's guess for this round (A = left = false, B = right = true).
/// Called from `apply_input` while `Ui::Busy { kind: Playing }`.
pub fn play_guess(cart: &mut Cart, guessed_right: bool) {
    let (seq, round, correct) = match cart.ui {
        Ui::Busy {
            kind:
                BusyKind::Playing {
                    seq,
                    round,
                    correct,
                },
        } => (seq, round, correct),
        _ => return,
    };
    let actual_right = (seq >> round) & 1 != 0;
    let new_correct = if guessed_right == actual_right {
        correct + 1
    } else {
        correct
    };
    let new_round = round + 1;
    if new_round >= 5 {
        finish_play(cart, new_correct);
    } else {
        cart.ui = Ui::Busy {
            kind: BusyKind::Playing {
                seq,
                round: new_round,
                correct: new_correct,
            },
        };
        cart.timers
            .set(EventKind::UiBusyEnd, cart.sim_now.saturating_add(30));
    }
}

/// Scores the finished (or timed-out) play session. `correct` is however many of the up-to-5
/// rounds were answered correctly before finishing.
pub(crate) fn finish_play(cart: &mut Cart, correct: u8) {
    let now = cart.sim_now;
    let won = correct >= 3;
    let happy_gain = if won {
        GAME.play_happy_win
    } else {
        GAME.play_happy_lose
    };
    let h = cart.pet.happy.value_at(now);
    cart.pet
        .happy
        .reanchor(now, h.saturating_add(happy_gain).min(100));
    cart.timers.set(
        EventKind::HappyEmpty,
        cart.pet.happy.reaches(GAME.sad_threshold),
    );
    cart.pet.weight = ((cart.pet.weight as i32) + GAME.play_weight as i32).max(1) as u16;
    cart.clear_attention(attention::SAD);

    cart.ui = Ui::Busy {
        kind: BusyKind::Result { won },
    };
    cart.timers.set(EventKind::UiBusyEnd, now.saturating_add(2));
}
