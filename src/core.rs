use crate::{
    content::{Concept, Lesson},
    normalize::{matches_pinyin, normalize_text},
};
use std::collections::HashMap;

pub const DEFAULT_BUDGET_SECONDS: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exercise {
    ListenPinyin,
    Read,
    Speak,
    Produce,
    Input,
    Transfer,
}

impl Exercise {
    pub fn modality(self) -> &'static str {
        match self {
            Self::ListenPinyin => "listening",
            Self::Read => "reading",
            Self::Speak | Self::Transfer => "spoken_production",
            Self::Produce => "typed_production",
            Self::Input => "digital_input",
        }
    }

    pub fn expected_seconds(self) -> u64 {
        match self {
            Self::ListenPinyin => 30,
            Self::Read => 20,
            Self::Speak => 45,
            Self::Produce | Self::Transfer => 45,
            Self::Input => 35,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interaction {
    pub concept_id: String,
    pub exercise: Exercise,
    pub prompt: String,
    pub answer_revealed: bool,
    pub expected_seconds: u64,
}

/// Authored lesson order is the spine. The budget only truncates the block;
/// it does not turn FSRS into a lesson generator.
pub fn plan_lesson(lesson: &Lesson, due: &[String]) -> Vec<Interaction> {
    plan_lesson_for_budget(lesson, due, DEFAULT_BUDGET_SECONDS)
}

pub fn plan_lesson_for_budget(
    lesson: &Lesson,
    due: &[String],
    budget_seconds: u64,
) -> Vec<Interaction> {
    plan_lesson_adaptive(lesson, due, budget_seconds, &HashMap::new())
}

pub fn plan_lesson_adaptive(
    lesson: &Lesson,
    due: &[String],
    budget_seconds: u64,
    average_response_ms: &HashMap<String, u64>,
) -> Vec<Interaction> {
    let mut steps = Vec::new();
    let mut spent = 0;
    let add = |steps: &mut Vec<Interaction>,
               concept: &Concept,
               exercise: Exercise,
               prompt: String,
               expected_seconds: u64| {
        steps.push(Interaction {
            concept_id: concept.id.clone(),
            exercise,
            prompt,
            answer_revealed: false,
            expected_seconds,
        });
    };
    for concept in &lesson.units {
        let exercises = [
            (
                Exercise::ListenPinyin,
                "Listen. Enter what you hear.".to_owned(),
            ),
            (
                Exercise::Read,
                format!("Read this. What does it mean?  {}", concept.hanzi),
            ),
            (
                Exercise::Speak,
                format!("Speak aloud in Mandarin: {}", concept.english),
            ),
            (
                Exercise::Produce,
                format!("Typed production. Enter Mandarin for: {}", concept.english),
            ),
            (
                Exercise::Input,
                format!(
                    "Digital input practice. Enter Mandarin for: {}",
                    concept.english
                ),
            ),
            (Exercise::Transfer, concept.transfer_prompt.clone()),
        ];
        for (exercise, prompt) in exercises {
            let cost = average_response_ms
                .get(exercise.modality())
                .copied()
                .unwrap_or(exercise.expected_seconds() * 1_000)
                / 1_000;
            let cost = cost.max(1);
            if spent + cost > budget_seconds {
                break;
            }
            if exercise == Exercise::Transfer && prompt.is_empty() {
                continue;
            }
            add(&mut steps, concept, exercise, prompt, cost);
            spent += cost;
        }
    }
    // Due cards are a small tail, never allowed to displace first exposure or coverage.
    for id in due {
        if let Some(concept) = lesson.units.iter().find(|unit| &unit.id == id) {
            let cost = average_response_ms
                .get(Exercise::Produce.modality())
                .copied()
                .unwrap_or(Exercise::Produce.expected_seconds() * 1_000)
                / 1_000;
            let cost = cost.max(1);
            if spent + cost > budget_seconds {
                break;
            }
            add(
                &mut steps,
                concept,
                Exercise::Transfer,
                if concept.transfer_prompt.is_empty() {
                    format!("Recall this in a new situation: {}", concept.english)
                } else {
                    concept.transfer_prompt.clone()
                },
                cost,
            );
            spent += cost;
        }
    }
    steps
}

pub fn plan_follow_on(lesson: &Lesson, budget_seconds: u64) -> Vec<Interaction> {
    let mut steps = Vec::new();
    let mut spent = 0;
    for concept in lesson.units.iter().rev() {
        for exercise in [
            Exercise::Transfer,
            Exercise::Produce,
            Exercise::Input,
            Exercise::Speak,
            Exercise::ListenPinyin,
        ] {
            let cost = exercise.expected_seconds();
            if spent + cost > budget_seconds {
                return steps;
            }
            let prompt = match exercise {
                Exercise::Transfer => {
                    if concept.transfer_prompt.is_empty() {
                        format!("Different situation. Use this: {}", concept.english)
                    } else {
                        concept.transfer_prompt.clone()
                    }
                }
                Exercise::Produce => format!("Say this in Mandarin: {}", concept.english),
                Exercise::Input => format!(
                    "Digital input practice. Enter Mandarin for: {}",
                    concept.english
                ),
                Exercise::Speak => format!("Speak aloud in Mandarin: {}", concept.english),
                Exercise::ListenPinyin => "Listen again. What did you hear?".to_owned(),
                _ => unreachable!(),
            };
            steps.push(Interaction {
                concept_id: concept.id.clone(),
                exercise,
                prompt,
                answer_revealed: false,
                expected_seconds: cost,
            });
            spent += cost;
        }
    }
    steps
}

pub fn answer_is_correct(concept: &Concept, exercise: Exercise, answer: &str) -> bool {
    let answer = normalize_text(answer);
    match exercise {
        Exercise::Read => concept
            .english
            .split(';')
            .any(|meaning| normalize_text(meaning) == answer),
        Exercise::Speak
        | Exercise::Input
        | Exercise::ListenPinyin
        | Exercise::Produce
        | Exercise::Transfer => {
            let pinyin = if exercise == Exercise::Transfer && !concept.transfer_pinyin.is_empty() {
                &concept.transfer_pinyin
            } else {
                &concept.pinyin
            };
            let hanzi = if exercise == Exercise::Transfer && !concept.transfer_hanzi.is_empty() {
                &concept.transfer_hanzi
            } else {
                &concept.hanzi
            };
            matches_pinyin(pinyin, &answer) || answer == normalize_text(hanzi)
        }
    }
}

pub fn select_interactions(concepts: &[Concept], due: &[String], day: u8) -> Vec<Interaction> {
    plan_lesson(
        &Lesson {
            day,
            theme: String::new(),
            units: concepts.to_vec(),
        },
        due,
    )
}
