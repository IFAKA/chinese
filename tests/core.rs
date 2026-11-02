use chinese::{
    content::{Concept, Lesson},
    core::{answer_is_correct, plan_lesson, Exercise},
    normalize::{matches_pinyin, normalize_pinyin, numbered_pinyin},
};

fn concept(id: &str, hanzi: &str, pinyin: &str, english: &str) -> Concept {
    Concept {
        id: id.into(),
        day: 1,
        hanzi: hanzi.into(),
        pinyin: pinyin.into(),
        english: english.into(),
        constructions: vec!["greeting".into()],
        transfer_prompt: "Use it at work.".into(),
        transfer_hanzi: hanzi.into(),
        transfer_pinyin: pinyin.into(),
    }
}

#[test]
fn accepts_numbered_and_marked_pinyin() {
    assert!(matches_pinyin("nǐ hǎo", "ni3 hao3"));
    assert_eq!(normalize_pinyin("Nǐ  hǎo"), "ni3 hao3");
    assert_eq!(numbered_pinyin("你好"), "ni3 hao3");
}

#[test]
fn phrase_answers_accept_beginner_formats() {
    let c = concept("x", "你好", "nǐ hǎo", "hello");
    assert!(answer_is_correct(&c, Exercise::Read, "HELLO"));
    assert!(answer_is_correct(&c, Exercise::ListenPinyin, "ni hao"));
    assert!(answer_is_correct(&c, Exercise::Input, "你好"));
    assert!(!answer_is_correct(&c, Exercise::Read, "goodbye"));
}

#[test]
fn accepts_plain_pinyin_with_phrase_punctuation() {
    let c = concept(
        "want-coffee",
        "我要咖啡。",
        "wǒ yào kāfēi.",
        "I want coffee",
    );
    assert!(answer_is_correct(&c, Exercise::Produce, "wo yao kafei"));
}

#[test]
fn lesson_is_authored_and_small() {
    let lesson = Lesson {
        day: 1,
        theme: "Meeting someone".into(),
        units: vec![
            concept("greeting", "你好", "nǐ hǎo", "hello"),
            concept("self-intro", "我是 Alex。", "wǒ shì Alex.", "I'm Alex."),
            concept(
                "question",
                "你是 Alex 吗？",
                "nǐ shì Alex ma?",
                "Are you Alex?",
            ),
        ],
    };
    let steps = plan_lesson(&lesson, &[]);
    assert!(
        steps.iter().map(|step| step.expected_seconds).sum::<u64>()
            <= chinese::core::DEFAULT_BUDGET_SECONDS
    );
    assert_eq!(steps[0].exercise, Exercise::ListenPinyin);
    assert!(steps.iter().any(|step| step.exercise == Exercise::Produce));
    assert!(steps.iter().any(|step| step.exercise == Exercise::Input));
    assert!(steps.iter().any(|step| step.exercise == Exercise::Speak));
    assert!(steps.iter().any(|step| step.exercise == Exercise::Transfer));
    assert!(steps.iter().any(|step| step.concept_id == "greeting"));
}

#[test]
fn spoken_and_typed_production_use_distinct_modalities() {
    assert_eq!(Exercise::Speak.modality(), "spoken_production");
    assert_eq!(Exercise::Transfer.modality(), "spoken_production");
    assert_eq!(Exercise::Produce.modality(), "typed_production");
    assert_eq!(Exercise::Input.modality(), "digital_input");
}

#[test]
fn follow_on_includes_spoken_and_typed_production() {
    let lesson = Lesson {
        day: 1,
        theme: "x".into(),
        units: vec![concept("a", "你好", "nǐ hǎo", "hello")],
    };
    let steps = chinese::core::plan_follow_on(&lesson, 300);
    assert!(steps.iter().any(|step| step.exercise == Exercise::Speak));
    assert!(steps.iter().any(|step| step.exercise == Exercise::Produce));
    assert!(steps.iter().any(|step| step.exercise == Exercise::Input));
}

#[test]
fn bundled_curriculum_has_seven_coherent_lessons() {
    let lessons =
        chinese::content::load_lessons(concat!(env!("CARGO_MANIFEST_DIR"), "/data/curriculum.tsv"))
            .unwrap();
    assert_eq!(lessons.len(), 7);
    assert!(lessons
        .iter()
        .all(|lesson| !lesson.units.is_empty() && !lesson.theme.is_empty()));
}

#[test]
fn constructions_are_reused_and_transfer_uses_a_new_answer() {
    let lessons =
        chinese::content::load_lessons(concat!(env!("CARGO_MANIFEST_DIR"), "/data/curriculum.tsv"))
            .unwrap();
    let drink = &lessons[1];
    assert_eq!(drink.units[0].constructions, drink.units[1].constructions);
    let transfer = plan_lesson(drink, &[])
        .into_iter()
        .find(|step| step.exercise == Exercise::Transfer)
        .unwrap();
    assert!(transfer.prompt.contains("water"));
    assert!(answer_is_correct(
        &drink.units[0],
        Exercise::Transfer,
        "wo3 yao4 shui3"
    ));
    assert!(!answer_is_correct(
        &drink.units[0],
        Exercise::Transfer,
        "wo3 yao4 ka fei"
    ));
}

#[test]
fn planner_is_deterministic_and_respects_whole_interaction_budget() {
    let lesson = Lesson {
        day: 1,
        theme: "x".into(),
        units: vec![concept("a", "你好", "nǐ hǎo", "hello")],
    };
    let first = chinese::core::plan_lesson_for_budget(&lesson, &[], 60);
    let second = chinese::core::plan_lesson_for_budget(&lesson, &[], 60);
    assert_eq!(first, second);
    assert!(first.iter().map(|step| step.expected_seconds).sum::<u64>() <= 60);
}
