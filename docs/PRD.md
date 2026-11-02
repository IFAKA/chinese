# Chinese PRD

## Positioning

Chinese is an offline-first Mandarin trainer for people preparing to live and work in mainland China. It turns a few minutes in the terminal into high-value practice for understanding and producing Mandarin in real situations.

## Objective

Optimize for durable real-world Mandarin ability gained per minute of learner attention. The target learner is an English-speaking beginner. Five minutes is a default practice budget for `chinese`, not the objective: the product must not optimize for exactly five minutes, a fixed number of questions, or maximum interaction count.

## Learning model

The smallest useful knowledge units are constructions, lexical items, and phrases/examples. A phrase can exercise multiple constructions. Lessons introduce a very small number of high-value constructions, words, and communicative functions, then demonstrate and recombine them in coherent situations. There is no exact phrase count per lesson.

The v1 model stores construction IDs on authored concepts, plus a practical transfer prompt and answer. It is intentionally not a linguistic knowledge graph.

## Practice contract

- `chinese` starts the highest-value available authored block for roughly five minutes.
- The deterministic planner uses estimated interaction time and value priorities, but never cuts an interaction off at 300 seconds.
- If the budget arrives during an answer/feedback interaction, that interaction reaches a clean boundary before the session ends.
- `chinese again` supplies another high-value block without pretending to be a separate deck.
- Listening prompts automatically play local audio, initially hide Hanzi and pinyin, and support immediate replay.
- `SPEAK` prompts can use macOS on-device speech recognition to show a live transcript while the learner speaks. Capture stops after 1.5 seconds of silence or 8 seconds maximum, then compares the final transcript with the expected Hanzi/pinyin and shows a green or red result. Microphone and speech permissions are required; unsupported systems fall back to manual Enter-to-reveal plus `y`/`n` self-rating. The result stays visible until Enter continues. Recognition is evidence only: v1 does not evaluate pronunciation acoustically.
- `PRODUCE` and `INPUT` remain typed production and digital-input exercises. They accept numbered pinyin, tone-marked pinyin, plain pinyin, and Hanzi where appropriate. Raw Hanzi typing and handwriting are not mandatory v1 exercises.
- On macOS, the terminal/helper must be granted Microphone and Speech Recognition permissions; without them, manual Enter-to-reveal remains available.

## Transfer and measurement

Immediate post-teaching correctness is weak evidence. Transfer probes are a first-class exercise and persistence stream. They use a different phrase, situation, or modality where possible and are recorded separately from ordinary attempts. Delayed transfer performance is measured as:

```text
delayed transfer performance / minutes practiced
```

Listening comprehension and contextual production receive the greatest emphasis. Supporting measures are delayed recall, response latency, listening/reading/production/digital-input competence, construction competence, lesson progression, and practice consistency. HSK coverage, vocabulary count, question count, streaks, XP, and time spent are not primary success measures.

Competence remains modality-specific: a learner may have strong reading recognition, weak listening, and medium contextual production for the same construction. Ordinary concept competence and construction competence are both persisted; transfer probes update confidence in reusable constructions through their linked authored concept.

## Scope

The vertical slice is seven situation-based lessons, local SQLite, Rust, ratatui/crossterm, FSRS, local audio, optional macOS on-device speech recognition, a deterministic planner, and the commands `chinese`, `chinese again`, and `chinese stats`. There are no accounts, cloud services, AI dependency, decks, XP, streak pressure, cloud speech service, handwriting course, or giant curriculum.

## Empirical status

This is a hypothesis-driven system intended to make useful Mandarin learning per minute measurable. It is not claimed to be optimal or “max SNR”. The implementation instruments transfer, modalities, constructions, latency, and time so future experiments can compare it with simpler scheduling approaches.
