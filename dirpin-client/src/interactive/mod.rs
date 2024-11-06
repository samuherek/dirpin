use crate::database::{Context, Database, FilterMode};
use crate::domain::Pin;
use crate::settings::Settings;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen};
use eyre::{Context as EyreContext, Result};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::prelude::{Buffer, Widget};
use ratatui::style::palette::tailwind::{GRAY, SLATE, YELLOW};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidget};
use ratatui::Frame;
use std::fs::{self, File};
use std::io::stdout;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

mod tui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn handle_active_entry_list(state: &mut AppState, event: &KeyEvent) -> Option<Event> {
    match event.code {
        KeyCode::Char('j') => state.entry_list.list.move_down(),
        KeyCode::Char('k') => state.entry_list.list.move_up(),
        KeyCode::Char('f') => state.entry_list.cycle_context_mode(),
        _ => {}
    }

    None
}

#[derive(Default, Clone)]
struct StatefullList {
    offset: usize,
    selected: usize,
    entries_len: usize,
}

struct EntryListWidget<'a> {
    items: &'a [Pin],
}

impl<'a> StatefulWidget for EntryListWidget<'a> {
    type State = StatefullList;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let height = area.height as usize;
        let block = Block::new();

        let lines = self
            .items
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

enum PromptSearchStep {
    Edit,
    Submit,
}

struct PromptSearch {
    input: InputCursor,
    show_cursor: bool,
    steps: PromptSearchStep,
}

impl PromptSearch {
    fn source(&self) -> &str {
        self.input.as_str()
    }
}

enum PromptState {
    Default,
    Input,
    Search(PromptSearch),
    Info,
}

impl PromptState {
    fn prefix(&self) -> Option<&str> {
        None
    }

    fn value(&self) -> &str {
        match self {
            PromptState::Default => "Type : to entr command",
            PromptState::Input => "TODO: input",
            PromptState::Search(s) => s.source(),
            PromptState::Info => "TODO: info",
        }
    }

    fn style(&self) -> Style {
        match self {
            PromptState::Default => Style::new().fg(GRAY.c500),
            PromptState::Input => todo!(),
            PromptState::Search(_) => todo!(),
            PromptState::Info => todo!(),
        }
    }
}

struct PromptWidget<'a> {
    prefix: &'a str,
    value: &'a str,
    style: Style,
}

impl<'a> Widget for PromptWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // frame.render_widget(input, input_l);
        //
        // if self.prompt.show_cursor {
        //     frame.set_cursor_position(Position::new(
        //         input_l.x + (self.prompt.source().len() as u16) + 1,
        //         input_l.y,
        //     ));
        // }
        //
        // let line = match self.keymap_mode {
        //     KeymapMode::Normal => Line::from(Span::raw("")),
        //     KeymapMode::Insert => Line::from(Span::raw(format!(" {}", self.prompt.source()))),
        // };
        // Paragraph::new(line)
        let layout = Layout::new(
            Direction::Horizontal,
            [Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)],
        );
        let [left_l, right_l] = layout.areas(area);

        let prompt =
            Line::from(vec![Span::raw(self.prefix), Span::raw(self.value)]).style(self.style);

        let help = Line::from(vec![
            Span::raw("   Search "),
            Span::styled(" / ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
            Span::raw("   Help "),
            Span::styled(" ? ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
            Span::raw("   Exit "),
            Span::styled(" C-c ", Style::new().bg(SLATE.c800).fg(GRAY.c400)),
        ])
        .style(Style::new().fg(GRAY.c200))
        .alignment(Alignment::Right);

        Paragraph::new(prompt).render(left_l, buf);
        Paragraph::new(help).render(right_l, buf);
    }
}

trait SelectableList {
    type Item;

    fn selected(&self) -> Option<&Self::Item>;
    fn selected_mut(&mut self) -> Option<&mut Self::Item>;
    fn move_up(&mut self);
    fn move_down(&mut self);
}

struct List<T> {
    items: Vec<T>,
    offset: usize,
    selected: usize,
}

impl<T> List<T> {
    fn new(items: Vec<T>) -> Self {
        Self {
            items,
            offset: 0,
            selected: 0,
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_data(&mut self, data: Vec<T>) {
        // TODO: we probably don't want to replace it all the time.
        // Instead just clear and load the data to the same vector?
        self.items = data;
        self.offset = 0;
        self.selected = 0;
    }
}

impl<T> SelectableList for List<T> {
    type Item = T;

    fn selected(&self) -> Option<&Self::Item> {
        self.items.get(self.selected)
    }

    fn selected_mut(&mut self) -> Option<&mut Self::Item> {
        self.items.get_mut(self.selected)
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1));
    }
}

enum ActiveEntryContext {
    Workspace,
    Directory,
    Global,
}

struct EntryList {
    list: List<Pin>,
    show_preview: bool,
    context: Context,
    filter_mode: FilterMode,
    refetch: bool,
}

impl EntryList {
    fn items(&self) -> &[Pin] {
        &self.list.items
    }

    fn set_data(&mut self, data: Vec<Pin>) {
        self.list.set_data(data);
    }

    fn set_context_mode(&mut self, next_context: FilterMode) {
        self.filter_mode = next_context;
    }

    fn cycle_context_mode(&mut self) {
        match self.filter_mode {
            FilterMode::Workspace => self.set_context_mode(FilterMode::Global),
            FilterMode::Directory => self.set_context_mode(FilterMode::Workspace),
            FilterMode::Global => self.set_context_mode(FilterMode::Directory),
        }
    }
}

struct DirList {
    list: List<String>,
}

enum BlockFocus {
    List,
    Prompt,
}

enum Route {
    EntryList,
    DirectoryList,
    Help,
}

enum RunningState {
    Active,
    Quit,
}

struct AppState<'a> {
    route: Route,
    entry_list: EntryList,
    directory_list: DirList,
    prompt: PromptState,
    block_focus: BlockFocus,
    database: &'a Database,
    status: RunningState,
}

impl AppState<'_> {
    async fn query_entry_list(&mut self) -> Result<()> {
        let data = self
            .database
            .list(&[FilterMode::Workspace], &self.entry_list.context, "")
            .await?;
        self.entry_list.set_data(data);

        Ok(())
    }

    async fn query_save(&self, item: &Pin) -> Result<()> {
        // self.database.save(item).await?;

        Ok(())
    }

    fn quit(&mut self) {
        self.status = RunningState::Quit;
    }

    fn running(&self) -> bool {
        match self.status {
            RunningState::Active => true,
            RunningState::Quit => false,
        }
    }

    fn handle_keymap_mode(&mut self) -> Option<Event> {
        // match mode {
        //     KeymapMode::Normal => {
        //         self.prompt.input.clear();
        //         self.prompt.show_cursor = false;
        //     }
        //     KeymapMode::Insert => self.prompt.show_cursor = true,
        // }
        // self.keymap_mode = mode;

        None
    }

    async fn handle_edit(&mut self) -> Option<Event> {
        // let temp_path = Path::new("/tmp/temp_edit.txt");
        // let mut temp_file = File::create(temp_path).expect("");
        // let selected = self.list.selected().unwrap();
        //
        // writeln!(temp_file, "{}", &selected.data).expect("Failed to to write to file");
        // drop(temp_file);
        // disable_raw_mode().expect("Failed to disable raw mode");
        //
        // let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        // Command::new(editor)
        //     .arg(temp_path)
        //     .status()
        //     .expect("Failed to open editor");
        // let next_data = fs::read_to_string(temp_path).expect("Failed to read from file");
        // let next_data = next_data.lines().next();
        // fs::remove_file(temp_path).expect("Failed to delete file");
        //
        // if let Some(text) = next_data {
        //     if text != selected.data {
        //         let item = {
        //             let item = self.selected_mut().expect("Failed to get selected item");
        //             item.data = text.to_string();
        //             item.version += 1;
        //             item.updated_at = OffsetDateTime::now_utc();
        //             item.clone()
        //         };
        //         self.query_save(&item)
        //             .await
        //             .expect("Failed to save to database");
        //     }
        // }
        //
        // execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen");
        // enable_raw_mode().expect("Failed to enable raw mode");
        // Some(Event::TerminalRepaint)
        None
    }

    fn handle_toggle_preview(&mut self) -> Option<Event> {
        // self.show_preview = !self.show_preview;

        None
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

    fn handle_global_exit(&mut self, event: &KeyEvent) -> bool {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        match event.code {
            KeyCode::Char('c') if ctrl => {
                self.quit();
                true
            }
            _ => false,
        }
    }

    async fn handle_key_input(&mut self, key_event: KeyEvent) -> Option<Event> {
        let mut event = None;
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);

        if self.handle_global_exit(&key_event) {
            return event;
        }

        // Ignore the tmux ctrl-a
        event = match self.route {
            Route::EntryList => handle_active_entry_list(self, &key_event),
            Route::DirectoryList => todo!(),
            Route::Help => todo!(),
        };
        // event = match self.keymap_mode {
        //     KeymapMode::Normal => match key_event.code {
        //         KeyCode::Char('j') => self.handle_search_down(),
        //         KeyCode::Char('k') => self.handle_search_up(),
        //         KeyCode::Char('e') => self.handle_edit().await,
        //         KeyCode::Char('/') => self.handle_keymap_mode(KeymapMode::Insert),
        //         KeyCode::Char('p') => self.handle_toggle_preview(),
        //         _ => None,
        //     },
        //     KeymapMode::Insert => match key_event.code {
        //         KeyCode::Esc => self.handle_keymap_mode(KeymapMode::Normal),
        //         KeyCode::Char(c) => {
        //             self.prompt.input.insert(c);
        //             None
        //         }
        //         KeyCode::Backspace => {
        //             self.prompt.input.remove();
        //             None
        //         }
        //         _ => None,
        //     },
        // };

        event
    }

    async fn handle_event(&mut self, event: Event) -> Option<Event> {
        // TODO:: move to the even thandlers
        match event {
            Event::KeyInput(key_input) => self.handle_key_input(key_input).await,
            _ => None,
        }
    }

    fn build_context(&self) -> Paragraph {
        Paragraph::new(Line::from(vec![
            Span::styled("[ workspace ] ", Style::new().fg(GRAY.c500)),
            Span::styled(&self.entry_list.context.cwd, Style::new().fg(SLATE.c500)),
        ]))
    }

    fn render_entry_list(&self, frame: &mut Frame, rect: Rect) {
        // TODO: This is not ideal but I am too tired this evening to think
        // about how to set this up. We create a new state for each rerender.
        // let state: StatefullList = self.entry_list.list.into();
        let state = &self.entry_list.list;
        let height = rect.height as usize;

        let lines = self
            .entry_list
            .items()
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

        match lines.len() {
            0 => frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    "No entries",
                    Style::new().fg(GRAY.c500),
                )])),
                rect,
            ),
            _ => {
                let content = Paragraph::new(lines);

                if self.entry_list.show_preview {
                    let layout_main = Layout::new(
                        Direction::Horizontal,
                        vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)],
                    );
                    let [list_l, preview_l] = layout_main.areas(rect);

                    frame.render_widget(content, list_l);
                    // frame.render_widget(preview, preview_l);
                } else {
                    frame.render_widget(content, rect);
                }
            }
        }
    }

    fn build_preview(&self) -> Paragraph {
        // let content = if let Some(el) = self.selected() {
        //     let created_at = el.created_at.to_string();
        //     let updated_at = el.updated_at.to_string();
        //     let text = el.data.as_str();
        //     vec![
        //         Line::from(Span::raw(text)),
        //         Line::from(Span::raw(format!("Created at: {}", created_at))),
        //         Line::from(Span::raw(format!("Updated at: {}", updated_at))),
        //     ]
        // } else {
        // vec![Line::from(Span::raw("N/A"))]
        // };
        let content = vec![Line::from(Span::raw("N/A"))];

        Paragraph::new(content).block(Block::bordered().border_style(GRAY.c500))
    }

    fn build_prompt(&self) -> PromptWidget {
        PromptWidget {
            prefix: self.prompt.prefix().unwrap_or(""),
            value: self.prompt.value(),
            style: self.prompt.style(),
        }
    }

    fn render_page(&mut self, frame: &mut Frame) {
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(1),
            ],
        );
        let [context_l, main_l, prompt_l] = layout.areas(frame.area());

        frame.render_widget(self.build_context(), context_l);
        match self.route {
            Route::EntryList => {
                self.render_entry_list(frame, main_l);
            }
            Route::DirectoryList => todo!(),
            Route::Help => todo!(),
        }
        frame.render_widget(self.build_prompt(), prompt_l);
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

enum Focus {
    Preview,
    Prompt,
    List,
    Help,
}

pub async fn run(_settings: &Settings, db: &Database, context: &Context) -> Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let (tx, rx) = mpsc::unbounded_channel();
    let mut app = AppState {
        route: Route::EntryList,
        entry_list: EntryList {
            list: List::new(Vec::new()),
            show_preview: false,
            context: context.clone(),
            filter_mode: FilterMode::Workspace,
            refetch: false,
        },
        directory_list: DirList {
            list: List::new(Vec::new()),
        },
        prompt: PromptState::Default,
        block_focus: BlockFocus::List,
        database: db,
        status: RunningState::Active,
    };
    let mut event_manager = EventManager {
        crossterm: EventStream::new(),
        events: rx,
        dispatch: tx.clone(),
    };

    app.query_entry_list().await?;

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

        if app.entry_list.refetch {
            app.query_entry_list().await?;
            app.entry_list.refetch = false;
        }
        // if app.prompt.source().len() > 0 || matches!(app.keymap_mode, KeymapMode::Insert) {
        //     app.query_list(context).await?
        // }
    }

    tui::restore()?;

    Ok(())
}
