use crate::database::{Context, Database, FilterMode};
use crate::domain::Pin;
use crate::settings::Settings;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use eyre::{Context as EyreContext, Result};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::prelude::{Buffer, Widget};
use ratatui::style::palette::tailwind::{GRAY, SLATE, YELLOW};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph, StatefulWidget};
use ratatui::{Frame, Terminal};
use std::fs::{self, File};
use std::io::stdout;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

mod tui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

enum RunningState {
    Active,
    Quit,
}

enum PageState {
    List,
}

struct EntryList<'a> {
    list: &'a [Pin],
}

#[derive(Default, Clone)]
struct ListState {
    offset: usize,
    selected: usize,
    entries_len: usize,
}

impl ListState {
    fn selected(&self) -> usize {
        self.selected
    }

    fn select(&mut self, i: usize) {
        self.selected = i;
    }
}

impl<'a> EntryList<'a> {
    fn new(list: &'a [Pin]) -> Self {
        Self { list }
    }
}

impl<'a> StatefulWidget for EntryList<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let height = area.height as usize;
        let block = Block::new();

        let lines = self
            .list
            .iter()
            .skip(state.offset)
            .take(height)
            .enumerate()
            .map(|(i, x)| {
                if i == state.selected {
                    Line::from(vec![
                        Span::styled(" > ", Style::new().fg(SLATE.c500)),
                        Span::styled(x.data.as_str(), Style::new().fg(SLATE.c500)),
                    ])
                } else {
                    Line::from(vec![Span::raw("   "), Span::raw(x.data.as_str())])
                }
            })
            .collect::<Vec<_>>();
        Paragraph::new(lines).block(block).render(area, buf);
    }
}

enum KeymapMode {
    Normal,
    Insert,
}

enum PromptMode {
    Info,
    Search,
}

struct InputCursor {
    source: String,
    index: usize,
}

impl InputCursor {
    fn insert(&mut self, c: char) {
        self.source.push(c);
        self.index += c.len_utf8();
    }

    fn remove(&mut self) {
        let len = self.source.len();
        if len == 0 {
            return;
        }

        let c = if self.index < len {
            self.source.remove(self.index)
        } else {
            self.source.pop().unwrap()
        };
        self.index -= c.len_utf8();
    }

    fn as_str(&self) -> &str {
        self.source.as_str()
    }

    fn clear(&mut self) {
        self.source.clear();
        self.index = 0;
    }
}

impl From<String> for InputCursor {
    fn from(source: String) -> Self {
        Self { source, index: 0 }
    }
}

struct PromptState {
    input: InputCursor,
    show_cursor: bool,
    mode: PromptMode,
}

impl PromptState {
    fn source(&self) -> &str {
        self.input.as_str()
    }
}

struct AppState<'a> {
    page: PageState,
    results_state: ListState,
    keymap_mode: KeymapMode,
    prompt: PromptState,
    results: Vec<Pin>,
    context: Context,
    running: RunningState,
    database: &'a Database,
}

impl AppState<'_> {
    async fn query_list(&mut self, ctx: &Context) -> Result<()> {
        self.results = self
            .database
            .list(&[FilterMode::Workspace], ctx, self.prompt.source())
            .await?;
        Ok(())
    }

    async fn query_save(&self, item: &Pin) -> Result<()> {
        self.database.save(item).await?;

        Ok(())
    }

    fn quit(&mut self) {
        self.running = RunningState::Quit;
    }

    fn running(&self) -> bool {
        match self.running {
            RunningState::Active => true,
            RunningState::Quit => false,
        }
    }

    fn selected(&self) -> Option<&Pin> {
        self.results.get(self.results_state.selected())
    }

    fn selected_mut(&mut self) -> Option<&mut Pin> {
        self.results.get_mut(self.results_state.selected())
    }

    fn handle_search_down(&mut self) -> Option<Event> {
        let i = self.results_state.selected() + 1;
        self.results_state
            .select(i.min(self.results.len().saturating_sub(1)));

        None
    }

    fn handle_search_up(&mut self) -> Option<Event> {
        let i = self.results_state.selected().saturating_sub(1);
        self.results_state.select(i);

        None
    }

    fn handle_keymap_mode(&mut self, mode: KeymapMode) -> Option<Event> {
        match mode {
            KeymapMode::Normal => {
                self.prompt.input.clear();
                self.prompt.show_cursor = false;
            }
            KeymapMode::Insert => self.prompt.show_cursor = true,
        }
        self.keymap_mode = mode;

        None
    }

    async fn handle_edit(&mut self) -> Option<Event> {
        let temp_path = Path::new("/tmp/temp_edit.txt");
        let mut temp_file = File::create(temp_path).expect("");
        let selected = self.selected().unwrap();

        writeln!(temp_file, "{}", &selected.data).expect("Failed to to write to file");
        drop(temp_file);
        disable_raw_mode().expect("Failed to disable raw mode");

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        Command::new(editor)
            .arg(temp_path)
            .status()
            .expect("Failed to open editor");
        let next_data = fs::read_to_string(temp_path).expect("Failed to read from file");
        let next_data = next_data.lines().next();
        fs::remove_file(temp_path).expect("Failed to delete file");

        if let Some(text) = next_data {
            if text != selected.data {
                let item = {
                    let item = self.selected_mut().expect("Failed to get selected item");
                    item.data = text.to_string();
                    item.clone()
                };
                self.query_save(&item)
                    .await
                    .expect("Failed to save to database");
            }
        }

        execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen");
        enable_raw_mode().expect("Failed to enable raw mode");
        Some(Event::TerminalRepaint)
    }

    // TODO: implement a sequence of keys
    fn handle_terminal_event(&mut self, ev: CrosstermEvent) -> Option<Event> {
        match ev {
            CrosstermEvent::FocusGained => None,
            CrosstermEvent::FocusLost => None,
            CrosstermEvent::Key(key_event) => return Some(Event::KeyInput(key_event)),
            CrosstermEvent::Mouse(_) => None,
            CrosstermEvent::Paste(_) => None,
            CrosstermEvent::Resize(_, _) => None,
        }
    }

    async fn handle_key_input(&mut self, key_event: KeyEvent) -> Option<Event> {
        let mut event = None;
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);

        match key_event.code {
            KeyCode::Char('c') if ctrl => {
                self.quit();
                return event;
            }
            _ => {}
        }

        event = match self.keymap_mode {
            KeymapMode::Normal => match key_event.code {
                KeyCode::Char('j') => self.handle_search_down(),
                KeyCode::Char('k') => self.handle_search_up(),
                KeyCode::Char('e') => self.handle_edit().await,
                KeyCode::Char('/') => self.handle_keymap_mode(KeymapMode::Insert),
                _ => None,
            },
            KeymapMode::Insert => match key_event.code {
                KeyCode::Esc => self.handle_keymap_mode(KeymapMode::Normal),
                KeyCode::Char(c) => {
                    self.prompt.input.insert(c);
                    None
                }
                KeyCode::Backspace => {
                    self.prompt.input.remove();
                    None
                }
                _ => None,
            },
        };

        event
    }

    async fn handle_event(&mut self, event: Event) -> Option<Event> {
        // TODO:: move to the even thandlers
        match event {
            Event::KeyInput(key_input) => self.handle_key_input(key_input).await,
            _ => None,
        }
    }

    fn build_title(&self) -> Paragraph {
        Paragraph::new(format!("Dirpin v{VERSION}"))
    }

    fn build_help(&self) -> Paragraph {
        Paragraph::new(Line::from(vec![
            Span::styled("<ctrl-c>", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(": exit"),
        ]))
        .alignment(Alignment::Right)
        .style(Style::new().fg(GRAY.c500))
    }

    fn build_context(&self) -> Paragraph {
        Paragraph::new(Line::from(vec![
            Span::styled("[ workspace ] ", Style::new().fg(GRAY.c500)),
            Span::styled(&self.context.cwd, Style::new().fg(SLATE.c500)),
        ]))
    }

    fn build_list(&self) -> EntryList {
        EntryList {
            list: &self.results,
        }
    }

    fn build_preview(&self) -> Paragraph {
        let content = if let Some(el) = self.selected() {
            let created_at = el.created_at.to_string();
            let updated_at = el.updated_at.to_string();
            let text = el.data.as_str();
            vec![
                Line::from(Span::raw(text)),
                Line::from(Span::raw(format!("Created at: {}", created_at))),
                Line::from(Span::raw(format!("Updated at: {}", updated_at))),
            ]
        } else {
            vec![Line::from(Span::raw("N/A"))]
        };

        Paragraph::new(content).block(Block::bordered().border_style(GRAY.c500))
    }

    fn build_mode(&self) -> Paragraph {
        let line = match self.keymap_mode {
            KeymapMode::Normal => {
                Line::from(Span::styled("[ normal ]", Style::new().fg(GRAY.c500)))
            }
            KeymapMode::Insert => {
                Line::from(Span::styled("[ insert ]", Style::new().fg(YELLOW.c500)))
            }
        };
        Paragraph::new(line)
    }

    fn build_input(&self) -> Paragraph {
        let line = match self.keymap_mode {
            KeymapMode::Normal => Line::from(Span::raw("")),
            KeymapMode::Insert => Line::from(Span::raw(format!(" {}", self.prompt.source()))),
        };
        Paragraph::new(line)
    }

    fn render_page(&mut self, frame: &mut Frame) {
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(1),
            ],
        );
        let layout_header = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)],
        );
        let layout_main = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)],
        );
        let layout_line = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Length(10), Constraint::Min(0)],
        );
        let [header_l, context_l, main_l, line_l] = layout.areas(frame.size());
        let [title_l, help_l] = layout_header.areas(header_l);
        let [list_l, preview_l] = layout_main.areas(main_l);
        let [mode_l, input_l] = layout_line.areas(line_l);

        let title = self.build_title();
        let help = self.build_help();
        let context = self.build_context();
        let content = match self.page {
            PageState::List => self.build_list(),
        };
        let preview = self.build_preview();
        let mode = self.build_mode();
        let input = self.build_input();
        // TODO: This is not idea but I am too tired this evening to think
        // about how to set this up.
        let mut state = self.results_state.clone();

        frame.render_widget(title, title_l);
        frame.render_widget(help, help_l);
        frame.render_widget(context, context_l);
        frame.render_stateful_widget(content, list_l, &mut state);
        frame.render_widget(preview, preview_l);
        frame.render_widget(mode, mode_l);
        frame.render_widget(input, input_l);

        if self.prompt.show_cursor {
            frame.set_cursor_position(Position::new(
                input_l.x + (self.prompt.source().len() as u16) + 1,
                input_l.y,
            ));
        }
    }
}

fn ev_key_press(ev: &CrosstermEvent) -> Option<&KeyEvent> {
    match ev {
        CrosstermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => Some(key_event),
        _ => None,
    }
}

//
// focus -> list, preview, prompt,
// prompt -> state
// results -> list of pins
// results_state -> the selected item
// show_preview -> bool
// state -> runing, quite
// filters -> view [FilterMode::workplace]
// context -> context,
//
//  Event
// // keymap_mode -> vim style, normal style
//  Query
// // database -> database reference
//
//

// UiAction -> app command
//  - start search
//  - line input
//  -

enum Event {
    AsyncDb(),
    KeyInput(KeyEvent),
    TerminalRepaint,
    TerminalTick,
    Quit,
}

struct EventManager {
    crossterm: EventStream,
    events: mpsc::UnboundedReceiver<Event>,
    dispatch: mpsc::UnboundedSender<Event>,
}

pub async fn run(_settings: &Settings, db: &Database, context: &Context) -> Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let (tx, rx) = mpsc::unbounded_channel();
    let mut app = AppState {
        page: PageState::List,
        results_state: ListState::default(),
        keymap_mode: KeymapMode::Normal,
        prompt: PromptState {
            input: InputCursor::from("".to_string()),
            show_cursor: false,
            mode: PromptMode::Info,
        },
        results: vec![],
        running: RunningState::Active,
        context: context.clone(),
        database: db,
    };
    let mut event_manager = EventManager {
        crossterm: EventStream::new(),
        events: rx,
        dispatch: tx.clone(),
    };

    app.query_list(context).await?;

    while app.running() {
        terminal
            .draw(|frame| app.render_page(frame))
            .wrap_err("failed to render terminal")?;

        let mut event = loop {
            match tokio::select! {
                event = event_manager.events.recv() => event,
                event = event_manager.crossterm.next() => match event {
                    Some(Ok(ev)) => app.handle_terminal_event(ev),
                    // TODO: there can be Some(Err()). Not sure if we need to handle it
                    _ => None
                },
                _ = sleep(Duration::from_millis(200)) => Some(Event::TerminalTick)
            } {
                Some(ev) => break Some(ev),
                _ => {}
            }
        };

        while let Some(ev) = event {
            match ev {
                Event::TerminalRepaint => {
                    terminal.clear().expect("Failed to clear terminal");
                    break;
                }
                ev => {
                    event = app.handle_event(ev).await;
                }
            }
        }

        if app.prompt.source().len() > 0 || matches!(app.keymap_mode, KeymapMode::Insert) {
            app.query_list(context).await?
        }
    }

    tui::restore()?;

    Ok(())
}
