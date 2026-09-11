# silence-engine

Deterministic scheduler for SILENCE.OBJECTS. CPU-only. No ML. No system clock. No `03_ee`.

**Cargo.lock is the source of truth.** CI runs `cargo test --locked` on rustc 1.98.1. Do not bump deps without regenerating the lock.

This crate lived as `04_packages/@silence/engine` without its own lockfile. That freeze is lifted here, **outside `silence-ecosystem`**.

Operating surface for unfreeze/setup: this repo + [ev-silence-owner](https://github.com/ev-silence-owner?tab=repositories) + [silence-agents-org](https://github.com/silence-agents-org).

9 locked tests: SHA-256 vectors, bitwise schedule determinism, golden input hash, validation.

Disclaimer: *Non-clinical behavioral protocol. No diagnosis. No therapy.*
