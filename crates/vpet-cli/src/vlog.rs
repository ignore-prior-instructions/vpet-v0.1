//! `.vlog` input log parser. See docs/TESTING.md.

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub enum Cmd {
    Reset { now_ms: u64, seed: u64 },
    Tick { now_ms: u64, buttons: u8 },
    Skip { ms: u64 }, // documentary only: the following `t` line already carries the new now_ms
    Snap { name: String },
    Save,
    Load,
}

fn strip_comment(line: &str) -> &str {
    match line.find(';') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn parse_buttons(tok: &str) -> Result<u8> {
    if tok == "-" {
        return Ok(0);
    }
    let mut mask = 0u8;
    for c in tok.chars() {
        mask |= match c {
            'A' => vpet_core::buttons::A,
            'B' => vpet_core::buttons::B,
            'C' => vpet_core::buttons::C,
            other => bail!("unknown button '{other}' in {tok:?}"),
        };
    }
    Ok(mask)
}

pub fn parse(text: &str) -> Result<Vec<Cmd>> {
    let mut out = Vec::new();
    for (i, raw_line) in text.lines().enumerate() {
        let line_no = i + 1;
        if raw_line.trim_start().starts_with('#') {
            continue; // full-line comment, e.g. a "# hatch.vlog" file header
        }
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let head = parts.next().unwrap();
        match head {
            "reset" => {
                let now_ms: u64 = parts
                    .next()
                    .context(format!("line {line_no}: reset needs a now_ms"))?
                    .parse()?;
                let mut seed = 0u64;
                for rest in parts {
                    if let Some(v) = rest.strip_prefix("seed=") {
                        seed = v.parse()?;
                    }
                }
                out.push(Cmd::Reset { now_ms, seed });
            }
            "t" => {
                let now_ms: u64 = parts
                    .next()
                    .context(format!("line {line_no}: t needs a now_ms"))?
                    .parse()?;
                let btn_tok = parts.next().unwrap_or("-");
                let buttons = parse_buttons(btn_tok)?;
                out.push(Cmd::Tick { now_ms, buttons });
            }
            "skip" => {
                let ms: u64 = parts
                    .next()
                    .context(format!("line {line_no}: skip needs an ms value"))?
                    .parse()?;
                out.push(Cmd::Skip { ms });
            }
            "snap" => {
                let name = parts
                    .next()
                    .context(format!("line {line_no}: snap needs a name"))?
                    .to_string();
                out.push(Cmd::Snap { name });
            }
            "save" => out.push(Cmd::Save),
            "load" => out.push(Cmd::Load),
            other => bail!("line {line_no}: unknown command {other:?}"),
        }
    }
    Ok(out)
}
