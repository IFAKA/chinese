use crate::{
    audio,
    content::{Concept, Lesson},
    core::{self, Exercise, Interaction},
    db::Db,
    normalize,
    speech::{SpeechEvent, SpeechSession},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
    Terminal,
};
use std::{
    io::{self, IsTerminal, Stdout, Write},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Answering,
    Feedback,
    Finished,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Replay,
    Revealed,
    Submitted(String),
    Rated(bool),
    Advanced,
    Quit,
}

pub struct LessonApp {
    pub interactions: Vec<Interaction>,
    pub index: usize,
    pub phase: Phase,
    pub input: String,
    pub cursor: usize,
    pub answer: String,
    pub correct: bool,
    pub live_transcript: String,
    pub speech_unavailable: bool,
    rated: bool,
    deadline: Instant,
    question_started: Instant,
}

impl LessonApp {
    pub fn new(interactions: Vec<Interaction>, budget_seconds: u64) -> Self {
        let now = Instant::now();
        Self {
            interactions,
            index: 0,
            phase: Phase::Answering,
            input: String::new(),
            cursor: 0,
            answer: String::new(),
            correct: false,
            live_transcript: String::new(),
            speech_unavailable: false,
            rated: false,
            deadline: now + Duration::from_secs(budget_seconds),
            question_started: now,
        }
    }

    pub fn current(&self) -> Option<&Interaction> {
        self.interactions.get(self.index)
    }
    pub fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }
    pub fn remaining_seconds(&self) -> u64 {
        self.deadline
            .saturating_duration_since(Instant::now())
            .as_secs()
    }
    pub fn response_ms(&self) -> u64 {
        self.question_started.elapsed().as_millis() as u64
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('`')) {
            self.phase = Phase::Quit;
            return Action::Quit;
        }
        match self.phase {
            Phase::Answering => {
                let is_speak = self
                    .current()
                    .is_some_and(|i| i.exercise == Exercise::Speak);
                if key.code == KeyCode::Char('r')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .current()
                        .is_some_and(|i| i.exercise == Exercise::ListenPinyin)
                {
                    return Action::Replay;
                }
                match key.code {
                    KeyCode::Enter => {
                        if is_speak {
                            return self.reveal_speak();
                        }
                        self.answer = self.input.trim().to_owned();
                        self.phase = Phase::Feedback;
                        Action::Submitted(self.answer.clone())
                    }
                    KeyCode::Backspace => {
                        self.backspace();
                        Action::None
                    }
                    KeyCode::Delete => {
                        self.delete();
                        Action::None
                    }
                    KeyCode::Left => {
                        self.cursor = if key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.previous_word_boundary()
                        } else {
                            self.input[..self.cursor]
                                .char_indices()
                                .next_back()
                                .map_or(0, |(index, _)| index)
                        };
                        Action::None
                    }
                    KeyCode::Right => {
                        self.cursor = if key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.next_word_boundary()
                        } else {
                            self.input[self.cursor..]
                                .chars()
                                .next()
                                .map_or(self.input.len(), |c| self.cursor + c.len_utf8())
                        };
                        Action::None
                    }
                    KeyCode::Home => {
                        self.cursor = 0;
                        Action::None
                    }
                    KeyCode::End => {
                        self.cursor = self.input.len();
                        Action::None
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.cursor = 0;
                        Action::None
                    }
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.cursor = self.input.len();
                        Action::None
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.input.drain(..self.cursor);
                        self.cursor = 0;
                        Action::None
                    }
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.input.truncate(self.cursor);
                        Action::None
                    }
                    KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let start = self.previous_word_boundary();
                        self.input.drain(start..self.cursor);
                        self.cursor = start;
                        Action::None
                    }
                    KeyCode::Char(c)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        self.input.insert(self.cursor, c);
                        self.cursor += c.len_utf8();
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Phase::Feedback => {
                if self
                    .current()
                    .is_some_and(|i| i.exercise == Exercise::Speak)
                {
                    match key.code {
                        KeyCode::Enter if self.rated => {
                            self.advance();
                            Action::Advanced
                        }
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            Action::Replay
                        }
                        KeyCode::Char('y') if !self.rated => {
                            self.correct = true;
                            self.rated = true;
                            Action::Rated(true)
                        }
                        KeyCode::Char('n') if !self.rated => {
                            self.correct = false;
                            self.rated = true;
                            Action::Rated(false)
                        }
                        _ => Action::None,
                    }
                } else if key.code == KeyCode::Enter {
                    self.advance();
                    Action::Advanced
                } else {
                    Action::None
                }
            }
            Phase::Finished | Phase::Quit => Action::Quit,
        }
    }

    fn advance(&mut self) {
        self.index += 1;
        self.input.clear();
        self.cursor = 0;
        self.answer.clear();
        self.correct = false;
        self.live_transcript.clear();
        self.speech_unavailable = false;
        self.rated = false;
        self.question_started = Instant::now();
        self.phase = if self.index >= self.interactions.len() {
            Phase::Finished
        } else {
            Phase::Answering
        };
    }

    fn reveal_speak(&mut self) -> Action {
        self.answer.clear();
        self.phase = Phase::Feedback;
        Action::Revealed
    }

    fn set_speech_event(&mut self, event: SpeechEvent) -> Option<Action> {
        match event {
            SpeechEvent::Partial(text) => self.live_transcript = text,
            SpeechEvent::Final(text) => {
                self.live_transcript = text;
                return Some(self.reveal_speak());
            }
            SpeechEvent::Unavailable(message) => {
                self.speech_unavailable = true;
                self.live_transcript = message;
            }
        }
        None
    }

    fn auto_rate_speech(&mut self, concept: &Concept) -> Option<bool> {
        let transcript = self.live_transcript.trim();
        if transcript.is_empty() || self.phase != Phase::Feedback {
            return None;
        }
        self.answer = transcript.to_owned();
        self.correct = core::answer_is_correct(concept, Exercise::Speak, transcript);
        self.rated = true;
        Some(self.correct)
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index);
        self.input.drain(start..self.cursor);
        self.cursor = start;
    }

    fn delete(&mut self) {
        let Some(character) = self.input[self.cursor..].chars().next() else {
            return;
        };
        self.input
            .drain(self.cursor..self.cursor + character.len_utf8());
    }

    fn previous_word_boundary(&self) -> usize {
        let before = &self.input[..self.cursor];
        let trimmed = before.trim_end_matches(char::is_whitespace);
        trimmed
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
            .map_or(0, |(index, character)| index + character.len_utf8())
    }

    fn next_word_boundary(&self) -> usize {
        let after = &self.input[self.cursor..];
        let leading = after.len() - after.trim_start_matches(char::is_whitespace).len();
        let non_whitespace = &after[leading..];
        non_whitespace
            .char_indices()
            .find(|(_, character)| character.is_whitespace())
            .map_or(self.input.len(), |(index, character)| {
                self.cursor + leading + index + character.len_utf8()
            })
    }

    pub fn header_text(&self, lesson: &Lesson, day: u8) -> String {
        format!("CHINESE   {}   DAY {}", lesson.theme, day)
    }
}

pub fn interactive(
    lesson: &Lesson,
    day: u8,
    interactions: Vec<Interaction>,
    db: &Db,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut terminal = TerminalGuard::enter()?;
    let mut app = LessonApp::new(interactions, core::DEFAULT_BUDGET_SECONDS);
    let mut audio_ready = false;
    let mut speech_session: Option<SpeechSession> = None;
    loop {
        if app.current().is_none() || (app.expired() && app.phase == Phase::Answering) {
            app.phase = Phase::Finished;
        }
        if app.phase == Phase::Finished || app.phase == Phase::Quit {
            break;
        }
        if app
            .current()
            .is_some_and(|interaction| interaction.exercise == Exercise::Speak)
            && app.phase == Phase::Answering
            && !app.speech_unavailable
            && speech_session.is_none()
        {
            match SpeechSession::start() {
                Ok(session) => speech_session = Some(session),
                Err(error) => {
                    app.speech_unavailable = true;
                    app.live_transcript = format!("Microphone unavailable: {error}");
                }
            }
        }
        let mut stop_speech_session = false;
        if let Some(session) = speech_session.as_mut() {
            match session.poll() {
                Ok(events) => {
                    for event in events {
                        if let Some(action) = app.set_speech_event(event) {
                            if action == Action::Revealed {
                                let interaction = app.current().cloned();
                                if let Some(interaction) = interaction {
                                    let concept = concept_for(lesson, &interaction);
                                    if let Some(correct) = app.auto_rate_speech(concept) {
                                        record_answer(
                                            db,
                                            concept,
                                            &interaction,
                                            correct,
                                            app.response_ms(),
                                        )?;
                                    }
                                }
                            }
                            stop_speech_session = true;
                            break;
                        }
                    }
                    if !stop_speech_session && session.finished() && app.phase == Phase::Answering {
                        stop_speech_session = true;
                        app.speech_unavailable = true;
                        app.live_transcript =
                            "Speech recognition stopped without a transcript".to_owned();
                    }
                }
                Err(error) => {
                    speech_session = None;
                    app.speech_unavailable = true;
                    app.live_transcript = format!("Microphone unavailable: {error}");
                }
            }
        }
        if stop_speech_session {
            speech_session = None;
        }
        if !audio_ready {
            if app
                .current()
                .is_some_and(|i| i.exercise == Exercise::ListenPinyin)
            {
                if let Some(interaction) = app.current() {
                    speak_for(lesson, interaction);
                }
            }
            audio_ready = true;
        }
        terminal
            .terminal
            .draw(|frame| draw(frame, &app, lesson, day))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let interaction = app.current().cloned();
                let response_ms = app.response_ms();
                let action = app.handle_key(key);
                if matches!(action, Action::Revealed) {
                    speech_session = None;
                }
                match action {
                    Action::Replay => {
                        if let Some(interaction) = app.current() {
                            speak_for(lesson, interaction);
                        }
                    }
                    Action::Submitted(answer) => {
                        if let Some(interaction) = interaction {
                            let concept = concept_for(lesson, &interaction);
                            let correct =
                                core::answer_is_correct(concept, interaction.exercise, &answer);
                            app.correct = correct;
                            record_answer(db, concept, &interaction, correct, response_ms)?;
                        }
                    }
                    Action::Rated(correct) => {
                        if let Some(interaction) = interaction {
                            let concept = concept_for(lesson, &interaction);
                            record_answer(db, concept, &interaction, correct, response_ms)?;
                        }
                        audio_ready = false;
                    }
                    Action::Revealed => {
                        if let Some(interaction) = interaction {
                            if interaction.exercise == Exercise::Speak {
                                let concept = concept_for(lesson, &interaction);
                                if app.live_transcript.trim().is_empty() {
                                    app.speech_unavailable = true;
                                } else if let Some(correct) = app.auto_rate_speech(concept) {
                                    record_answer(db, concept, &interaction, correct, response_ms)?;
                                }
                            }
                        }
                    }
                    Action::Advanced => audio_ready = false,
                    Action::None | Action::Quit => {}
                }
            }
        }
    }
    Ok(app.phase == Phase::Finished)
}

pub fn line(
    lesson: &Lesson,
    interactions: &[Interaction],
    db: &Db,
) -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    for interaction in interactions {
        if started.elapsed().as_secs() >= core::DEFAULT_BUDGET_SECONDS {
            break;
        }
        let concept = concept_for(lesson, interaction);
        let (answer, response_ms) = ask(interaction.exercise, concept, &interaction.prompt)?;
        let correct = if interaction.exercise == Exercise::Speak {
            answer == "__spoken_success__"
        } else {
            core::answer_is_correct(concept, interaction.exercise, &answer)
        };
        if interaction.exercise != Exercise::Speak {
            feedback(concept, interaction.exercise, &answer, correct);
        }
        record_answer(db, concept, interaction, correct, response_ms)?;
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &LessonApp, lesson: &Lesson, day: u8) {
    let area = frame.area();
    let accent = Color::Rgb(126, 200, 192);
    let ink = Color::Rgb(232, 236, 232);
    let muted = Color::Rgb(112, 122, 120);
    let quiet = Color::Rgb(72, 82, 80);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);
    let completed = app.index.min(app.interactions.len());
    let total = app.interactions.len().max(1);
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(19)])
        .split(vertical[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "CHINESE",
                Style::default().fg(ink).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ·  {}  ·  DAY {:02}", lesson.theme, day),
                Style::default().fg(muted),
            ),
        ])),
        header[0],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{:02}/{:02}  {}",
            completed + 1,
            total,
            format_time(app.remaining_seconds())
        ))
        .alignment(Alignment::Right)
        .style(Style::default().fg(muted)),
        header[1],
    );
    let Some(interaction) = app.current() else {
        return;
    };
    let concept = lesson
        .units
        .iter()
        .find(|unit| unit.id == interaction.concept_id);
    let Some(concept) = concept else {
        return;
    };
    let (title, body, input) = if app.phase == Phase::Feedback {
        (
            if interaction.exercise == Exercise::Speak && app.speech_unavailable && !app.rated {
                "NO TRANSCRIPT · SPEAK".to_owned()
            } else if interaction.exercise == Exercise::Speak && !app.rated {
                "MODEL ANSWER · SPEAK".to_owned()
            } else {
                format!(
                    "{} · {}",
                    feedback_title(app.correct),
                    modality(interaction.exercise)
                )
            },
            feedback_body(concept, interaction.exercise, &app.answer, app.correct),
            String::new(),
        )
    } else {
        (
            modality(interaction.exercise).to_owned(),
            interaction.prompt.clone(),
            app.input.clone(),
        )
    };
    let mut lines = vec![
        Line::from(Span::styled(
            title.to_uppercase(),
            Style::default()
                .fg(feedback_color(app, interaction.exercise, accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(body.lines().map(|line| {
        let response_line = line.starts_with("You entered:") || line.starts_with("You said:");
        let color = if response_line && app.phase == Phase::Feedback {
            feedback_color(app, interaction.exercise, ink)
        } else {
            ink
        };
        Line::from(Span::styled(line.to_owned(), Style::default().fg(color)))
    }));
    if app.phase == Phase::Answering && interaction.exercise == Exercise::Speak {
        lines.extend([
            Line::from(""),
            Line::from(Span::styled(
                if app.live_transcript.is_empty() {
                    "Listening… speak now"
                } else {
                    "Live transcript"
                },
                Style::default().fg(accent),
            )),
            Line::from(Span::styled(
                if app.live_transcript.is_empty() {
                    "(waiting for speech)"
                } else {
                    &app.live_transcript
                },
                Style::default().fg(ink),
            )),
            Line::from(Span::styled(
                if app.speech_unavailable {
                    "Speech recognition unavailable — press Enter for manual reveal"
                } else {
                    "Stops after 1.5s silence or 8s maximum"
                },
                Style::default().fg(muted),
            )),
        ]);
    }
    if app.phase == Phase::Answering && interaction.exercise != Exercise::Speak {
        lines.extend([
            Line::from(""),
            Line::from(vec![
                Span::styled("› ", Style::default().fg(accent)),
                Span::styled(
                    format!("{}▌{}", &input[..app.cursor], &input[app.cursor..]),
                    Style::default().fg(ink),
                ),
            ]),
        ]);
        if interaction.exercise == Exercise::ListenPinyin {
            lines.push(Line::from(Span::styled(
                "ctrl+r  replay",
                Style::default().fg(muted),
            )));
        }
    }
    let panel = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .block(Block::default().padding(Padding::horizontal(2)))
        .wrap(Wrap { trim: true });
    frame.render_widget(panel, vertical[1]);
    let footer = Paragraph::new(
        if app.phase == Phase::Feedback && interaction.exercise == Exercise::Speak && app.rated {
            "enter  continue    esc  quit"
        } else if app.phase == Phase::Feedback && interaction.exercise == Exercise::Speak {
            "y  I said it    n  needs practice    esc  quit"
        } else if app.phase == Phase::Feedback {
            "enter  continue    esc  quit"
        } else if interaction.exercise == Exercise::Speak {
            "speak now  silence stops    enter  reveal manually    esc  quit"
        } else if interaction.exercise == Exercise::ListenPinyin {
            "enter  submit    ctrl+r  replay    esc  quit"
        } else {
            "enter  submit    esc  quit"
        },
    )
    .alignment(Alignment::Left)
    .block(Block::default().padding(Padding::horizontal(2)))
    .style(Style::default().fg(quiet));
    frame.render_widget(footer, vertical[2]);
}
fn feedback_color(app: &LessonApp, exercise: Exercise, accent: Color) -> Color {
    if app.phase == Phase::Feedback && (exercise != Exercise::Speak || app.rated) {
        if app.correct {
            Color::Green
        } else {
            Color::Red
        }
    } else {
        accent
    }
}
fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn modality(exercise: Exercise) -> &'static str {
    match exercise {
        Exercise::ListenPinyin => "LISTEN",
        Exercise::Read => "READ",
        Exercise::Speak => "SPEAK",
        Exercise::Produce => "PRODUCE",
        Exercise::Transfer => "TRANSFER",
        Exercise::Input => "INPUT",
    }
}
fn feedback_title(correct: bool) -> &'static str {
    if correct {
        "✓ Correct"
    } else {
        "Not quite"
    }
}
fn feedback_body(concept: &Concept, exercise: Exercise, answer: &str, correct: bool) -> String {
    let response_label = if exercise == Exercise::Speak {
        "You said"
    } else {
        "You entered"
    };
    let response = if answer.is_empty() {
        "(nothing)"
    } else {
        answer
    };
    let no_transcript_note = if exercise == Exercise::Speak && answer.is_empty() {
        "No transcript captured. Check Microphone and Speech Recognition permissions.\n\n"
    } else {
        ""
    };
    if !correct
        && matches!(
            exercise,
            Exercise::ListenPinyin | Exercise::Produce | Exercise::Transfer
        )
        && normalize::same_pinyin_letters(&concept.pinyin, answer)
    {
        format!(
            "{}{}: {}\n\nAlmost — tones matter\n{}\n{}\n{}",
            no_transcript_note,
            response_label,
            response,
            if exercise == Exercise::Transfer && !concept.transfer_pinyin.is_empty() {
                &concept.transfer_pinyin
            } else {
                &concept.pinyin
            },
            normalize::numbered_pinyin(&concept.hanzi),
            concept.english
        )
    } else {
        format!(
            "{}{}: {}\n\n{}\n{}\n{}",
            no_transcript_note,
            response_label,
            response,
            if exercise == Exercise::Transfer && !concept.transfer_hanzi.is_empty() {
                &concept.transfer_hanzi
            } else {
                &concept.hanzi
            },
            if exercise == Exercise::Transfer && !concept.transfer_pinyin.is_empty() {
                &concept.transfer_pinyin
            } else {
                &concept.pinyin
            },
            concept.english
        )
    }
}
fn concept_for<'a>(lesson: &'a Lesson, interaction: &Interaction) -> &'a Concept {
    lesson
        .units
        .iter()
        .find(|unit| unit.id == interaction.concept_id)
        .expect("curriculum unit")
}
fn speak_for(lesson: &Lesson, interaction: &Interaction) {
    audio::speak(&concept_for(lesson, interaction).hanzi);
}

fn record_answer(
    db: &Db,
    concept: &Concept,
    interaction: &Interaction,
    correct: bool,
    response_ms: u64,
) -> rusqlite::Result<()> {
    let now = Db::now();
    db.record_attempt(
        &concept.id,
        interaction.exercise.modality(),
        exercise_name(interaction.exercise),
        correct,
        response_ms,
        now,
    )?;
    for construction in &concept.constructions {
        db.record_construction_attempt(construction, interaction.exercise.modality(), correct)?;
    }
    Ok(())
}
fn exercise_name(exercise: Exercise) -> &'static str {
    match exercise {
        Exercise::ListenPinyin => "listen_pinyin",
        Exercise::Read => "read_meaning",
        Exercise::Speak => "speak",
        Exercise::Produce => "produce",
        Exercise::Input => "digital_input",
        Exercise::Transfer => "transfer",
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let mut terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show);
                let _ = disable_raw_mode();
                return Err(error);
            }
        };
        if let Err(error) = terminal.clear() {
            let _ = disable_raw_mode();
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                crossterm::cursor::Show
            );
            let _ = terminal.backend_mut().flush();
            return Err(error);
        }
        Ok(Self { terminal })
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        let _ = self.terminal.backend_mut().flush();
    }
}

fn ask(exercise: Exercise, concept: &Concept, prompt: &str) -> io::Result<(String, u64)> {
    let started = Instant::now();
    if exercise == Exercise::ListenPinyin {
        audio::speak(&concept.hanzi);
    }
    println!("\n  {}\n\n  {}", modality(exercise), prompt);
    if exercise == Exercise::Speak {
        print!("\n  Press enter after speaking aloud. ");
        io::stdout().flush()?;
        let mut spoken = String::new();
        io::stdin().read_line(&mut spoken)?;
        feedback(concept, exercise, "", false);
        print!("\n  Did you say it successfully? [y/n] ");
        io::stdout().flush()?;
        let mut rating = String::new();
        io::stdin().read_line(&mut rating)?;
        return Ok((
            if rating.trim().eq_ignore_ascii_case("y") {
                "__spoken_success__"
            } else {
                ""
            }
            .to_owned(),
            started.elapsed().as_millis() as u64,
        ));
    }
    print!("\n  › ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim().eq_ignore_ascii_case("r") && exercise == Exercise::ListenPinyin {
        audio::speak(&concept.hanzi);
        answer.clear();
        io::stdin().read_line(&mut answer)?;
    }
    Ok((
        answer.trim().to_owned(),
        started.elapsed().as_millis() as u64,
    ))
}
fn feedback(concept: &Concept, exercise: Exercise, answer: &str, correct: bool) {
    println!(
        "\n  {}\n\n  {}\n  {}\n  {}",
        if correct {
            "✓ Nice."
        } else {
            "The phrase is:"
        },
        concept.hanzi,
        concept.pinyin,
        concept.english
    );
    if !correct
        && matches!(
            exercise,
            Exercise::ListenPinyin | Exercise::Produce | Exercise::Transfer
        )
        && normalize::same_pinyin_letters(&concept.pinyin, answer)
    {
        println!(
            "  Almost. Tones: {}",
            normalize::numbered_pinyin(&concept.hanzi)
        );
    }
}

pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn interaction(exercise: Exercise) -> Interaction {
        Interaction {
            concept_id: "x".into(),
            exercise,
            prompt: "Prompt".into(),
            answer_revealed: false,
            expected_seconds: 1,
        }
    }
    #[test]
    fn initial_state_is_empty() {
        let app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        assert_eq!(
            (app.index, app.phase, app.input),
            (0, Phase::Answering, String::new())
        );
    }
    #[test]
    fn typing_and_backspace_update_input() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.input, "a");
    }

    #[test]
    fn terminal_editing_shortcuts_edit_at_the_cursor() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        for character in "hello world".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.input, "hllo world");
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(app.input, "hllo ");
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.input, "");
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn cursor_can_insert_and_delete_unicode_characters() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        for character in "你好".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('啊'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.input, "你好");
    }
    #[test]
    fn replay_is_only_listening_and_does_not_submit() {
        let mut app = LessonApp::new(vec![interaction(Exercise::ListenPinyin)], 300);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Action::None
        );
        assert_eq!(app.input, "r");
        app.input.clear();
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::Replay
        );
        assert_eq!(app.phase, Phase::Answering);
        let mut read = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        assert_eq!(
            read.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::None
        );
    }
    #[test]
    fn ctrl_r_replays_while_typing_without_changing_input() {
        let mut app = LessonApp::new(vec![interaction(Exercise::ListenPinyin)], 300);
        app.input = "ni".into();
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::Replay
        );
        assert_eq!(app.input, "ni");
    }
    #[test]
    fn enter_feedback_then_enter_advances_and_clears() {
        let mut app = LessonApp::new(
            vec![interaction(Exercise::Read), interaction(Exercise::Input)],
            300,
        );
        app.input = "answer".into();
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Submitted("answer".into())
        );
        assert_eq!(app.phase, Phase::Feedback);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Advanced
        );
        assert_eq!(
            (app.index, app.phase, app.input),
            (1, Phase::Answering, String::new())
        );
    }

    #[test]
    fn speak_reveals_on_enter_and_rates_only_with_y_or_n() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Speak)], 300);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Revealed
        );
        assert_eq!(app.phase, Phase::Feedback);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Action::None
        );
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
            Action::Rated(true)
        );
        assert_eq!(app.phase, Phase::Feedback);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Advanced
        );
        assert_eq!(app.phase, Phase::Finished);
    }

    #[test]
    fn speak_replay_is_available_only_after_reveal() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Speak)], 300);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::None
        );
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Revealed
        );
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::Replay
        );
    }

    #[test]
    fn feedback_uses_green_for_success_and_red_for_error() {
        let mut success = LessonApp::new(vec![interaction(Exercise::Speak)], 300);
        success.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        success.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(
            feedback_color(&success, Exercise::Speak, Color::Cyan),
            Color::Green
        );

        let mut error = LessonApp::new(vec![interaction(Exercise::Speak)], 300);
        error.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        error.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(
            feedback_color(&error, Exercise::Speak, Color::Cyan),
            Color::Red
        );
    }

    #[test]
    fn live_speech_updates_before_final_reveal() {
        let concept = Concept {
            id: "x".into(),
            day: 1,
            hanzi: "你好".into(),
            pinyin: "nǐ hǎo".into(),
            english: "hello".into(),
            constructions: vec![],
            transfer_prompt: String::new(),
            transfer_hanzi: String::new(),
            transfer_pinyin: String::new(),
        };
        let mut app = LessonApp::new(vec![interaction(Exercise::Speak)], 300);
        assert_eq!(
            app.set_speech_event(SpeechEvent::Partial("你".into())),
            None
        );
        assert_eq!(app.live_transcript, "你");
        assert_eq!(app.phase, Phase::Answering);
        assert_eq!(
            app.set_speech_event(SpeechEvent::Final("你好".into())),
            Some(Action::Revealed)
        );
        assert_eq!(app.live_transcript, "你好");
        assert_eq!(app.phase, Phase::Feedback);
        assert_eq!(app.auto_rate_speech(&concept), Some(true));
        assert_eq!(app.answer, "你好");
        assert!(
            feedback_body(&concept, Exercise::Speak, &app.answer, app.correct)
                .contains("You said: 你好")
        );
    }
    #[test]
    fn feedback_reveals_answer_only_after_submission() {
        let concept = Concept {
            id: "x".into(),
            day: 1,
            hanzi: "你好".into(),
            pinyin: "nǐ hǎo".into(),
            english: "hello".into(),
            constructions: vec!["greeting".into()],
            transfer_prompt: "Greet someone at work.".into(),
            transfer_hanzi: "你好".into(),
            transfer_pinyin: "nǐ hǎo".into(),
        };
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        assert!(app.answer.is_empty());
        app.input = "hello".into();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.phase, Phase::Feedback);
        assert!(feedback_body(&concept, Exercise::Read, &app.answer, true)
            .contains("You entered: hello"));
        assert_eq!(
            feedback_body(&concept, Exercise::Read, &app.answer, true)
                .lines()
                .count(),
            5
        );
    }
    #[test]
    fn escape_quits_without_submission() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Quit
        );
        assert_eq!(app.phase, Phase::Quit);
    }

    #[test]
    fn backtick_quits_like_escape() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('`'), KeyModifiers::NONE)),
            Action::Quit
        );
        assert_eq!(app.phase, Phase::Quit);
    }
    #[test]
    fn header_has_time_but_no_question_count() {
        let lesson = Lesson {
            day: 1,
            theme: "Meeting".into(),
            units: vec![],
        };
        let app = LessonApp::new(vec![], 300);
        assert!(!app.header_text(&lesson, 1).contains('/'));
    }
    #[test]
    fn zero_budget_expires() {
        let app = LessonApp::new(vec![interaction(Exercise::Read)], 0);
        assert!(app.expired());
    }

    #[test]
    fn expired_feedback_finishes_only_after_current_interaction_is_advanced() {
        let mut app = LessonApp::new(vec![interaction(Exercise::Read)], 300);
        app.input = "answer".into();
        assert!(matches!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Submitted(_)
        ));
        app.deadline = Instant::now() - Duration::from_secs(1);
        assert_eq!(app.phase, Phase::Feedback);
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Advanced
        );
        assert_eq!(app.phase, Phase::Finished);
    }
}
