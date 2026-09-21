//! `vpet-cli`: the test oracle and the fastest way to poke the core. See docs/hosts/cli.md.

mod autoplay;
#[cfg(feature = "parity")]
mod parity;
mod replay;
mod vlog;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use vpet_core::Cart;

#[derive(Parser)]
#[command(name = "vpet-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run one or more .vlog files, comparing against (or writing, with --bless) .expected.
    Replay {
        files: Vec<PathBuf>,
        #[arg(long)]
        bless: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Print Inspect and the decoded save for a blob (a file path, or a hex string).
    Dump { blob: String },
    /// Interactive terminal pet: type a/b/c then Enter to press a button, + to skip an hour, q
    /// to quit. Not raw-mode (docs/hosts/cli.md keeps this simple on purpose).
    Play {
        #[arg(long, default_value_t = 1)]
        speed: u64,
    },
    /// Play the pet by policy and write the resulting .vlog (docs/TESTING.md "Golden replays":
    /// how the long lifecycle goldens are produced). Deterministic per --seed.
    Autoplay {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 1_700_000_000_000)]
        start_ms: u64,
        /// Simulated seconds after --start-ms to stop at (a snap named `end` is taken there).
        #[arg(long, default_value_t = 86_400)]
        until_secs: u64,
        #[arg(long, value_enum, default_value_t = autoplay::Policy::Caretaker)]
        policy: autoplay::Policy,
        /// Fixed visit cadence (sloppy/neglect), or the longest gap between visits (caretaker).
        #[arg(long, default_value_t = 7_200)]
        every_secs: u64,
        /// Stop once the pet reaches any adult stage.
        #[arg(long, default_value_t = false)]
        stop_at_adult: bool,
    },
    /// Run each .vlog natively and through vpet.wasm on wasmi; diff flags, frames, blobs and
    /// Inspect (docs/TESTING.md "Cross-host parity"). Needs `--features parity`.
    #[cfg(feature = "parity")]
    Parity {
        #[arg(long, default_value = "target/dist/vpet.wasm")]
        wasm: PathBuf,
        files: Vec<PathBuf>,
    },
    /// Print the ABI version this build implements.
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Replay {
            files,
            bless,
            verbose,
        } => cmd_replay(&files, bless, verbose),
        Commands::Dump { blob } => cmd_dump(&blob),
        Commands::Play { speed } => cmd_play(speed),
        Commands::Autoplay {
            out,
            seed,
            start_ms,
            until_secs,
            policy,
            every_secs,
            stop_at_adult,
        } => autoplay::run(
            &autoplay::Opts {
                seed,
                start_ms,
                until_secs,
                policy,
                every_secs,
                stop_at_adult,
            },
            &out,
        ),
        #[cfg(feature = "parity")]
        Commands::Parity { wasm, files } => cmd_parity(&wasm, &files),
        Commands::Version => {
            println!("vpet-cli, ABI version {}", vpet_core::ABI_VERSION);
            Ok(())
        }
    }
}

#[cfg(feature = "parity")]
fn cmd_parity(wasm: &std::path::Path, files: &[PathBuf]) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("parity needs at least one .vlog path");
    }
    let mut any_failed = false;
    for path in files {
        let r = parity::run_file(wasm, path)?;
        match r.error {
            None => println!("ok    {} ({} commands)", r.path, r.steps),
            Some(msg) => {
                any_failed = true;
                println!("FAIL  {} at command {}:\n{}", r.path, r.steps, msg);
            }
        }
    }
    if any_failed {
        anyhow::bail!("native and wasm diverged on one or more logs");
    }
    Ok(())
}

fn cmd_replay(files: &[PathBuf], bless: bool, verbose: bool) -> Result<()> {
    if files.is_empty() {
        anyhow::bail!("replay needs at least one .vlog path");
    }
    let mut any_failed = false;
    for path in files {
        let result = replay::run(path, bless, verbose)?;
        if result.passed {
            println!("ok    {}", result.path);
        } else {
            any_failed = true;
            println!("FAIL  {}", result.path);
            if let Some(diff) = result.diff {
                print!("{diff}");
            }
        }
    }
    if any_failed {
        anyhow::bail!("one or more golden replays did not match");
    }
    Ok(())
}

fn cmd_dump(blob: &str) -> Result<()> {
    let bytes: Vec<u8> = if let Ok(data) = std::fs::read(blob) {
        data
    } else {
        // Treat as a hex string (optionally space-separated).
        let hex: String = blob.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .context("blob is neither a readable file nor a hex string")?
    };

    let mut cart = Cart::new_uninit();
    cart.load(&bytes)
        .map_err(|e| anyhow::anyhow!("load failed: {e:?}"))?;
    let insp = cart.inspect();
    println!("{insp:#?}");
    Ok(())
}

fn cmd_play(speed: u64) -> Result<()> {
    let mut cart = Cart::new_uninit();
    let mut now_ms: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    cart.reset(now_ms, now_ms);
    print_frame(cart.frame());

    println!("commands: a b c (press+release), + (skip 1h), q (quit)");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let cmd = line.trim();
        match cmd {
            "q" => break,
            "+" => {
                now_ms += 3_600_000 * speed;
                cart.update(now_ms, 0);
            }
            "a" | "b" | "c" => {
                let mask = match cmd {
                    "a" => vpet_core::buttons::A,
                    "b" => vpet_core::buttons::B,
                    _ => vpet_core::buttons::C,
                };
                now_ms += 100;
                cart.update(now_ms, mask);
                now_ms += 100;
                cart.update(now_ms, 0);
            }
            "" => {
                now_ms += 100;
                cart.update(now_ms, 0);
            }
            other => {
                println!("unknown command {other:?}");
                continue;
            }
        }
        print_frame(cart.frame());
        io::stdout().flush().ok();
    }
    Ok(())
}

fn print_frame(frame: &[u8; vpet_core::render::fb::FRAME_LEN]) {
    println!("{}", replay::frame_to_ascii(frame));
}
