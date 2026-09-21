# 0001: Rust core as a cartridge with thin hosts

Status: accepted
Date: 2026-09-20

## Context

v0 interleaved rules, DOM, storage, and dev tools in one TypeScript process. The new goal is one
pet that runs in a browser, on a server, and on a microcontroller. Candidates were Python
(CPython server, Pyodide browser, MicroPython device: three runtimes that mostly agree, and a
10 MB browser download), TypeScript (no real microcontroller story), and Rust.

## Decision

The simulation, rules, and renderer are one `no_std` Rust crate with an eight-function C ABI.
It compiles to `wasm32` for the browser and natively for the server and the ESP32. Hosts do
time, input, blitting, and persistence, and nothing else.

## Consequences

- One implementation of the game; parity between hosts is a test, not a hope.
- Rust's learning curve is the price; the core is small (a few thousand lines) and the hosts are
  thin, so most iteration is inside one crate.
- Art tooling stays in Python because it is offline and never ships.
