//! The eight-export C ABI around a single static `Cart`. See docs/HOST_ABI.md. This crate
//! builds as a `cdylib` for `wasm32-unknown-unknown` (zero imports, `vpet.wasm`) and as an
//! `rlib` for native use (the parity harness loads both).
#![cfg_attr(not(test), no_std)]

use core::ptr::{addr_of, addr_of_mut};
use vpet_core::save::LoadError;
use vpet_core::{Cart, Inspect};

pub const VPET_IO_CAP: usize = 2048;

static mut CART: Cart = Cart::new_uninit();
static mut FRAME: [u8; 64] = [0u8; 64];
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

#[no_mangle]
pub extern "C" fn vpet_inspect() -> u32 {
    let insp: InspectAbi = unsafe { (*addr_of!(CART)).inspect() }.into();
    let bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(
            (&insp as *const InspectAbi) as *const u8,
            core::mem::size_of::<InspectAbi>(),
        )
    };
    // SAFETY: see sync_frame.
    unsafe {
        let io = &mut *addr_of_mut!(IO);
        io[0..bytes.len()].copy_from_slice(bytes);
    }
    bytes.len() as u32
}

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
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
