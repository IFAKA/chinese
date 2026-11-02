use std::{fs, io, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concept {
    pub id: String,
    pub day: u8,
    pub hanzi: String,
    pub pinyin: String,
    pub english: String,
    pub constructions: Vec<String>,
    pub transfer_prompt: String,
    pub transfer_hanzi: String,
    pub transfer_pinyin: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lesson {
    pub day: u8,
    pub theme: String,
    pub units: Vec<Concept>,
}

pub fn load(path: impl AsRef<Path>) -> io::Result<Vec<Concept>> {
    let text = fs::read_to_string(path)?;
    text.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(parse_concept)
        .collect()
}

pub fn load_lessons(path: impl AsRef<Path>) -> io::Result<Vec<Lesson>> {
    let text = fs::read_to_string(path)?;
    let mut lessons: Vec<Lesson> = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 9 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid lesson row",
            ));
        }
        let day = fields[0]
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid day"))?;
        let concept = Concept {
            id: fields[2].to_owned(),
            day,
            hanzi: fields[3].to_owned(),
            pinyin: fields[4].to_owned(),
            english: fields[5].to_owned(),
            constructions: split_list(fields[6]),
            transfer_prompt: fields[7].to_owned(),
            transfer_hanzi: fields[8].split('|').next().unwrap_or("").to_owned(),
            transfer_pinyin: fields[8].split('|').nth(1).unwrap_or("").to_owned(),
        };
        if let Some(lesson) = lessons.iter_mut().find(|lesson| lesson.day == day) {
            lesson.units.push(concept);
        } else {
            lessons.push(Lesson {
                day,
                theme: fields[1].to_owned(),
                units: vec![concept],
            });
        }
    }
    Ok(lessons)
}

fn parse_concept(line: &str) -> io::Result<Concept> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() < 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid curriculum row",
        ));
    }
    Ok(Concept {
        id: fields[0].to_owned(),
        day: fields[1]
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid day"))?,
        hanzi: fields[2].to_owned(),
        pinyin: fields[3].to_owned(),
        english: fields[4].to_owned(),
        constructions: Vec::new(),
        transfer_prompt: String::new(),
        transfer_hanzi: String::new(),
        transfer_pinyin: String::new(),
    })
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .filter(|item| !item.trim().is_empty())
        .map(|item| item.trim().to_owned())
        .collect()
}
