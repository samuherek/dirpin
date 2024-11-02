use crate::domain::Pin;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use eyre::{Context, Result};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::{SLATE, GRAY};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
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

struct AppState {
    page: PageState,
    result_list: Vec<Pin>,
    running: RunningState,
}

impl AppState {
    fn new() -> Self {
        Self {
            page: PageState::List,
            result_list: vec![],
            running: RunningState::Active,
        }
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

    fn build_title(&self) -> Paragraph {
        Paragraph::new(format!("Dirpin {VERSION}"))
    }

    fn build_help(&self) -> Paragraph {
        Paragraph::new(Line::from(vec![
            Span::styled("<ctrl-c>", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(": exit"),
        ]))
        .alignment(Alignment::Right)
        .style(Style::new().fg(GRAY.c500))
    }

    fn build_list(&self) -> Paragraph {
        Paragraph::new("Content")
    }

    fn render_page(&self, frame: &mut Frame) {
        let length = 1;
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(length),
            ],
        );
        let [header_l, line_l, main_l] = layout.areas(frame.size());
        let layout_header = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)],
        );
        let [title_l, help_l] = layout_header.areas(header_l);
        let title = self.build_title();
        let help = self.build_help();
        let content = match self.page {
            PageState::List => self.build_list(),
        };
        frame.render_widget(title, title_l);
        frame.render_widget(help, help_l);
        frame.render_widget(Paragraph::new(""), line_l);
        frame.render_widget(content, main_l);
    }
}

pub async fn run() -> Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut app = AppState::new();
    let mut event = EventStream::new();

    while app.running() {
        terminal
            .draw(|frame| app.render_page(frame))
            .wrap_err("failed to render terminal")?;

        match event.next().await {
            Some(Ok(ev)) => match ev {
                CrosstermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
                        _ => {}
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    tui::restore()?;

    Ok(())
}
