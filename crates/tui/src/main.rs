// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{
            self,
            Event,
            KeyCode,
        },
        execute,
        terminal::{
            EnterAlternateScreen,
            LeaveAlternateScreen,
            disable_raw_mode,
            enable_raw_mode,
        },
    },
    layout::{
        Constraint,
        Layout
    },
    style::{
        Color,
        Modifier,
        Style
    },
    widgets::{
        Block,
        Borders,
        Row,
        TableState,
        Cell,
        Table
    },
};
use std::io;

struct Frame {
    id: u64,
    timestamp: u64,
    timestamp_delta: u64,
    speed: u8,
    rssi: i8,
    channel: u8,
    home_id: u32,
    src_node_id: u16,
    dst_node_id: u16,
    payload: String,
    payload_raw: Vec<u8>,
}

struct App {
    items: Vec<Frame>,
    state: TableState,
}

impl App {
    fn new() -> App {
        let mut state = TableState::default();
        state.select(Some(0));

        // Generate 10,000 items to demonstrate performance with large datasets
        let items: Vec<Frame> = (1..=10000)
            .map(|i| Frame {
                id: i as u64,
                timestamp: i * 100,
                src_node_id: ((i % 232) + 1) as u16, // Node IDs from 1 to 232
                dst_node_id: ((i % 232) + 1) as u16,
                home_id: 0,
                timestamp_delta: 0,
                speed: 0,
                rssi: 0,
                channel: 0,
                payload: String::new(),
                payload_raw: Vec::new(),
            })
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

    fn add(&mut self, item: Frame) {
        self.items.push(item);
        self.state.select(Some(self.items.len() - 1));
    }

    fn start(&mut self) {
        self.add(Frame {
            timestamp: 0,
            src_node_id: 0,
            dst_node_id: 0,
            home_id: 0,
            id: 0,
            timestamp_delta: 0,
            speed: 0,
            rssi: 0,
            channel: 0,
            payload: String::new(),
            payload_raw: Vec::new(),
        });
    }

    fn stop(&mut self) {
        self.add(Frame {
            timestamp: 0,
            src_node_id: 0,
            dst_node_id: 0,
            home_id: 0,
            id: 0,
            timestamp_delta: 0,
            speed: 0,
            rssi: 0,
            channel: 0,
            payload: String::new(),
            payload_raw: Vec::new(),
        });
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
            let items: Vec<Row> = app.items[visible_start..visible_end]
                .iter()
                .map(|item| {
                    Row::new(vec![
                        Cell::from(item.id.to_string()),
                        Cell::from(item.timestamp.to_string()),
                        Cell::from(item.timestamp_delta.to_string()),
                        Cell::from(item.speed.to_string()),
                        Cell::from(item.rssi.to_string()),
                        Cell::from(item.channel.to_string()),
                        Cell::from(item.home_id.to_string()),
                        Cell::from(item.src_node_id.to_string()),
                        Cell::from(item.dst_node_id.to_string()),
                        Cell::from(item.payload.clone()),
                        Cell::from(format!("{:02X?}", item.payload_raw)),
                    ])
                })
                .collect();

            let header = [
                    "ID",
                    "Timestamp",
                    "Timestamp Delta",
                    "Speed",
                    "RSSI",
                    "Channel",
                    "Home ID",
                    "Src Node ID",
                    "Dst Node ID",
                    "Payload",
                    "Payload Raw",
                ]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                //.style(header_style)
                .height(1);

            let list = Table::new(items, &[
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Min(10),
                    Constraint::Min(20),
                ])
                .header(header)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Frames ({}/{})", selected + 1, total_items)))
                .row_highlight_style(
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
                if key.kind == event::KeyEventKind::Press {
                    // The check for key.kind is needed to avoid handling both press and release on Windows.
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::PageDown => app.page_down(page_size),
                        KeyCode::PageUp => app.page_up(page_size),
                        KeyCode::Home => app.go_to_start(),
                        KeyCode::End => app.go_to_end(),
                        KeyCode::Char('n') => app.add(Frame {
                            timestamp: 0,
                            src_node_id: 0,
                            dst_node_id: 0,
                            home_id: 0,
                            id: 0,
                            timestamp_delta: 0,
                            speed: 0,
                            rssi: 0,
                            channel: 0,
                            payload: String::new(),
                            payload_raw: Vec::new(),
                        }),
                        KeyCode::Char('s') => app.start(),
                        KeyCode::Char('S') => app.stop(),
                        _ => {}
                    }
                }
            }
        }
    }
}
