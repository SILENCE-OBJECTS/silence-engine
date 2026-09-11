# silence-engine

Deterministic scheduler for SILENCE.OBJECTS. CPU-only. No ML. No system clock. No `03_ee`.

**Cargo.lock is the source of truth.** CI runs `cargo test --locked`. Do not bump deps without regenerating the lock.

This crate lived as `04_packages/@silence/engine` without its own lockfile. That freeze is lifted here, outside `silence-ecosystem`.

Disclaimer: *Non-clinical behavioral protocol. No diagnosis. No therapy.*
