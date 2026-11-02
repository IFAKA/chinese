use pinyin::{Pinyin, ToPinyin};

pub fn normalize_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn numbered_pinyin(hanzi: &str) -> String {
    let internal = hanzi
        .to_pinyin()
        .map(|item| item.map(Pinyin::with_tone_num).unwrap_or("").to_owned())
        .collect::<Vec<_>>()
        .join(" ");
    normalize_pinyin(&internal)
}

pub fn normalize_pinyin(value: &str) -> String {
    value
        .split_whitespace()
        .map(normalize_syllable)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_syllable(raw: &str) -> String {
    let lowered = raw.trim().to_lowercase();
    let chars = lowered.chars();
    let mut out = String::new();
    let mut numeric_tone = None;
    for ch in chars {
        if ch.is_ascii_digit() {
            numeric_tone = ch.to_digit(10);
            continue;
        }
        if !ch.is_alphabetic() && ch != 'ü' {
            continue;
        }
        let (base, tone) = tone_mark(ch);
        out.push(base);
        if tone.is_some() {
            numeric_tone = tone;
        }
    }
    out.push(char::from_digit(numeric_tone.unwrap_or(5), 10).unwrap());
    out
}

fn tone_mark(ch: char) -> (char, Option<u32>) {
    const MAP: &[(char, char, u32)] = &[
        ('ā', 'a', 1),
        ('á', 'a', 2),
        ('ǎ', 'a', 3),
        ('à', 'a', 4),
        ('ē', 'e', 1),
        ('é', 'e', 2),
        ('ě', 'e', 3),
        ('è', 'e', 4),
        ('ī', 'i', 1),
        ('í', 'i', 2),
        ('ǐ', 'i', 3),
        ('ì', 'i', 4),
        ('ō', 'o', 1),
        ('ó', 'o', 2),
        ('ǒ', 'o', 3),
        ('ò', 'o', 4),
        ('ū', 'u', 1),
        ('ú', 'u', 2),
        ('ǔ', 'u', 3),
        ('ù', 'u', 4),
        ('ǖ', 'ü', 1),
        ('ǘ', 'ü', 2),
        ('ǚ', 'ü', 3),
        ('ǜ', 'ü', 4),
    ];
    MAP.iter()
        .find(|(marked, _, _)| *marked == ch)
        .map_or((ch, None), |(_, base, tone)| (*base, Some(*tone)))
}

pub fn matches_pinyin(expected: &str, answer: &str) -> bool {
    let expected = normalize_pinyin(expected);
    let answer = normalize_pinyin(answer);
    expected == answer || same_letters(&expected) == same_letters(&answer)
}

pub fn same_pinyin_letters(expected: &str, answer: &str) -> bool {
    same_letters(&normalize_pinyin(expected)) == same_letters(&normalize_pinyin(answer))
}

fn same_letters(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_ascii_digit())
        .collect::<String>()
}
