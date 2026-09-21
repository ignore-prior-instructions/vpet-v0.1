//! Runs a `.vlog` against a `Cart`, comparing (or writing, with `--bless`) the resulting
//! `.expected` file. See docs/TESTING.md "Golden replays".

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use vpet_core::{Cart, Inspect};

use crate::vlog::{self, Cmd};

pub fn frame_to_ascii(frame: &[u8; 64]) -> String {
    let mut s = String::with_capacity(16 * 33);
    for y in 0..16usize {
        for x in 0..32usize {
            let byte = frame[y * 4 + x / 8];
            let bit = 7 - (x % 8);
            let on = (byte >> bit) & 1 != 0;
            s.push(if on { '#' } else { '.' });
        }
        s.push('\n');
    }
    s
}

fn inspect_lines(insp: &Inspect) -> String {
    format!(
        "abi_version = {}\n\
         save_version = {}\n\
         content_hash = {:#010x}\n\
         state = {}\n\
         species = {}\n\
         stage = {}\n\
         hunger = {}\n\
         happiness = {}\n\
         discipline = {}\n\
         health = {}\n\
         weight = {}\n\
         age_secs = {}\n\
         flags = {}\n\
         attention = {}\n\
         poops = {}\n\
         care_mistakes = {}\n\
         sim_now = {}\n\
         next_event_at = {}\n",
        insp.abi_version,
        insp.save_version,
        insp.content_hash,
        insp.state,
        insp.species,
        insp.stage,
        insp.hunger,
        insp.happiness,
        insp.discipline,
        insp.health,
        insp.weight,
        insp.age_secs,
        insp.flags,
        insp.attention,
        insp.poops,
        insp.care_mistakes,
        insp.sim_now,
        insp.next_event_at,
    )
}

fn snap_block(name: &str, cart: &Cart) -> String {
    format!(
        "## {name}\n{}\n{}\n",
        frame_to_ascii(cart.frame()),
        inspect_lines(&cart.inspect())
    )
}

pub struct RunResult {
    pub path: String,
    pub passed: bool,
    pub diff: Option<String>,
}

fn unified_diff(expected: &str, actual: &str) -> String {
    let e: Vec<&str> = expected.lines().collect();
    let a: Vec<&str> = actual.lines().collect();
    let mut out = String::new();
    let max = e.len().max(a.len());
    let mut shown = 0;
    for i in 0..max {
        let el = e.get(i).copied();
        let al = a.get(i).copied();
        if el != al {
            out.push_str(&format!(
                "line {}: expected {:?}, got {:?}\n",
                i + 1,
                el.unwrap_or("<EOF>"),
                al.unwrap_or("<EOF>")
            ));
            shown += 1;
            if shown >= 20 {
                out.push_str("... (further diffs suppressed)\n");
                break;
            }
        }
    }
    out
}

pub fn run(path: &Path, bless: bool, verbose: bool) -> Result<RunResult> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
    let cmds = vlog::parse(&text)?;

    let mut cart = Cart::new_uninit();
    let mut saved_blob: Option<Vec<u8>> = None;
    let mut blocks: Vec<String> = Vec::new();

    for cmd in cmds {
        match cmd {
            Cmd::Reset { now_ms, seed } => {
                cart.reset(now_ms, seed);
                if verbose {
                    eprintln!("reset now_ms={now_ms} seed={seed}");
                }
            }
            Cmd::Tick { now_ms, buttons } => {
                let flags = cart.update(now_ms, buttons);
                if verbose {
                    eprintln!("t {now_ms} buttons={buttons:#b} -> flags={flags:#b}");
                }
            }
            Cmd::Skip { ms } => {
                if verbose {
                    eprintln!("skip {ms}ms (documentary; next t carries the new now_ms)");
                }
            }
            Cmd::Snap { name } => {
                if verbose {
                    eprintln!("snap {name}");
                }
                blocks.push(snap_block(&name, &cart));
            }
            Cmd::Save => {
                let mut scratch = [0u8; 512];
                let mut blob = [0u8; 512];
                let len = cart
                    .save(&mut scratch, &mut blob)
                    .context("save: buffer too small")?;
                saved_blob = Some(blob[0..len].to_vec());
                if verbose {
                    eprintln!("save -> {len} bytes");
                }
            }
            Cmd::Load => {
                let blob = saved_blob
                    .as_ref()
                    .context("load: no prior `save` in this .vlog")?;
                cart.load(blob)
                    .map_err(|e| anyhow::anyhow!("load failed: {e:?}"))?;
                if verbose {
                    eprintln!("load <- {} bytes", blob.len());
                }
            }
        }
    }

    let actual = blocks.join("\n");
    let expected_path = path.with_extension("expected");

    if bless {
        fs::write(&expected_path, &actual).with_context(|| format!("writing {expected_path:?}"))?;
        return Ok(RunResult {
            path: path.display().to_string(),
            passed: true,
            diff: None,
        });
    }

    if !expected_path.exists() {
        bail!("{expected_path:?} does not exist; run with --bless to create it");
    }
    let expected =
        fs::read_to_string(&expected_path).with_context(|| format!("reading {expected_path:?}"))?;

    if expected == actual {
        Ok(RunResult {
            path: path.display().to_string(),
            passed: true,
            diff: None,
        })
    } else {
        Ok(RunResult {
            path: path.display().to_string(),
            passed: false,
            diff: Some(unified_diff(&expected, &actual)),
        })
    }
}
