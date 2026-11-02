use fsrs::{MemoryState, FSRS};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardSummary {
    pub id: String,
    pub due_at: i64,
    pub reviews: u32,
    pub correct: u32,
}

pub struct Db {
    conn: Connection,
}

type CardRow = (Option<f32>, Option<f32>, i64, u32, u32);

impl Db {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS cards (id TEXT PRIMARY KEY, stability REAL, difficulty REAL, due_at INTEGER NOT NULL, reviewed_at INTEGER, reviews INTEGER NOT NULL DEFAULT 0, correct INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS lessons (day INTEGER PRIMARY KEY, completed_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS attempts (id INTEGER PRIMARY KEY, concept_id TEXT NOT NULL, modality TEXT NOT NULL, exercise_type TEXT NOT NULL, correct INTEGER NOT NULL, response_ms INTEGER NOT NULL, memory_state TEXT NOT NULL, answered_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS attempts_concept_modality ON attempts(concept_id, modality); CREATE TABLE IF NOT EXISTS competence (concept_id TEXT NOT NULL, modality TEXT NOT NULL, score REAL NOT NULL, attempts INTEGER NOT NULL, PRIMARY KEY(concept_id, modality)); CREATE TABLE IF NOT EXISTS construction_competence (construction_id TEXT NOT NULL, modality TEXT NOT NULL, score REAL NOT NULL, attempts INTEGER NOT NULL, PRIMARY KEY(construction_id, modality)); CREATE TABLE IF NOT EXISTS transfer_probes (id INTEGER PRIMARY KEY, attempt_id INTEGER NOT NULL, concept_id TEXT NOT NULL, correct INTEGER NOT NULL, response_ms INTEGER NOT NULL, session_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS transfer_probes_concept ON transfer_probes(concept_id);")?;
        Ok(Self { conn })
    }

    pub fn due_ids(&self, now: i64) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM cards WHERE due_at <= ?1 ORDER BY due_at, id")?;
        let rows = stmt.query_map([now], |row| row.get(0))?;
        rows.collect()
    }

    pub fn record(&self, id: &str, correct: bool, now: i64) -> rusqlite::Result<()> {
        self.record_attempt(id, "unknown", "legacy", correct, 0, now)
    }

    pub fn record_attempt(
        &self,
        id: &str,
        modality: &str,
        exercise_type: &str,
        correct: bool,
        response_ms: u64,
        now: i64,
    ) -> rusqlite::Result<()> {
        let current: Option<CardRow> = self.conn.query_row(
            "SELECT stability, difficulty, reviewed_at, reviews, correct FROM cards WHERE id=?1", [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))).optional()?;
        let was_reviewed = current.is_some();
        let (stability, difficulty, reviewed_at, reviews, correct_count) =
            current.unwrap_or((None, None, now, 0, 0));
        let elapsed_days = ((now - reviewed_at).max(0) / 86_400) as u32;
        let fsrs = FSRS::default();
        let states = fsrs
            .next_states(
                stability
                    .zip(difficulty)
                    .map(|(stability, difficulty)| MemoryState {
                        stability,
                        difficulty,
                    }),
                0.9,
                elapsed_days,
            )
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let state = if correct { states.good } else { states.again };
        let interval_seconds = (state.interval.max(1.0) * 86_400.0).round() as i64;
        self.conn.execute(
            "INSERT INTO cards (id, stability, difficulty, due_at, reviewed_at, reviews, correct) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET stability=excluded.stability, difficulty=excluded.difficulty, due_at=excluded.due_at, reviewed_at=excluded.reviewed_at, reviews=excluded.reviews, correct=excluded.correct",
            params![id, state.memory.stability, state.memory.difficulty, now + interval_seconds, now, reviews + 1, correct_count + u32::from(correct)],
        )?;
        let memory_state = if was_reviewed { "review" } else { "new" };
        self.conn.execute("INSERT INTO attempts (concept_id, modality, exercise_type, correct, response_ms, memory_state, answered_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![id, modality, exercise_type, correct, response_ms as i64, memory_state, now])?;
        self.conn.execute("INSERT INTO competence (concept_id, modality, score, attempts) VALUES (?1,?2,?3,1) ON CONFLICT(concept_id, modality) DO UPDATE SET score=(competence.score * competence.attempts + excluded.score)/(competence.attempts + 1), attempts=competence.attempts + 1", params![id, modality, f64::from(correct)])?;
        if exercise_type == "transfer" {
            let attempt_id = self.conn.last_insert_rowid();
            self.conn.execute("INSERT INTO transfer_probes (attempt_id, concept_id, correct, response_ms, session_at) VALUES (?1,?2,?3,?4,?5)", params![attempt_id, id, correct, response_ms as i64, now])?;
        }
        Ok(())
    }

    pub fn record_construction_attempt(
        &self,
        construction_id: &str,
        modality: &str,
        correct: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute("INSERT INTO construction_competence (construction_id, modality, score, attempts) VALUES (?1,?2,?3,1) ON CONFLICT(construction_id, modality) DO UPDATE SET score=(construction_competence.score * construction_competence.attempts + excluded.score)/(construction_competence.attempts + 1), attempts=construction_competence.attempts + 1", params![construction_id, modality, f64::from(correct)])?;
        Ok(())
    }

    pub fn seconds_practiced(&self) -> rusqlite::Result<u64> {
        let milliseconds: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(response_ms), 0) FROM attempts",
            [],
            |row| row.get(0),
        )?;
        Ok((milliseconds.max(0) as u64) / 1_000)
    }

    pub fn average_response_ms(&self) -> rusqlite::Result<HashMap<String, u64>> {
        let mut stmt = self.conn.prepare("SELECT modality, AVG(response_ms) FROM attempts WHERE response_ms > 0 GROUP BY modality")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;
        let mut averages = HashMap::new();
        for row in rows {
            let (modality, average) = row?;
            averages.insert(modality, average.max(1.0) as u64);
        }
        Ok(averages)
    }

    pub fn attempt_count(&self) -> rusqlite::Result<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM attempts", [], |row| {
                row.get::<_, i64>(0).map(|value| value as u64)
            })
    }

    pub fn competence_score(
        &self,
        concept_id: &str,
        modality: &str,
    ) -> rusqlite::Result<Option<f64>> {
        self.conn
            .query_row(
                "SELECT score FROM competence WHERE concept_id=?1 AND modality=?2",
                params![concept_id, modality],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn construction_competence_score(
        &self,
        construction_id: &str,
        modality: &str,
    ) -> rusqlite::Result<Option<f64>> {
        self.conn.query_row("SELECT score FROM construction_competence WHERE construction_id=?1 AND modality=?2", params![construction_id, modality], |row| row.get(0)).optional()
    }

    pub fn transfer_stats(&self) -> rusqlite::Result<(u32, u32)> {
        self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(correct), 0) FROM transfer_probes",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    pub fn delayed_transfer_per_minute(&self) -> rusqlite::Result<f64> {
        let (probes, correct) = self.transfer_stats()?;
        let minutes = (self.seconds_practiced()? as f64 / 60.0).max(1.0 / 60.0);
        Ok(f64::from(correct) / minutes / f64::from(probes.max(1)))
    }

    pub fn stats(&self) -> rusqlite::Result<(u32, u32, u32)> {
        self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(reviews),0), COALESCE(SUM(correct),0) FROM cards",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }

    pub fn next_lesson_day(&self) -> rusqlite::Result<u8> {
        let completed: u8 =
            self.conn
                .query_row("SELECT COALESCE(MAX(day), 0) FROM lessons", [], |row| {
                    row.get(0)
                })?;
        Ok((completed + 1).min(7))
    }

    pub fn complete_lesson(&self, day: u8, now: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lessons (day, completed_at) VALUES (?1, ?2)",
            params![day, now],
        )?;
        Ok(())
    }

    pub fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}
