// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};
use std::io;

struct App {
    items: Vec<String>,
    state: ListState,
}

impl App {
    fn new() -> App {
        let mut state = ListState::default();
        state.select(Some(0));

        // Generate 10,000 items to demonstrate performance with large datasets
        let items: Vec<String> = (1..=10000)
            .map(|i| format!("Z-Wave Frame {}: [Timestamp: {}ms, NodeID: {}, Command: 0x{:02X}]",
                i, i * 100, (i % 232) + 1, i % 256))
            .collect();

        App {
            items,
            state,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn page_down(&mut self, page_size: usize) {
        let i = match self.state.selected() {
            Some(i) => (i + page_size).min(self.items.len() - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn page_up(&mut self, page_size: usize) {
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(page_size),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn go_to_start(&mut self) {
        self.state.select(Some(0));
    }

    fn go_to_end(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    fn add(&mut self, item: String) {
        self.items.push(item);
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Run the app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(f.area());

            // Calculate visible range for virtual scrolling
            let area_height = chunks[0].height.saturating_sub(2) as usize; // Subtract borders
            let selected = app.state.selected().unwrap_or(0);
            let total_items = app.items.len();

            // Calculate the window of items to display
            let visible_start = selected.saturating_sub(area_height / 2);
            let visible_end = (visible_start + area_height).min(total_items);
            let visible_start = visible_start.min(total_items.saturating_sub(area_height));

            // Only create ListItems for visible range
            let items: Vec<ListItem> = app.items[visible_start..visible_end]
                .iter()
                .map(|item| ListItem::new(item.as_str()))
                .collect();

            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Scrollable List ({}/{})", selected + 1, total_items)))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightBlue)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            // Adjust the state offset for the visible window
            let mut adjusted_state = app.state.clone();
            adjusted_state.select(Some(selected - visible_start));

            f.render_stateful_widget(list, chunks[0], &mut adjusted_state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let page_size = terminal.size()?.height.saturating_sub(3) as usize;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::PageDown => app.page_down(page_size),
                    KeyCode::PageUp => app.page_up(page_size),
                    KeyCode::Home => app.go_to_start(),
                    KeyCode::End => app.go_to_end(),
                    KeyCode::Char('n') => app.add(format!("New Item {}", app.items.len() + 1)),
                    _ => {}
                }
            }
        }
    }
}
