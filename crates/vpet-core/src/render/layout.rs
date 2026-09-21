//! Every screen coordinate the compositor uses, in one place. docs/art/SCREEN_LAYOUT.md is the
//! picture; this file is the numbers (docs/adr/0016-screen-64x32.md). `compose.rs` must not
//! contain a bare coordinate.

/// The pet cell is bottom-anchored in a 32-row screen, so it is always drawn at y = 0.
pub const PET_CELL: i32 = 32;
/// Where a standing pet is pinned (eat, sick, sleep, busy, egg, tombstone) and where the walk
/// starts from.
pub const PET_HOME_X: i8 = 16;
/// Idle walk range, inclusive (docs/art/SCREEN_LAYOUT.md "Walk ranges and limits").
pub const WALK_LO: i8 = 2;
pub const WALK_HI: i8 = 30;
/// While a poop is on screen the walk stops short of the pile.
pub const DIRTY_WALK_HI: i8 = 14;

/// The 16x16 HUD slot (skull / `!`): top-right, never covered by the pet.
pub const HUD: (i32, i32) = (48, 0);
/// Poop pile: first at the bottom-right corner, later ones drawn over and up-left.
pub const POOP_PILE: [(i32, i32); 3] = [(48, 16), (50, 14), (52, 12)];

/// Eat scene: the food item at the left edge, mouth height.
pub const FOOD: (i32, i32) = (0, 16);
/// Result / evolution / tombstone effect above the pet's head-right.
pub const HEAD_EFFECT: (i32, i32) = (28, 0);
/// Evolution sparkles: opposite corners.
pub const SPARKLES: [(i32, i32); 2] = [(0, 0), (48, 16)];

/// Menu, laid out like the Tamagotchi Connection bezel: four 12x12 icons along the top edge
/// (feed, lights, play, medicine), four along the bottom (clean, status, discipline, battle),
/// the selected one inverted in its block, and the selected action's name centred in the
/// band between the rows (docs/art/SCREEN_LAYOUT.md "Menu").
pub const MENU_ICON: i32 = 12;
pub const MENU_SLOTS: [(i32, i32); 8] = [
    (2, 0),
    (18, 0),
    (34, 0),
    (50, 0),
    (2, 20),
    (18, 20),
    (34, 20),
    (50, 20),
];
/// Top row of the 5x7 label band (rows 12..=18, between the icon rows).
pub const LABEL_Y: i32 = 12;

/// Feed submenu: the two 16x16 items high, `MEAL` / `SNACK` under them.
pub const FEED_SUB_FOOD: (i32, i32) = (8, 4);
pub const FEED_SUB_SNACK: (i32, i32) = (40, 4);
pub const FEED_SUB_LABEL_Y: i32 = 24;
pub const ITEM: i32 = 16;

/// Status pages: label at the top-left, four 14x14 hearts (or bar segments) under it, page
/// dots along the bottom.
pub const STATUS_LABEL: (i32, i32) = (0, 0);
pub const STATUS_VALUE_X: i32 = 40;
pub const HEART_XS: [i32; 4] = [0, 16, 32, 48];
pub const HEART_Y: i32 = 10;
pub const BAR_Y: i32 = 14;
pub const BAR_SEG: (i32, i32) = (14, 6);
pub const STATUS_SKULL: (i32, i32) = (48, 12);
pub const PAGE_DOT_XS: [i32; 5] = [24, 28, 32, 36, 40];
pub const PAGE_DOT_Y: i32 = 30;
pub const DOT: i32 = 2;

/// Play: the `L?R` hint centred at the top, five round dots along the bottom.
pub const PLAY_HINT: (i32, i32) = (24, 0);
pub const PLAY_DOT_XS: [i32; 5] = [22, 26, 30, 34, 38];
pub const PLAY_DOT_Y: i32 = 30;

/// Text: the 5x7 font advances 6 px per glyph.
pub const GLYPH_W: i32 = 5;
pub const GLYPH_H: i32 = 7;
pub const GLYPH_ADVANCE: i32 = 6;
