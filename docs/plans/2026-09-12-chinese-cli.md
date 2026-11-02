# Mandarin Real-World Learning Objective Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `chinese` spend a default five-minute budget on durable, real-world Mandarin ability, prioritizing listening and production while instrumenting transfer and response time.

**Architecture:** Keep seven manually authored situation lessons in `data/`, with each lesson supplying a compact sequence of exposure, listening, reading, production, digital-input, and transfer exercises. A time-budgeted scheduler chooses a bounded authored block; SQLite stores every attempt, response time, modality and construction competence, separate transfer probes, and FSRS card state. The CLI remains a thin terminal/audio adapter and FSRS contributes retrievability only, never lesson structure.

**Tech Stack:** Rust 2021, `clap`, `rusqlite` (bundled SQLite), `fsrs`, `pinyin` (the maintained rust-pinyin project), `serde`/`serde_json`, `dirs`, and macOS `say` as the local audio fallback.

---

### Task 1: Establish the product contract and curriculum

**Files:** Create `Cargo.toml`, `src/`, `data/curriculum.tsv`, `LICENSE`, `NOTICE.md`, `.gitignore`.

Preserve the existing crate and replace the fixed 12-step lesson contract with a roughly 300-second default budget. Curate seven coherent real-world situations with reusable construction IDs and explicit transfer prompts; keep provenance in `NOTICE.md` and avoid pretending source rows are lesson design.

### Task 2: Time-budgeted scheduler and modality-aware validation

**Files:** Create `src/content.rs`, `src/normalize.rs`, `src/core.rs`, `tests/core.rs`.

Write tests first for normalization, modality-aware validation, authored progression, transfer prompts, budget-bounded planning, optional digital input, construction reuse, and no fixed question count. Implement simple deterministic value-per-second heuristics and actual-time session stopping at clean interaction boundaries.

### Task 3: Telemetry, competence, and FSRS persistence

**Files:** Create `src/db.rs`, `tests/db.rs`.

Write tests first for schema creation, first-seen cards, attempt persistence including response time/modality/exercise type, separate modality and construction competence, separately tracked transfer probes, due-card ordering, and reopening the same database. Keep FSRS state and explicit timestamps, but use FSRS only to rank due review candidates.

### Task 4: CLI lesson loop and first-class audio

**Files:** Create `src/main.rs`, `src/cli.rs`, `src/audio.rs`.

Implement default `chinese`, `chinese stats`, and `chinese again`. The default command starts immediately with one obvious prompt and runs until the budget is spent or the learner exits. Audio plays automatically before Hanzi/pinyin for listening, with one replay shortcut and macOS `say` as the minimal fallback. Do not expose decks, ratings, XP, streaks, or implementation vocabulary.

### Task 5: Verification and runnable handoff

Run `cargo fmt --check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo build`, and an end-to-end Day 1 session using an isolated database and piped answers. Inspect the diff, confirm no runtime network calls, and report intentional gaps and empirical assumptions.
