# chinese

Chinese is an offline-first Mandarin trainer for people preparing to live and work in mainland China. It turns a few minutes in the terminal into high-value practice for understanding and producing Mandarin in real situations.

```text
chinese          # today's situation-based practice
chinese again    # another high-value block
chinese stats    # local progress summary
```

## Product contract

The north-star objective is durable real-world Mandarin ability gained per minute of learner attention. Five minutes is the default practice budget, not a hard cutoff, question quota, or optimization target. The planner selects a deterministic, small block by expected learning value and estimated time; an active interaction finishes cleanly before the session ends.

Lessons teach a few reusable constructions, words, and communicative functions through practical phrases and recombination. The seven-lesson slice covers meeting people, buying drinks, transport, housing, deliveries, coworker repair, and work meetings. A phrase may exercise several constructions, and constructions are reused across examples.

Listening is first-class: audio plays automatically before listening prompts, Hanzi and pinyin are initially hidden, and `Ctrl-R` replays. macOS `say` is the local fallback; `CHINESE_AUDIO=off` disables it. No network or speech-recognition dependency is required.

Speak prompts can use macOS on-device speech recognition to show a live transcript while the learner speaks. Capture stops after 1.5 seconds of silence or 8 seconds maximum, then automatically compares the final transcript with the expected Hanzi/pinyin and shows a green or red result. Microphone and speech permissions are required; unsupported systems fall back to manual Enter-to-reveal plus `y`/`n` self-rating. The result stays visible until Enter continues, and model audio can be replayed after reveal. Recognition is evidence only: v1 does not evaluate pronunciation acoustically. Typed `PRODUCE` and `INPUT` remain separate practices for pinyin and Chinese IME input. Raw Hanzi recall and handwriting are not core v1 skills.

The primary learning metric is delayed transfer performance per minute practiced, emphasizing listening comprehension and contextual production. Supporting measurements include delayed recall, response latency, modality-specific competence, construction competence, lesson progression, and practice consistency. HSK coverage, vocabulary count, questions, streaks, XP, and time in the app are not primary success metrics; HSK can remain metadata.

Progress is local SQLite. FSRS schedules quiet retrievability reviews but does not define the curriculum. There are no accounts, cloud services, AI dependencies, decks, XP, streak pressure, handwriting course, giant curriculum, or cloud speech service.

## Development

```sh
CARGO_TARGET_DIR=/tmp/chinese-target cargo fmt --all -- --check
CARGO_TARGET_DIR=/tmp/chinese-target cargo test
CARGO_TARGET_DIR=/tmp/chinese-target cargo clippy --all-targets --all-features -- -D warnings
CARGO_TARGET_DIR=/tmp/chinese-target cargo build
```

See [docs/PRD.md](docs/PRD.md) for the full product requirements and [NOTICE.md](NOTICE.md) for data provenance and licensing.

## Install and run

Build it with Rust: `cargo install --path .`, then run `chinese`.

## Tech

Rust, a local offline word and phrase dataset, and a text terminal interface.

## Support

If this project is useful to you, you can support my work:

[![Support me on Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/zfaka)
