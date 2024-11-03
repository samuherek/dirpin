use crate::database::{Context, Database, FilterMode};
use crate::domain::Pin;
use crate::settings::Settings;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use eyre::{Context as EyreContext, Result};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::prelude::{Buffer, Widget};
use ratatui::style::palette::tailwind::{GRAY, SLATE, YELLOW};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidget};
use ratatui::Frame;

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

enum LineMode {
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

struct LineState {
    input: InputCursor,
    show_cursor: bool,
    mode: LineMode,
}

impl LineState {
    fn source(&self) -> &str {
        self.input.as_str()
    }
}

struct AppState {
    page: PageState,
    results_state: ListState,
    keymap_mode: KeymapMode,
    line: LineState,
    results: Vec<Pin>,
    context: Context,
    running: RunningState,
}

impl AppState {
    async fn query_list(&mut self, db: &Database, ctx: &Context) -> Result<()> {
        self.results = db
            .list(&[FilterMode::Workspace], ctx, self.line.source())
            .await?;
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

    fn handle_search_down(&mut self) {
        let i = self.results_state.selected() + 1;
        self.results_state
            .select(i.min(self.results.len().saturating_sub(1)));
    }

    fn handle_search_up(&mut self) {
        let i = self.results_state.selected().saturating_sub(1);
        self.results_state.select(i);
    }

    fn handle_keymap_mode(&mut self, mode: KeymapMode) {
        match mode {
            KeymapMode::Normal => {
                self.line.input.clear();
                self.line.show_cursor = false;
            }
            KeymapMode::Insert => self.line.show_cursor = true,
        }
        self.keymap_mode = mode;
    }

    fn handle_line_input(&mut self, ev: &KeyEvent) {
        match ev.code {
            KeyCode::Char(c) => self.line.input.insert(c),
            KeyCode::Backspace => self.line.input.remove(),
            _ => {}
        }
    }

    // TODO: implement a sequence of keys
    fn handle_key_events(&mut self, ev: &KeyEvent) {
        let ctrl = ev.modifiers.contains(KeyModifiers::CONTROL);

        // Global events
        match ev.code {
            KeyCode::Char('c') if ctrl => self.quit(),
            _ => {}
        }

        // TODO: Ignore tmux ctrl+a

        match self.keymap_mode {
            KeymapMode::Normal => match ev.code {
                KeyCode::Char('j') => self.handle_search_down(),
                KeyCode::Char('k') => self.handle_search_up(),
                KeyCode::Char('i') => self.handle_keymap_mode(KeymapMode::Insert),
                _ => {}
            },
            KeymapMode::Insert => match ev.code {
                KeyCode::Esc => self.handle_keymap_mode(KeymapMode::Normal),
                _ => self.handle_line_input(ev),
            },
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
            KeymapMode::Insert => Line::from(Span::raw(format!(" {}", self.line.source()))),
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
        let layout_line = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Length(10), Constraint::Min(0)],
        );
        let [header_l, context_l, main_l, line_l] = layout.areas(frame.size());
        let [title_l, help_l] = layout_header.areas(header_l);
        let [mode_l, input_l] = layout_line.areas(line_l);

        let title = self.build_title();
        let help = self.build_help();
        let context = self.build_context();
        let content = match self.page {
            PageState::List => self.build_list(),
        };
        let mode = self.build_mode();
        let input = self.build_input();
        // TODO: This is not idea but I am too tired this evening to think
        // about how to set this up.
        let mut state = self.results_state.clone();

        frame.render_widget(title, title_l);
        frame.render_widget(help, help_l);
        frame.render_widget(context, context_l);
        frame.render_stateful_widget(content, main_l, &mut state);
        frame.render_widget(mode, mode_l);
        frame.render_widget(input, input_l);

        if self.line.show_cursor {
            frame.set_cursor_position(Position::new(
                input_l.x + (self.line.source().len() as u16) + 1,
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

pub async fn run(_settings: &Settings, db: &Database, context: &Context) -> Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut app = AppState {
        page: PageState::List,
        results_state: ListState::default(),
        keymap_mode: KeymapMode::Normal,
        line: LineState {
            input: InputCursor::from("".to_string()),
            show_cursor: false,
            mode: LineMode::Info,
        },
        results: vec![],
        running: RunningState::Active,
        context: context.clone(),
    };
    let mut event = EventStream::new();

    app.query_list(db, context).await?;

    while app.running() {
        terminal
            .draw(|frame| app.render_page(frame))
            .wrap_err("failed to render terminal")?;

        match event.next().await {
            Some(Ok(ev)) => {
                if let Some(key_ev) = ev_key_press(&ev) {
                    app.handle_key_events(key_ev);
                }
            }
            _ => {}
        }

        if app.line.source().len() > 0 || matches!(app.keymap_mode, KeymapMode::Insert) {
            app.query_list(db, context).await?
        }
    }

    tui::restore()?;

    Ok(())
}
