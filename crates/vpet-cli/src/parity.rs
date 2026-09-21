//! `vpet-cli parity`: the cross-host parity harness (docs/TESTING.md, docs/DETERMINISM.md's
//! "all" row). Runs each `.vlog` through the native `Cart` and through `vpet.wasm` on the
//! wasmi interpreter, asserting identical update flags and 64-byte frames after every `t`,
//! identical blobs at every `save`, and identical `Inspect` bytes at every `snap`. The web and
//! ESP32 hosts are, by construction, that same wasm module or that same native crate, so this
//! is what "every host is the same core" rests on.
//!
//! wasmi rather than wasmtime: a pure-Rust interpreter that builds in seconds and needs no JIT;
//! the module has zero imports, so nothing is lost.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use wasmi::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::replay::frame_to_ascii;
use crate::vlog::{self, Cmd};
use vpet_core::{Cart, Inspect};

const FRAME_LEN: usize = 64;
const INSPECT_LEN: usize = 40;

struct Wasm {
    store: Store<()>,
    memory: Memory,
    frame_ptr: usize,
    io_ptr: usize,
    reset: TypedFunc<(i64, i64), ()>,
    update: TypedFunc<(i64, i32), i32>,
    save: TypedFunc<(), i32>,
    load: TypedFunc<i32, i32>,
    inspect: TypedFunc<(), i32>,
}

impl Wasm {
    fn load_module(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {path:?}"))?;
        let engine = Engine::default();
        let module = Module::new(&engine, &bytes).context("compiling vpet.wasm")?;
        if module.imports().len() != 0 {
            bail!("{path:?} has imports; the core must have none (docs/DETERMINISM.md rule 5)");
        }
        let mut store = Store::new(&engine, ());
        let linker: Linker<()> = Linker::new(&engine);
        let instance: Instance = linker
            .instantiate_and_start(&mut store, &module)
            .context("instantiating vpet.wasm")?;

        let memory = instance
            .get_memory(&store, "memory")
            .context("vpet.wasm exports no `memory`")?;
        let version = instance
            .get_typed_func::<(), i32>(&store, "vpet_abi_version")?
            .call(&mut store, ())?;
        if version as u32 != vpet_core::ABI_VERSION {
            bail!(
                "ABI mismatch: vpet.wasm is v{version}, this vpet-cli is v{}",
                vpet_core::ABI_VERSION
            );
        }
        let frame_ptr = instance
            .get_typed_func::<(), i32>(&store, "vpet_frame_ptr")?
            .call(&mut store, ())? as usize;
        let io_ptr = instance
            .get_typed_func::<(), i32>(&store, "vpet_io_ptr")?
            .call(&mut store, ())? as usize;

        Ok(Wasm {
            reset: instance.get_typed_func(&store, "vpet_reset")?,
            update: instance.get_typed_func(&store, "vpet_update")?,
            save: instance.get_typed_func(&store, "vpet_save")?,
            load: instance.get_typed_func(&store, "vpet_load")?,
            inspect: instance.get_typed_func(&store, "vpet_inspect")?,
            store,
            memory,
            frame_ptr,
            io_ptr,
        })
    }

    fn reset(&mut self, now_ms: u64, seed: u64) -> Result<()> {
        Ok(self
            .reset
            .call(&mut self.store, (now_ms as i64, seed as i64))?)
    }

    fn update(&mut self, now_ms: u64, buttons: u8) -> Result<u32> {
        Ok(self
            .update
            .call(&mut self.store, (now_ms as i64, buttons as i32))? as u32)
    }

    fn frame(&self) -> [u8; FRAME_LEN] {
        let mut out = [0u8; FRAME_LEN];
        out.copy_from_slice(
            &self.memory.data(&self.store)[self.frame_ptr..self.frame_ptr + FRAME_LEN],
        );
        out
    }

    fn io(&self, len: usize) -> Vec<u8> {
        self.memory.data(&self.store)[self.io_ptr..self.io_ptr + len].to_vec()
    }

    fn save(&mut self) -> Result<Vec<u8>> {
        let len = self.save.call(&mut self.store, ())? as usize;
        Ok(self.io(len))
    }

    fn load(&mut self, blob: &[u8]) -> Result<i32> {
        let io = &mut self.memory.data_mut(&mut self.store)[self.io_ptr..self.io_ptr + blob.len()];
        io.copy_from_slice(blob);
        Ok(self.load.call(&mut self.store, blob.len() as i32)?)
    }

    fn inspect_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.inspect.call(&mut self.store, ())? as usize;
        if len != INSPECT_LEN {
            bail!("vpet_inspect returned {len} bytes, expected {INSPECT_LEN}");
        }
        Ok(self.io(len))
    }
}

/// The native `Inspect` laid out exactly as `vpet_abi::InspectAbi` (`#[repr(C)]`, pinned by
/// the const assertions in crates/vpet-abi/src/lib.rs and mirrored by hosts/web/src/inspect.ts).
fn inspect_to_abi_bytes(i: &Inspect) -> [u8; INSPECT_LEN] {
    let mut b = [0u8; INSPECT_LEN];
    b[0..4].copy_from_slice(&i.abi_version.to_le_bytes());
    b[4..6].copy_from_slice(&i.save_version.to_le_bytes());
    b[8..12].copy_from_slice(&i.content_hash.to_le_bytes());
    b[12] = i.state;
    b[13] = i.species;
    b[14] = i.stage;
    b[15] = i.hunger;
    b[16] = i.happiness;
    b[17] = i.discipline;
    b[18] = i.health;
    b[20..22].copy_from_slice(&i.weight.to_le_bytes());
    b[24..28].copy_from_slice(&i.age_secs.to_le_bytes());
    b[28] = i.flags;
    b[29] = i.attention;
    b[30] = i.poops;
    b[31] = i.care_mistakes;
    b[32..36].copy_from_slice(&i.sim_now.to_le_bytes());
    b[36..40].copy_from_slice(&i.next_event_at.to_le_bytes());
    b
}

fn native_save(cart: &Cart) -> Result<Vec<u8>> {
    let mut scratch = [0u8; 512];
    let mut out = [0u8; 512];
    let len = cart
        .save(&mut scratch, &mut out)
        .context("native save: buffer too small")?;
    Ok(out[..len].to_vec())
}

pub struct FileResult {
    pub path: String,
    pub steps: usize,
    pub error: Option<String>,
}

pub fn run_file(wasm_path: &Path, vlog_path: &Path) -> Result<FileResult> {
    let text = fs::read_to_string(vlog_path).with_context(|| format!("reading {vlog_path:?}"))?;
    let cmds = vlog::parse(&text)?;
    let mut wasm = Wasm::load_module(wasm_path)?;
    let mut cart = Cart::new_uninit();
    let mut saved: Option<Vec<u8>> = None;
    let mut steps = 0usize;
    let path = vlog_path.display().to_string();

    let fail = |steps: usize, msg: String| FileResult {
        path: path.clone(),
        steps,
        error: Some(msg),
    };

    for (i, cmd) in cmds.iter().enumerate() {
        steps = i + 1;
        match cmd {
            Cmd::Reset { now_ms, seed } => {
                cart.reset(*now_ms, *seed);
                wasm.reset(*now_ms, *seed)?;
            }
            Cmd::Tick { now_ms, buttons } => {
                let nf = cart.update(*now_ms, *buttons);
                let wf = wasm.update(*now_ms, *buttons)?;
                if nf != wf {
                    return Ok(fail(
                        steps,
                        format!("t {now_ms}: flags differ: native {nf:#b}, wasm {wf:#b}"),
                    ));
                }
                let nfr = *cart.frame();
                let wfr = wasm.frame();
                if nfr != wfr {
                    return Ok(fail(
                        steps,
                        format!(
                            "t {now_ms}: frames differ\n--- native ---\n{}--- wasm ---\n{}",
                            frame_to_ascii(&nfr),
                            frame_to_ascii(&wfr)
                        ),
                    ));
                }
            }
            Cmd::Skip { .. } => {}
            Cmd::Snap { name } => {
                let nb = inspect_to_abi_bytes(&cart.inspect());
                let wb = wasm.inspect_bytes()?;
                if nb[..] != wb[..] {
                    return Ok(fail(
                        steps,
                        format!("snap {name}: inspect bytes differ\nnative {nb:?}\nwasm   {wb:?}"),
                    ));
                }
            }
            Cmd::Save => {
                let nb = native_save(&cart)?;
                let wb = wasm.save()?;
                if nb != wb {
                    return Ok(fail(
                        steps,
                        format!("save: blobs differ ({} vs {} bytes)", nb.len(), wb.len()),
                    ));
                }
                saved = Some(nb);
            }
            Cmd::Load => {
                let blob = saved
                    .as_ref()
                    .context("load: no prior `save` in this .vlog")?;
                cart.load(blob)
                    .map_err(|e| anyhow::anyhow!("native load failed: {e:?}"))?;
                let rc = wasm.load(blob)?;
                if rc != 0 {
                    return Ok(fail(steps, format!("load: wasm returned {rc}")));
                }
            }
        }
    }

    // Final blobs must agree too, whether or not the log ever said `save`.
    let nb = native_save(&cart)?;
    let wb = wasm.save()?;
    if nb != wb {
        return Ok(fail(steps, "end: final blobs differ".to_string()));
    }
    Ok(FileResult {
        path,
        steps,
        error: None,
    })
}
