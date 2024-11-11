use crate::tui;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use dirpin_client::database::{Context, Database, FilterMode};
use dirpin_client::domain::Entry;
use dirpin_client::settings::Settings;
use eyre::{Context as EyreContext, Result};
use futures_util::stream::StreamExt;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::prelude::{Buffer, Widget};
use ratatui::style::palette::tailwind::{GRAY, RED, SLATE};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = r#"
    Help: This is the help seciton stuff;
"#;

#[derive(Default, Debug)]
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

    fn set(&mut self, value: &str) {
        self.source.clear();
        self.source.push_str(value);
        self.index = self.source.len();
    }
}

impl From<String> for InputCursor {
    fn from(source: String) -> Self {
        Self { source, index: 0 }
    }
}

#[derive(Debug)]
enum PromptSearchStep {
    Edit,
    Submit,
}

#[derive(Debug)]
struct PromptSearch {
    input: InputCursor,
    show_cursor: bool,
    step: PromptSearchStep,
}

impl PromptSearch {
    fn builder() -> Self {
        Self {
            input: InputCursor::default(),
            show_cursor: true,
            step: PromptSearchStep::Edit,
        }
    }

    fn input(mut self, value: &str) -> Self {
        self.input.set(value);
        self
    }

    fn step(mut self, step: PromptSearchStep) -> Self {
        self.step = step;
        self
    }

    fn set_step(&mut self, step: PromptSearchStep) {
        self.step = step;
    }

    fn cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }

    fn value(&self) -> &str {
        self.input.as_str()
    }
}

// struct PromptDialog {
//     message: String,
//     input: InputCursor,
//     show_cursor: bool,
//     kind:
// }

#[derive(Debug)]
enum InfoKind {
    Error,
    Normal,
}

#[derive(Debug)]
struct PromptInfo {
    value: String,
    kind: InfoKind,
}

impl PromptInfo {
    fn builder() -> Self {
        Self {
            value: String::new(),
            kind: InfoKind::Normal,
        }
    }

    fn set_kind(mut self, kind: InfoKind) -> Self {
        self.kind = kind;
        self
    }

    fn set_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }

    fn value(&self) -> &str {
        self.value.as_str()
    }
}

#[derive(Debug)]
enum ConfirmKind {
    DeleteEntry,
}

#[derive(Debug)]
struct PromptConfirm {
    input: InputCursor,
    kind: ConfirmKind,
}

impl PromptConfirm {
    fn builder() -> Self {
        Self {
            input: InputCursor::default(),
            kind: ConfirmKind::DeleteEntry,
        }
    }

    fn set_kind(mut self, kind: ConfirmKind) -> Self {
        self.kind = kind;
        self
    }

    fn input(mut self, value: &str) -> Self {
        self.input.set(value);
        self
    }

    fn value(&self) -> &str {
        self.input.as_str()
    }
}

#[derive(Debug)]
enum PromptState {
    Default,
    Input,
    Confirm(PromptConfirm),
    Search(PromptSearch),
    Info(PromptInfo),
}

impl PromptState {
    fn prefix(&self) -> Option<String> {
        match self {
            PromptState::Default => None,
            PromptState::Input => Some(": ".into()),
            PromptState::Search(_) => Some("Search: ".into()),
            PromptState::Info(_) => None,
            PromptState::Confirm(_) => None,
        }
    }

    fn value(&self) -> String {
        match self {
            PromptState::Default => "Type : to entr command".into(),
            PromptState::Input => "TODO: input".into(),
            PromptState::Search(s) => s.value().into(),
            PromptState::Info(i) => i.value().into(),
            PromptState::Confirm(confirm) => format!("Are you sure? (y)  {}", confirm.value()),
        }
    }

    fn style(&self) -> Style {
        match self {
            PromptState::Default => Style::new().fg(GRAY.c500),
            PromptState::Input => todo!(),
            PromptState::Search(s) => match s.step {
                PromptSearchStep::Edit => Style::default(),
                PromptSearchStep::Submit => Style::new(),
            },
            PromptState::Info(i) => match i.kind {
                InfoKind::Error => Style::new().fg(RED.c500),
                InfoKind::Normal => Style::default(),
            },
            PromptState::Confirm(_) => Style::default(),
        }
    }

    fn get_search_input(&self) -> Option<&str> {
        match self {
            PromptState::Search(search) => Some(search.value()),
            _ => None,
        }
    }

    fn search() -> Self {
        let search = PromptSearch::builder();
        PromptState::Search(search)
    }

    fn confirm(kind: ConfirmKind) -> Self {
        let confirm = PromptConfirm::builder().set_kind(kind);
        PromptState::Confirm(confirm)
    }

    fn info(value: String) -> Self {
        let info = PromptInfo::builder()
            .set_value(value)
            .set_kind(InfoKind::Normal);
        PromptState::Info(info)
    }

    fn set(&mut self, value: PromptState) {
        *self = value;
    }
}

struct PromptWidget {
    prefix: String,
    value: String,
    style: Style,
}

impl Widget for PromptWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
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

#[derive(Debug)]
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

    fn selected_item(&self) -> &T {
        let index = self.selected();
        &self.items[index]
    }

    fn set_data(&mut self, data: Vec<T>) {
        // TODO: we probably don't want to replace it all the time.
        // Instead just clear and load the data to the same vector?
        self.selected = self.selected.min(data.len() - 1);
        self.items = data;
        self.offset = 0;
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

#[derive(Debug)]
struct EntryList {
    list: List<Entry>,
    show_preview: bool,
    context: Context,
    context_len: i64,
    filter_mode: FilterMode,
    refetch: bool,
}

impl EntryList {
    fn items(&self) -> &[Entry] {
        &self.list.items
    }

    fn set_data(&mut self, data: Vec<Entry>) {
        self.list.set_data(data);
    }

    fn set_count(&mut self, count: i64) {
        self.context_len = count;
    }

    fn set_context_mode(&mut self, next_context: FilterMode) {
        self.filter_mode = next_context;
    }

    fn cycle_context_mode(&mut self) {
        match self.filter_mode {
            FilterMode::Workspace => self.set_context_mode(FilterMode::All),
            FilterMode::Directory => self.set_context_mode(FilterMode::Workspace),
            FilterMode::All => self.set_context_mode(FilterMode::Directory),
        }
    }
}

#[derive(Debug)]
struct DirList {
    list: List<String>,
}

#[derive(Debug, Clone)]
enum BlockFocus {
    List,
    Prompt,
    Debug,
}

impl BlockFocus {
    fn prompt(&mut self) {
        *self = BlockFocus::Prompt
    }
}

#[derive(Debug)]
enum Route {
    EntryList,
    DirectoryList,
    Help,
}

#[derive(Debug)]
enum RunningState {
    Active,
    Quit,
}

#[derive(Debug)]
struct Debug<'a> {
    show: bool,
    return_focus: Option<BlockFocus>,
    scroll_offset: usize,
    settings: &'a Settings,
}

impl<'a> Debug<'a> {
    fn move_down(&mut self, offset: usize) {
        self.scroll_offset += offset;
    }

    fn move_up(&mut self, offset: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(offset);
    }
}

#[derive(Debug)]
enum QueryKind {
    Entries,
    DeleteEntry,
}

#[derive(Debug)]
struct QueryQueue(Vec<QueryKind>);

impl QueryQueue {
    fn push(&mut self, query: QueryKind) {
        self.0.push(query);
    }

    fn pop(&mut self) -> Option<QueryKind> {
        self.0.pop()
    }
}

struct AppState<'a> {
    route: Route,
    entry_list: EntryList,
    directory_list: DirList,
    prompt: PromptState,
    block_focus: BlockFocus,
    query_queue: QueryQueue,
    database: &'a Database,
    status: RunningState,
    debug: Debug<'a>,
}

impl<'a> std::fmt::Debug for AppState<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("route", &self.route)
            .field("entry_list", &self.entry_list)
            .field("prompt", &self.prompt)
            .field("block_focus", &self.block_focus)
            .field("status", &self.status)
            .field("debug", &self.debug)
            .finish()
    }
}

impl AppState<'_> {
    async fn query_entry_list(&mut self) -> Result<()> {
        let filter_mode = [self.entry_list.filter_mode.clone()];
        let search = self.prompt.get_search_input().unwrap_or("");
        let data = self
            .database
            .list(&filter_mode, &self.entry_list.context, search)
            .await?;
        let context_count = self
            .database
            .count(&filter_mode, &self.entry_list.context, search)
            .await?;
        self.entry_list.set_data(data);
        self.entry_list.set_count(context_count as i64);

        Ok(())
    }

    async fn query_delete(&mut self) -> Result<()> {
        let entry = self.entry_list.list.selected_item();
        self.database.delete(entry.id.clone()).await?;
        self.query_queue.push(QueryKind::Entries);
        self.prompt.set(PromptState::info("Entry deleted!".into()));

        Ok(())
    }

    async fn query_save(&mut self, item: &Entry) -> Result<()> {
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

    fn set_focus(&mut self, focus: BlockFocus) {
        self.block_focus = focus;
    }

    fn set_prompt(&mut self, prompt: PromptState) {
        self.prompt = prompt;
    }

    fn handle_prompt_search_exit(&mut self) {
        self.block_focus = BlockFocus::List;
        self.prompt.set(PromptState::Default);
        self.query_queue.push(QueryKind::Entries);
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

    fn handle_debug_toggle(&mut self, event: &KeyEvent) -> bool {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);

        match event.code {
            KeyCode::Char('d') if ctrl => {
                if self.debug.show {
                    self.block_focus = self.debug.return_focus.clone().unwrap_or(BlockFocus::List);
                    self.debug.show = false;
                } else {
                    self.debug.return_focus = Some(self.block_focus.clone());
                    self.debug.show = true;
                    self.block_focus = BlockFocus::Debug;
                }
                true
            }
            _ => false,
        }
    }

    async fn handle_key_input(&mut self, key_event: KeyEvent) -> Option<Event> {
        let mut event = None;
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);

        // Handle global exit
        if self.handle_global_exit(&key_event) {
            return event;
        }

        if self.handle_debug_toggle(&key_event) {
            return event;
        }

        // Handle prompt events
        event = match self.block_focus {
            BlockFocus::List => match self.route {
                Route::EntryList => {
                    match self.prompt {
                        PromptState::Search(_) => match key_event.code {
                            KeyCode::Esc => {
                                self.handle_prompt_search_exit();
                                return None;
                            }
                            _ => {}
                        },
                        _ => {}
                    };

                    match key_event.code {
                        KeyCode::Char('j') => self.entry_list.list.move_down(),
                        KeyCode::Char('k') => self.entry_list.list.move_up(),
                        KeyCode::Char('f') if ctrl => {
                            self.entry_list.cycle_context_mode();
                            self.query_queue.push(QueryKind::Entries);
                        }
                        KeyCode::Char('/') => {
                            self.set_prompt(PromptState::search());
                            self.set_focus(BlockFocus::Prompt);
                        }
                        KeyCode::Char('d') => {
                            self.set_prompt(PromptState::confirm(ConfirmKind::DeleteEntry));
                            self.set_focus(BlockFocus::Prompt);
                        }
                        _ => {}
                    }

                    None
                }
                Route::DirectoryList => None,
                _ => None,
            },
            BlockFocus::Prompt => match &mut self.prompt {
                PromptState::Search(search) => match search.step {
                    PromptSearchStep::Edit => match key_event.code {
                        KeyCode::Char('f') if ctrl => {
                            self.entry_list.cycle_context_mode();
                            None
                        }
                        KeyCode::Char(c) => {
                            search.input.insert(c);
                            self.query_queue.push(QueryKind::Entries);
                            None
                        }
                        KeyCode::Backspace => {
                            search.input.remove();
                            self.query_queue.push(QueryKind::Entries);
                            None
                        }
                        KeyCode::Enter => {
                            self.block_focus = BlockFocus::List;
                            search.set_step(PromptSearchStep::Submit);
                            None
                        }
                        KeyCode::Esc => {
                            self.handle_prompt_search_exit();
                            None
                        }
                        _ => None,
                    },
                    _ => None,
                },
                PromptState::Input => todo!("handle prompt input"),
                PromptState::Confirm(confirm) => match key_event.code {
                    KeyCode::Char(c) => {
                        confirm.input.insert(c);
                        None
                    }
                    KeyCode::Backspace => {
                        confirm.input.remove();
                        None
                    }
                    KeyCode::Enter => {
                        self.block_focus = BlockFocus::List;
                        match confirm.value() {
                            "y" => {
                                self.query_queue.push(QueryKind::DeleteEntry);
                                self.prompt.set(PromptState::Default);
                            }
                            "n" => {
                                self.prompt.set(PromptState::Default);
                            }
                            _ => {
                                self.prompt
                                    .set(PromptState::info("Only 'y' or 'n' are allowed!".into()));
                            }
                        }
                        None
                    }
                    KeyCode::Esc => {
                        self.block_focus = BlockFocus::List;
                        self.prompt.set(PromptState::Default);
                        None
                    }
                    _ => None,
                },
                _ => None,
            },
            BlockFocus::Debug => {
                match key_event.code {
                    KeyCode::Char('j') => self.debug.move_down(1),
                    KeyCode::Char('k') => self.debug.move_up(1),
                    _ => {}
                }
                None
            }
        };

        // event = match self.route {
        //     Route::EntryList => match self.block_focus {
        //         BlockFocus::List => handle_active_entry_list(self, &key_event),
        //         BlockFocus::Prompt => match self.prompt {
        //             PromptState::Search(_) => handle_prompt_search_entry_list(self, &key_event),
        //             PromptState::Input => todo!("handle prompt input"),
        //             _ => None,
        //         },
        //     },
        //     Route::DirectoryList => todo!(),
        //     Route::Help => todo!(),
        // };

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
        // NOTE: This is set basd on the long set word as hardcoded value for now!
        let max_len = format!("workspace").len();
        let context_value = self.entry_list.filter_mode.as_str();
        let padding = (max_len - context_value.len()) / 2;
        let formatted = format!(
            "[ {:^width$} ]",
            context_value,
            width = context_value.len() + padding * 2
        );

        let context_target = match self.entry_list.filter_mode {
            FilterMode::All => &self.entry_list.context.hostname,
            FilterMode::Directory => &self.entry_list.context.cwd,
            FilterMode::Workspace => {
                let value = self.entry_list.context.cgd.as_ref().map(|x| x.as_str());
                value.unwrap_or_else(|| "Not available")
            }
        };

        Paragraph::new(Line::from(vec![
            Span::styled(formatted, Style::new().fg(GRAY.c500)),
            Span::raw("  "),
            Span::raw(context_target),
            Span::styled(
                format!("  ({})", self.entry_list.context_len),
                Style::new().fg(GRAY.c500),
            ),
        ]))
    }

    fn render_entry_list(&self, frame: &mut Frame, rect: Rect) {
        // TODO: This is not ideal but I am too tired this evening to think
        // about how to set this up. We create a new state for each rerender.
        let state = &self.entry_list.list;
        let height = rect.height as usize;

        let lines = self
            .entry_list
            .items()
            .iter()
            .skip(state.offset)
            .take(height)
            .map(|x| {
                let context = match self.entry_list.filter_mode {
                    FilterMode::All => x
                        .cgd
                        .as_ref()
                        .map(|x| x.split("/").last().unwrap())
                        .unwrap_or(&x.cwd)
                        .to_string(),
                    FilterMode::Directory => "".to_string(),
                    FilterMode::Workspace => {
                        x.cwd.replace(&self.entry_list.context.cwd, "").to_string()
                    }
                };
                Line::from(vec![
                    Span::styled("[   note    ]  ", Style::new().fg(GRAY.c500)),
                    Span::raw(x.value.as_str()),
                    Span::styled(format!("  {}", context), Style::new().fg(GRAY.c500)),
                ])
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
                let constraints = if self.entry_list.show_preview {
                    vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]
                } else {
                    vec![Constraint::Min(1), Constraint::Max(0)]
                };
                let layout_main = Layout::new(Direction::Horizontal, constraints);
                let [list_l, preview_l] = layout_main.areas(rect);

                for (i, line) in lines.into_iter().enumerate() {
                    let style = if self.entry_list.list.selected() == i {
                        Style::new().bg(GRAY.c800)
                    } else {
                        Style::new()
                    };

                    let item_rect = Rect::new(list_l.x, list_l.y + i as u16, list_l.width, 1);
                    let paragraph = Paragraph::new(line).style(style);
                    frame.render_widget(paragraph, item_rect);
                }

                // if self.entry_list.show_preview {
                //     let layout_main = Layout::new(
                //         Direction::Horizontal,
                //         vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)],
                //     );
                //     let [list_l, preview_l] = layout_main.areas(rect);
                //
                //     frame.render_widget(content, list_l);
                //     // frame.render_widget(preview, preview_l);
                // } else {
                //     frame.render_widget(content, rect);
                // }
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
            prefix: self.prompt.prefix().unwrap_or("".into()),
            value: self.prompt.value(),
            style: self.prompt.style(),
        }
    }

    fn build_debug(&self, height: u16) -> Paragraph {
        let content = format!("{:#?}", self)
            .lines()
            .skip(self.debug.scroll_offset)
            .take(height as usize - 2)
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(content).block(Block::default().borders(Borders::all()).title("Debug"))
    }

    fn render_page(&mut self, frame: &mut Frame) {
        let layout = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ],
        );
        let [context_l, sapcer_l, main_l, prompt_l] = layout.areas(frame.area());

        frame.render_widget(self.build_context(), context_l);
        frame.render_widget(
            Paragraph::new(Line::raw("-".repeat(sapcer_l.width.into())))
                .style(Style::new().fg(GRAY.c500)),
            sapcer_l,
        );
        match self.route {
            Route::EntryList => {
                self.render_entry_list(frame, main_l);
            }
            Route::DirectoryList => todo!(),
            Route::Help => todo!(),
        }
        frame.render_widget(self.build_prompt(), prompt_l);
        self.set_prompt_cursor(frame, prompt_l);

        if self.debug.show {
            let rect = build_modal_block(frame.area());
            frame.render_widget(Clear, rect);
            let debug = self.build_debug(rect.height);
            frame.render_widget(debug, rect);
        }
    }

    fn set_prompt_cursor(&self, frame: &mut Frame, rect: Rect) {
        match self.block_focus {
            BlockFocus::Prompt => match self.prompt {
                PromptState::Search(ref s) => {
                    if s.show_cursor {
                        let len = self.prompt.prefix().unwrap_or("".into()).len()
                            + self.prompt.value().len();
                        frame.set_cursor_position(Position::new(rect.x + (len as u16), rect.y));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn build_modal_block(rect: Rect) -> Rect {
    let vertical = Layout::new(
        Direction::Vertical,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ],
    );
    let [_, main, _] = vertical.areas(rect);
    let horizontal = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ],
    );
    let [_, block, _] = horizontal.areas(main);
    block
}

// fn build_help_modal() {}

fn ev_key_press(ev: &CrosstermEvent) -> Option<&KeyEvent> {
    match ev {
        CrosstermEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => Some(key_event),
        _ => None,
    }
}

enum Event {
    KeyInput(KeyEvent),
    TerminalRepaint,
    TerminalTick,
}

struct EventManager {
    crossterm: EventStream,
    events: mpsc::UnboundedReceiver<Event>,
    dispatch: mpsc::UnboundedSender<Event>,
}

pub async fn run(settings: &Settings, db: &Database, context: &Context) -> Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let (tx, rx) = mpsc::unbounded_channel();
    let mut app = AppState {
        route: Route::EntryList,
        entry_list: EntryList {
            list: List::new(Vec::new()),
            show_preview: false,
            context: context.clone(),
            context_len: 0,
            filter_mode: FilterMode::Directory,
            refetch: false,
        },
        directory_list: DirList {
            list: List::new(Vec::new()),
        },
        prompt: PromptState::Default,
        block_focus: BlockFocus::List,
        query_queue: QueryQueue(Vec::new()),
        database: db,
        status: RunningState::Active,
        debug: Debug {
            show: false,
            return_focus: None,
            scroll_offset: 0,
            settings,
        },
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

        while let Some(query) = app.query_queue.pop() {
            match query {
                QueryKind::Entries => {
                    app.query_entry_list().await?;
                }
                QueryKind::DeleteEntry => {
                    app.query_delete().await?;
                }
            }
        }
        //
        // if app.entry_list.refetch {
        //     app.query_entry_list().await?;
        //     app.entry_list.refetch = false;
        // }
        // if app.prompt.source().len() > 0 || matches!(app.keymap_mode, KeymapMode::Insert) {
        //     app.query_list(context).await?
        // }
    }

    tui::restore()?;

    Ok(())
}
