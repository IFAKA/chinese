# Content and dependency provenance

The bundled seven-day slice is a deliberately small selection from HSK 3.0
beginner vocabulary. Its machine-readable reference is `ivankra/hsk30`,
particularly the `hsk30.csv` snapshot and its stated upstream sources:
https://github.com/ivankra/hsk30 (MIT for the repository's data snapshot; the
repository notes that the underlying standard is published by China's Ministry
of Education). Definitions are cross-checked against CC-CEDICT:
https://cc-cedict.org/wiki/start, licensed CC BY-SA 3.0. The slice is included
with attribution and is not a claim that the full HSK standard is bundled.

Runtime dependencies:

- `fsrs` 6.6.2: BSD-3-Clause, https://github.com/open-spaced-repetition/fsrs-rs
- `pinyin` 0.11.0 from `mozillazg/rust-pinyin`: MIT,
  https://github.com/mozillazg/rust-pinyin
- `rusqlite`, `clap`, `dirs`, `serde`, and `serde_json`: their licenses are
  recorded in Cargo's resolved dependency metadata.

No runtime network access is used. macOS `say` is an optional local audio
adapter using the Eddy mainland-Mandarin male voice by default; the lesson
remains usable if no Mandarin voice is installed.

Audio choice: fixed-curriculum native recordings are the preferred future
source when licensing and packaging are appropriate. This first slice has no
recording bundle, so it uses lightweight local macOS speech synthesis and does
not download a model or speech corpus. Mozilla Common Voice's Chinese (China)
release is CC0 but is far too large for this tiny CLI; OpenSLR 38 is smaller
but is CC BY-NC-ND 4.0 and is not suitable as a bundled application asset.
