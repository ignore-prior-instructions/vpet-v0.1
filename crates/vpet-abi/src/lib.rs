//! The eight-export C ABI around a single static `Cart`. See docs/HOST_ABI.md. This crate
//! builds as a `cdylib` for `wasm32-unknown-unknown` (zero imports, `vpet.wasm`) and as an
//! `rlib` for native use (the parity harness loads both).
//!
//! The `dev-overrides` feature (docs/CONTENT.md "Hot preview without rebuilding the core")
//! pulls in `std` transitively through `vpet-core`, so this crate stops being `no_std` and
//! drops its own `#[panic_handler]`s under that feature too (std supplies one; two would
//! conflict). `just wasm-dev` builds that variant as `vpet-dev.wasm`, never shipped and not
//! subject to the size gate.
#![cfg_attr(not(any(test, feature = "dev-overrides")), no_std)]

use core::ptr::{addr_of, addr_of_mut};
use vpet_core::render::fb::FRAME_LEN;
use vpet_core::save::LoadError;
use vpet_core::{Cart, Inspect};

pub const VPET_IO_CAP: usize = 2048;

static mut CART: Cart = Cart::new_uninit();
static mut FRAME: [u8; FRAME_LEN] = [0u8; FRAME_LEN];
static mut IO: [u8; VPET_IO_CAP] = [0u8; VPET_IO_CAP];

/// Mirrors docs/HOST_ABI.md's `Inspect` table exactly, `#[repr(C)]` so a host can view the IO
/// buffer as this struct after `vpet_inspect()`.
#[repr(C)]
pub struct InspectAbi {
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

impl From<Inspect> for InspectAbi {
    fn from(i: Inspect) -> Self {
        InspectAbi {
            abi_version: i.abi_version,
            save_version: i.save_version,
            content_hash: i.content_hash,
            state: i.state,
            species: i.species,
            stage: i.stage,
            hunger: i.hunger,
            happiness: i.happiness,
            discipline: i.discipline,
            health: i.health,
            weight: i.weight,
            age_secs: i.age_secs,
            flags: i.flags,
            attention: i.attention,
            poops: i.poops,
            care_mistakes: i.care_mistakes,
            sim_now: i.sim_now,
            next_event_at: i.next_event_at,
        }
    }
}

fn sync_frame() {
    // SAFETY: single-threaded (wasm has no shared-memory threads here; native use is likewise
    // single-threaded per docs/HOST_ABI.md's host loop), and this is the only place that writes
    // FRAME, always right after the only place that mutates CART.
    unsafe {
        let cart = &*addr_of!(CART);
        (*addr_of_mut!(FRAME)) = *cart.frame();
    }
}

#[no_mangle]
pub extern "C" fn vpet_abi_version() -> u32 {
    vpet_core::ABI_VERSION
}

#[no_mangle]
pub extern "C" fn vpet_frame_ptr() -> *const u8 {
    addr_of!(FRAME) as *const u8
}

#[no_mangle]
pub extern "C" fn vpet_io_ptr() -> *mut u8 {
    addr_of_mut!(IO) as *mut u8
}

#[no_mangle]
pub extern "C" fn vpet_reset(now_ms: u64, seed: u64) {
    // SAFETY: see sync_frame.
    unsafe {
        (*addr_of_mut!(CART)).reset(now_ms, seed);
    }
    sync_frame();
}

#[no_mangle]
pub extern "C" fn vpet_load(len: u32) -> i32 {
    let len = len as usize;
    // SAFETY: see sync_frame; IO is read-only here.
    let blob: &[u8] = unsafe {
        let io = &*addr_of!(IO);
        match io.get(0..len) {
            Some(b) => b,
            None => return LoadError::TooLong.code(),
        }
    };
    let result = unsafe { (*addr_of_mut!(CART)).load(blob) };
    match result {
        Ok(()) => {
            sync_frame();
            0
        }
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "C" fn vpet_save() -> u32 {
    let mut scratch = [0u8; 512];
    // SAFETY: see sync_frame.
    let len = unsafe {
        let cart = &*addr_of!(CART);
        let io = &mut *addr_of_mut!(IO);
        cart.save(&mut scratch, io)
    };
    len.unwrap_or(0) as u32
}

#[no_mangle]
pub extern "C" fn vpet_update(now_ms: u64, buttons: u32) -> u32 {
    let out = unsafe { (*addr_of_mut!(CART)).update(now_ms, buttons as u8) };
    sync_frame();
    out
}

impl InspectAbi {
    pub const LEN: usize = 40;

    /// The struct's exact `#[repr(C)]` image with every padding byte zero. Copying the struct's
    /// memory directly would ship whatever the stack held in the three padding gaps (offsets
    /// 6..8, 19, 22..24) — harmless to a decoder, but the parity harness compares these bytes
    /// across hosts, and a host must not see different garbage per platform.
    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut b = [0u8; Self::LEN];
        b[0..4].copy_from_slice(&self.abi_version.to_le_bytes());
        b[4..6].copy_from_slice(&self.save_version.to_le_bytes());
        b[8..12].copy_from_slice(&self.content_hash.to_le_bytes());
        b[12] = self.state;
        b[13] = self.species;
        b[14] = self.stage;
        b[15] = self.hunger;
        b[16] = self.happiness;
        b[17] = self.discipline;
        b[18] = self.health;
        b[20..22].copy_from_slice(&self.weight.to_le_bytes());
        b[24..28].copy_from_slice(&self.age_secs.to_le_bytes());
        b[28] = self.flags;
        b[29] = self.attention;
        b[30] = self.poops;
        b[31] = self.care_mistakes;
        b[32..36].copy_from_slice(&self.sim_now.to_le_bytes());
        b[36..40].copy_from_slice(&self.next_event_at.to_le_bytes());
        b
    }
}

#[no_mangle]
pub extern "C" fn vpet_inspect() -> u32 {
    let insp: InspectAbi = unsafe { (*addr_of!(CART)).inspect() }.into();
    let bytes = insp.to_bytes();
    // SAFETY: see sync_frame.
    unsafe {
        let io = &mut *addr_of_mut!(IO);
        io[0..bytes.len()].copy_from_slice(&bytes);
    }
    bytes.len() as u32
}

/// `vpet_peek_sim_now(len)`: the `sim_now` inside the blob the host copied into IO, without
/// loading it (docs/SYNC.md: compare candidates, then load only the one that simulated
/// furthest). Returns `u32::MAX` if the blob does not parse (bad magic/CRC/decode, too new).
#[no_mangle]
pub extern "C" fn vpet_peek_sim_now(len: u32) -> u32 {
    // SAFETY: see sync_frame; IO is read-only here.
    let blob: &[u8] = unsafe {
        let io = &*addr_of!(IO);
        match io.get(0..len as usize) {
            Some(b) => b,
            None => return u32::MAX,
        }
    };
    vpet_core::save::peek_sim_now(blob).unwrap_or(u32::MAX)
}

#[cfg(all(not(test), not(feature = "dev-overrides"), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(all(not(test), not(feature = "dev-overrides"), not(target_arch = "wasm32")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// `dev-overrides`-only exports for `hosts/web`'s `/dev/sprites` hot-preview page (docs/
/// CONTENT.md "Hot preview without rebuilding the core"). All four functions read their
/// arguments out of the shared `IO` buffer as small hand-rolled binary records (there is no
/// room for a schema on the wire, and this is a dev tool only, not part of docs/HOST_ABI.md's
/// versioned surface). `hosts/web/src/dev/wire.ts` is the matching TS encoder; a change to any
/// record layout here must update it in the same commit.
///
/// Record layouts (all lengths are byte counts, all strings UTF-8):
/// - `dev_set_pose` payload: `slug_len:u8, slug, stage:u8, name_len:u8, name, w:u8, h:u8,
///   mouth_x:i8, mouth_y:i8, bbox_x0:u8, bbox_y0:u8, bbox_x1:u8, bbox_y1:u8, has_mask:u8,
///   img[ceil(w/8)*h], mask[ceil(w/8)*h] (only if has_mask != 0)`. `stage`: 0=baby, 1=child,
///   2=adult, 3=adult_alt.
/// - `dev_set_global` payload: `category_len:u8, category, name_len:u8, name, w:u8, h:u8,
///   img[ceil(w/8)*h]`.
/// - `dev_render_clip` payload: `slug_len:u8, slug, stage:u8, activity_len:u8, activity,
///   tick:u32 (little-endian)`. On success, overwrites `IO[0..FRAME_LEN]` with the rendered
///   frame (docs/HOST_ABI.md "Frame format") and returns `FRAME_LEN` (256); returns `0` (and
///   leaves `IO` untouched) if the payload itself is malformed.
#[cfg(feature = "dev-overrides")]
mod dev_exports {
    use core::ptr::{addr_of, addr_of_mut};
    use core::str;

    use super::{FRAME_LEN, IO, VPET_IO_CAP};

    struct Cursor<'a> {
        buf: &'a [u8],
        pos: usize,
    }

    impl<'a> Cursor<'a> {
        fn new(buf: &'a [u8]) -> Self {
            Cursor { buf, pos: 0 }
        }

        fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
            let b = self.buf.get(self.pos..self.pos + n)?;
            self.pos += n;
            Some(b)
        }

        fn u8(&mut self) -> Option<u8> {
            self.bytes(1).map(|b| b[0])
        }

        fn i8(&mut self) -> Option<i8> {
            self.u8().map(|b| b as i8)
        }

        fn u32_le(&mut self) -> Option<u32> {
            let b = self.bytes(4)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }

        fn str_with_u8_len(&mut self) -> Option<&'a str> {
            let len = self.u8()? as usize;
            let b = self.bytes(len)?;
            str::from_utf8(b).ok()
        }
    }

    fn stride(w: u8) -> usize {
        (w as usize).div_ceil(8)
    }

    /// -1: payload too short / malformed. -2: not valid UTF-8. 0: success.
    #[no_mangle]
    pub extern "C" fn dev_set_pose(len: u32) -> i32 {
        // SAFETY: see `sync_frame` in the parent module; IO is read-only for the duration of
        // this call (dev tooling is single-threaded, same as the rest of the ABI).
        let io: &[u8] = unsafe { &*addr_of!(IO) };
        let Some(payload) = io.get(0..(len as usize).min(VPET_IO_CAP)) else {
            return -1;
        };
        let mut c = Cursor::new(payload);
        let Some(slug) = c.str_with_u8_len() else {
            return -2;
        };
        let (Some(stage), Some(name)) = (c.u8(), c.str_with_u8_len()) else {
            return -1;
        };
        let (Some(w), Some(h)) = (c.u8(), c.u8()) else {
            return -1;
        };
        let (Some(mouth_x), Some(mouth_y)) = (c.i8(), c.i8()) else {
            return -1;
        };
        let (Some(bx0), Some(by0), Some(bx1), Some(by1)) = (c.u8(), c.u8(), c.u8(), c.u8()) else {
            return -1;
        };
        let Some(has_mask) = c.u8() else {
            return -1;
        };
        let n = stride(w) * h as usize;
        let Some(img) = c.bytes(n) else {
            return -1;
        };
        let mask = if has_mask != 0 {
            match c.bytes(n) {
                Some(m) => Some(m),
                None => return -1,
            }
        } else {
            None
        };
        vpet_core::dev::set_pose(
            slug,
            stage,
            name,
            w,
            h,
            img,
            mask,
            (mouth_x, mouth_y),
            (bx0, by0, bx1, by1),
        );
        0
    }

    /// Same error codes as `dev_set_pose`.
    #[no_mangle]
    pub extern "C" fn dev_set_global(len: u32) -> i32 {
        // SAFETY: see dev_set_pose.
        let io: &[u8] = unsafe { &*addr_of!(IO) };
        let Some(payload) = io.get(0..(len as usize).min(VPET_IO_CAP)) else {
            return -1;
        };
        let mut c = Cursor::new(payload);
        let Some(category) = c.str_with_u8_len() else {
            return -2;
        };
        let Some(name) = c.str_with_u8_len() else {
            return -2;
        };
        let (Some(w), Some(h)) = (c.u8(), c.u8()) else {
            return -1;
        };
        let n = stride(w) * h as usize;
        let Some(img) = c.bytes(n) else {
            return -1;
        };
        vpet_core::dev::set_global(category, name, w, h, img);
        0
    }

    #[no_mangle]
    pub extern "C" fn dev_clear_overrides() {
        vpet_core::dev::clear_all();
    }

    /// Returns `FRAME_LEN` (a frame was written to `IO[0..FRAME_LEN]`) or `0` (malformed
    /// payload; `IO` left untouched).
    #[no_mangle]
    pub extern "C" fn dev_render_clip(len: u32) -> u32 {
        // SAFETY: see dev_set_pose; the read of IO for parsing and the later write of the
        // frame don't overlap in time (the frame is only written once parsing has fully
        // succeeded), so there is no aliasing between the shared and exclusive borrows below.
        let io: &[u8] = unsafe { &*addr_of!(IO) };
        let Some(payload) = io.get(0..(len as usize).min(VPET_IO_CAP)) else {
            return 0;
        };
        let mut c = Cursor::new(payload);
        let Some(slug) = c.str_with_u8_len() else {
            return 0;
        };
        let (Some(stage), Some(activity)) = (c.u8(), c.str_with_u8_len()) else {
            return 0;
        };
        let Some(tick) = c.u32_le() else {
            return 0;
        };

        let frame = vpet_core::dev::render_clip(slug, stage, activity, tick);
        // SAFETY: see sync_frame in the parent module.
        unsafe {
            let io = &mut *addr_of_mut!(IO);
            io[0..FRAME_LEN].copy_from_slice(frame.as_bytes());
        }
        FRAME_LEN as u32
    }
}

/// Pins `InspectAbi`'s exact `#[repr(C)]` byte layout so the TypeScript decoder
/// (`hosts/web/src/inspect.ts`) can read fixed offsets without a schema on the wire. Any
/// change here (a reordered/added/removed field, or a size change) must update the TS decoder
/// in the same commit. A `const` assertion rather than a `#[test]`: `vpet-abi` can only be
/// linked for `wasm32-unknown-unknown` (see root `Cargo.toml`'s `default-members` comment), so
/// this is checked on every normal build instead, native or wasm alike.
const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(offset_of!(InspectAbi, abi_version) == 0);
    assert!(offset_of!(InspectAbi, save_version) == 4);
    assert!(offset_of!(InspectAbi, content_hash) == 8);
    assert!(offset_of!(InspectAbi, state) == 12);
    assert!(offset_of!(InspectAbi, species) == 13);
    assert!(offset_of!(InspectAbi, stage) == 14);
    assert!(offset_of!(InspectAbi, hunger) == 15);
    assert!(offset_of!(InspectAbi, happiness) == 16);
    assert!(offset_of!(InspectAbi, discipline) == 17);
    assert!(offset_of!(InspectAbi, health) == 18);
    assert!(offset_of!(InspectAbi, weight) == 20);
    assert!(offset_of!(InspectAbi, age_secs) == 24);
    assert!(offset_of!(InspectAbi, flags) == 28);
    assert!(offset_of!(InspectAbi, attention) == 29);
    assert!(offset_of!(InspectAbi, poops) == 30);
    assert!(offset_of!(InspectAbi, care_mistakes) == 31);
    assert!(offset_of!(InspectAbi, sim_now) == 32);
    assert!(offset_of!(InspectAbi, next_event_at) == 36);
    assert!(size_of::<InspectAbi>() == 40);
};
