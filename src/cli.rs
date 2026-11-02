use crate::{content, core, db::Db};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum Command {
    Stats,
    Again,
}

pub fn run(command: Option<Command>) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::var_os("CHINESE_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("chinese/progress.sqlite")
        });
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Db::open(&db_path)?;
    match command {
        Some(Command::Stats) => stats(&db)?,
        Some(Command::Again) | None => lesson(&db, command.is_some())?,
    }
    Ok(())
}

fn stats(db: &Db) -> Result<(), Box<dyn std::error::Error>> {
    let (_, reviews, correct) = db.stats()?;
    let day = db.next_lesson_day()?;
    let seconds = db.seconds_practiced()?;
    let (transfer_probes, transfer_correct) = db.transfer_stats()?;
    let transfer_rate = if transfer_probes == 0 {
        "—".to_owned()
    } else {
        format!("{transfer_correct}/{transfer_probes}")
    };
    println!("\n  Mandarin practice\n\n  Lesson day  {day}\n  Practice    {reviews} answers\n  Recall      {correct}/{reviews}\n  Transfer    {transfer_rate}\n  Time        {seconds}s\n");
    Ok(())
}

fn lesson(db: &Db, again: bool) -> Result<(), Box<dyn std::error::Error>> {
    let lessons =
        content::load_lessons(concat!(env!("CARGO_MANIFEST_DIR"), "/data/curriculum.tsv"))?;
    let day = std::env::var("CHINESE_DAY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(db.next_lesson_day()?);
    let lesson = lessons
        .iter()
        .find(|lesson| lesson.day == day)
        .or_else(|| lessons.last())
        .ok_or("curriculum is empty")?;
    let due = if again {
        Vec::new()
    } else {
        db.due_ids(Db::now())?
    };
    let response_times = db.average_response_ms()?;
    let interactions = if again {
        core::plan_follow_on(lesson, core::DEFAULT_BUDGET_SECONDS)
    } else {
        core::plan_lesson_adaptive(lesson, &due, core::DEFAULT_BUDGET_SECONDS, &response_times)
    };
    if crate::tui::is_interactive() {
        if crate::tui::interactive(lesson, day, interactions, db)? {
            db.complete_lesson(day, Db::now())?;
        }
    } else {
        crate::tui::line(lesson, &interactions, db)?;
        db.complete_lesson(day, Db::now())?;
    }
    Ok(())
}
