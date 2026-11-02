use chinese::db::Db;

#[test]
fn persists_reviews_and_due_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("progress.sqlite");
    let db = Db::open(&path).unwrap();
    db.record("a", true, 1_000_000).unwrap();
    db.record("b", false, 1_000_000).unwrap();
    db.record_attempt("a", "listening", "listen_pinyin", true, 1_250, 1_000_001)
        .unwrap();
    let (cards, reviews, correct) = db.stats().unwrap();
    assert_eq!((cards, reviews, correct), (2, 3, 2));
    assert_eq!(db.attempt_count().unwrap(), 3);
    assert_eq!(db.competence_score("a", "listening").unwrap(), Some(1.0));
    assert_eq!(
        db.average_response_ms().unwrap().get("listening"),
        Some(&1_250)
    );
    assert!(db.due_ids(1_000_000).unwrap().is_empty());
    drop(db);
    let reopened = Db::open(&path).unwrap();
    assert_eq!(reopened.stats().unwrap(), (2, 3, 2));
}

#[test]
fn tracks_transfer_separately_from_ordinary_attempts_and_construction_competence() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path().join("progress.sqlite")).unwrap();
    db.record_attempt(
        "want-coffee",
        "production",
        "produce",
        true,
        2_000,
        2_000_000,
    )
    .unwrap();
    db.record_attempt(
        "want-coffee",
        "production",
        "transfer",
        false,
        3_000,
        2_000_100,
    )
    .unwrap();
    db.record_construction_attempt("want-X", "production", true)
        .unwrap();
    assert_eq!(db.attempt_count().unwrap(), 2);
    assert_eq!(db.transfer_stats().unwrap(), (1, 0));
    assert_eq!(
        db.competence_score("want-coffee", "production").unwrap(),
        Some(0.5)
    );
    assert_eq!(
        db.construction_competence_score("want-X", "production")
            .unwrap(),
        Some(1.0)
    );
    assert_eq!(db.delayed_transfer_per_minute().unwrap(), 0.0);
}

#[test]
fn competence_is_separate_by_modality() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path().join("progress.sqlite")).unwrap();
    db.record_attempt("hello", "reading", "read_meaning", true, 500, 3_000_000)
        .unwrap();
    db.record_attempt("hello", "listening", "listen_pinyin", false, 700, 3_000_001)
        .unwrap();
    assert_eq!(db.competence_score("hello", "reading").unwrap(), Some(1.0));
    assert_eq!(
        db.competence_score("hello", "listening").unwrap(),
        Some(0.0)
    );
}

#[test]
fn spoken_attempts_and_spoken_transfer_are_separate_from_typed_production() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path().join("progress.sqlite")).unwrap();
    db.record_attempt(
        "hello",
        "spoken_production",
        "speak",
        true,
        1_000,
        4_000_000,
    )
    .unwrap();
    db.record_attempt(
        "hello",
        "spoken_production",
        "transfer",
        false,
        1_500,
        4_000_001,
    )
    .unwrap();
    db.record_attempt(
        "hello",
        "typed_production",
        "produce",
        true,
        1_200,
        4_000_002,
    )
    .unwrap();
    assert_eq!(
        db.competence_score("hello", "spoken_production").unwrap(),
        Some(0.5)
    );
    assert_eq!(
        db.competence_score("hello", "typed_production").unwrap(),
        Some(1.0)
    );
    assert_eq!(db.transfer_stats().unwrap(), (1, 0));
}
