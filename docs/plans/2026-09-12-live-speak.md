# Live Speak Recognition Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement the plan task-by-task.

**Goal:** Add an opt-in local macOS microphone path that shows partial speech transcription during `SPEAK`, stops on silence or a hard timeout, and preserves the existing manual offline fallback.

**Architecture:** A small Swift helper uses Apple Speech and AVFoundation frameworks to capture microphone audio, emit partial transcripts as newline-delimited JSON, stop after 1.5 seconds of silence or 8 seconds maximum, and emit a final result. A Rust `speech` module launches the helper and polls its output from the existing TUI loop. Non-macOS builds and helper failures keep the current Enter-to-reveal self-check path.

**Tech Stack:** Rust, ratatui/crossterm, macOS Swift Speech/AVFoundation bridge, Cargo `build.rs`, local SQLite.

---

### Task 1: Define the recognition session contract

**Files:**
- Create: `src/speech.rs`
- Modify: `src/lib.rs`
- Test: `src/speech.rs`

Add a `SpeechEvent` enum for partial text, final text, stopped, and unavailable states, plus a platform-neutral `SpeechSession` interface. Unit-test parsing of helper output and silence/max-duration constants without requiring a microphone.

### Task 2: Add the macOS helper build

**Files:**
- Create: `build.rs`
- Create: `src/macos_speech.swift`

Compile the helper only on macOS with Speech and AVFoundation frameworks. The helper must request microphone/speech authorization, emit partial text immediately, stop after 1.5 seconds without new speech or 8 seconds total, and exit cleanly with a final transcript. Build failures must be reported at runtime as unavailable rather than breaking non-macOS builds.

### Task 3: Integrate live transcript into the TUI state

**Files:**
- Modify: `src/tui.rs`
- Test: `src/tui.rs`

Add live transcript text and a capture-active flag to `LessonApp`. Start capture when a Speak interaction is displayed, update the transcript while polling the helper, reveal the model answer when final capture arrives, and retain Enter as the manual fallback/reveal key. Add tests for partial text visibility, finalization, and timeout-safe advancement.

### Task 4: Preserve scoring and feedback behavior

**Files:**
- Modify: `src/tui.rs`
- Modify: `README.md`
- Modify: `docs/PRD.md`

Keep `y`/`n` self-rating as the v1 competence signal. Display the recognized transcript as evidence only; do not claim acoustic pronunciation scoring. Document macOS permissions, silence/max timing, and fallback behavior.

### Task 5: Verify

Run:

```sh
CARGO_TARGET_DIR=/tmp/chinese-target cargo fmt --all -- --check
CARGO_TARGET_DIR=/tmp/chinese-target cargo test
CARGO_TARGET_DIR=/tmp/chinese-target cargo clippy --all-targets --all-features -- -D warnings
CARGO_TARGET_DIR=/tmp/chinese-target cargo build
```

Run an isolated interactive test with `CHINESE_AUDIO=off`, verify the live transcript/result states when microphone permission is available, and verify manual fallback when the helper is unavailable.
