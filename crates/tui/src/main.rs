// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::panic;
use tracing::error;
use tokio;

use clap::Parser;
use std::fs::File;
use zniff_rs_core::zlf::{
    ZlfReader,
    ZlfRecord,
};
use zniff_rs_core::zniffer_parser;
use zniff_rs_core::storage::{FrameDatabase, SqliteFrameDatabase, DbFrame};

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
        Layout,
        Rect,
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
        Table,
        Paragraph,
        Wrap,
        Clear,
    },
};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppMode {
    Normal,
    Detail,
}

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
    items: Vec<u128>,
    state: TableState,
    mode: AppMode,
    db: SqliteFrameDatabase,
}

impl App {
    fn try_new(file: File) -> Result<App, bool> {
        let mut state = TableState::default();
        state.select(Some(0));

        let mut zlf_reader = ZlfReader::new(file).expect("Failed to create ZLF reader");

        let db = SqliteFrameDatabase::new();

        let mut items: Vec<u128> = Vec::new();
        let mut frame_id: u128 = 1;

        let mut zniffer_parser = zniffer_parser::Parser::new();

        zlf_reader.read_frames(|rec| {
            match rec {
                ZlfRecord::Other(raw) => {
                    for byte in raw.payload.iter() {
                        let result = zniffer_parser.parse(*byte);

                        match result {
                            zniffer_parser::ParserResult::ValidFrame { frame } => {
                                items.push(frame_id);

                                let db_frame = DbFrame {
                                    id: frame_id as i64, // You can generate or extract an ID for the frame
                                    timestamp: frame.timestamp as i64, // You can extract this from the frame if needed
                                    speed: frame.speed,     // You can extract this from the frame if needed
                                    rssi: frame.rssi as i8,      // You can extract this from the frame if needed
                                    channel: frame.channel,   // You can extract this from the frame if needed
                                    home_id: 0x12345678, // Example home_id, replace with actual value if available
                                    src_node_id: 1, // Example src_node_id, replace with actual value if available
                                    dst_node_id: 2, // Example dst_node_id, replace with actual value if available
                                    payload: frame.payload.clone(), // Use the raw payload from the parsed frame
                                };

                                //println!("Insert frame");
                                db.add_frame(db_frame);
                                frame_id += 1;
                            },
                            _ => {
                                // Don't care about other parser results than a valid frame for now.
                            },
                        }
                    }
                },
                _ => {
                    // Don't care about other record types for now.
                }
            }
        }).expect("Failed to read frames from ZLF file");

        Ok(App {
            items,
            state,
            mode: AppMode::Normal,
            db
        })
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

    fn add(&mut self, _item: Frame) {
        // TODO: Modify this function to actually add the frame to the database and update the items list accordingly.
        //self.items.push(item);
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

    fn toggle_detail(&mut self) {
        self.mode = match self.mode {
            AppMode::Normal => AppMode::Detail,
            AppMode::Detail => AppMode::Normal,
        };
    }

    fn handle_key_event(&mut self, key: KeyCode, page_size: usize) -> io::Result<bool> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode_key(key, page_size),
            AppMode::Detail => self.handle_detail_mode_key(key),
        }
    }

    fn handle_normal_mode_key(&mut self, key: KeyCode, page_size: usize) -> io::Result<bool> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => Ok(true), // Signal to exit
            KeyCode::Enter => {
                self.toggle_detail();
                Ok(false)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous();
                Ok(false)
            }
            KeyCode::PageDown => {
                self.page_down(page_size);
                Ok(false)
            }
            KeyCode::PageUp => {
                self.page_up(page_size);
                Ok(false)
            }
            KeyCode::Home => {
                self.go_to_start();
                Ok(false)
            }
            KeyCode::End => {
                self.go_to_end();
                Ok(false)
            }
            KeyCode::Char('n') => {
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
                Ok(false)
            }
            KeyCode::Char('s') => {
                self.start();
                Ok(false)
            }
            KeyCode::Char('S') => {
                self.stop();
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn handle_detail_mode_key(&mut self, key: KeyCode) -> io::Result<bool> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                self.toggle_detail();
                Ok(false)
            }
            // Could add navigation between frames in detail view here
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous();
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

#[derive(Parser)]
#[command(name = "zniff-rs-tui")]
#[command(about = "zniff-rs-tui is a tool for sniffing, parsing and converting Z-Wave data.", long_about = None)]
struct Cli {
    /// Path to the ZLF file to read frames from.
    #[arg(short, long)]
    trace: String,
}

fn install_panic_hook() {
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // TODO: restore terminal raw mode here if necessary
        error!(?info, "panic occurred");
        default(info);
    }));
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {

    install_panic_hook();
    let cli = Cli::parse();

    let file = File::open(&cli.trace)?;

    // Create app state
    let mut app = match App::try_new(file) {
        Ok(app) => app,
        Err(_) => {
            eprintln!("Failed to create app");
            return Ok(());
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

            let visible_count = visible_end - visible_start;

            let frames = app.db.get_frames(visible_start, visible_count);

            let mut previous_timestamp: Option<i64> = None;

            let items: Vec<Row> = frames.iter().map(|frame| {
                let timestamp_delta = if let Some(prev) = previous_timestamp {
                    frame.timestamp - prev
                } else {
                    0
                };
                previous_timestamp = Some(frame.timestamp);

                let home_id = 0x12345678; // Example home_id, replace with actual value if available
                let src_node_id = 1; // Example src_node_id, replace with actual value if available
                let dst_node_id = 2; // Example dst_node_id, replace with actual value

                // Create string with the raw hex data of the payload.
                let payload_hex = format!("{:02X?}", frame.payload);

                Row::new(vec![
                    Cell::from(frame.id.to_string()),
                    Cell::from(frame.timestamp.to_string()),
                    Cell::from(timestamp_delta.to_string()),
                    Cell::from(frame.speed.to_string()),
                    Cell::from(frame.rssi.to_string()),
                    Cell::from(frame.channel.to_string()),
                    Cell::from(format!("0x{:08X}", home_id)),
                    Cell::from(src_node_id.to_string()),
                    Cell::from(dst_node_id.to_string()),
                    Cell::from(payload_hex.clone()), // This is supposed to be the parsed payload.
                    Cell::from(payload_hex),
                ])
            }).collect::<Vec<Row>>();

            let header = [
                    "ID",
                    "Timestamp",
                    "ΔTimestamp",
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
                    Constraint::Length(5),
                    Constraint::Length(15),
                    Constraint::Length(15),
                    Constraint::Length(6),
                    Constraint::Length(6),
                    Constraint::Length(7),
                    Constraint::Length(15),
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

            // Render detail popup if in detail mode
            if app.mode == AppMode::Detail {
                if let Some(selected_idx) = app.state.selected() {
                    if let Some(frame_id) = app.items.get(selected_idx) {
                        render_detail_popup(f, *frame_id, &app);
                    }
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let page_size = terminal.size()?.height.saturating_sub(3) as usize;
                if key.kind == event::KeyEventKind::Press {
                    // The check for key.kind is needed to avoid handling both press and release on Windows.
                    let should_exit = app.handle_key_event(key.code, page_size)?;
                    if should_exit {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn render_detail_popup(f: &mut ratatui::Frame, frame_id: u128, app: &App) {
    // Create a centered rectangle (70% width, 70% height)
    let area = centered_rect(70, 70, f.area());

    // Fetch the frame details from the database using the frame_id
    let frame = match app.db.get_frame(frame_id as u64) {
        Some(frame) => frame,
        None => DbFrame { id: 0, channel: 0, speed: 0, timestamp: 0, rssi: 0, home_id: 0, src_node_id: 0, dst_node_id: 0, payload: vec![] },
    };

    // Format the detailed information
    let detail_text = format!(
        "Frame Details\n\n\
        ID:               {}\n\
        Timestamp:        {}\n\
        Timestamp Delta:  {}\n\
        Speed:            {}\n\
        RSSI:             {} dBm\n\
        Channel:          {}\n\
        Home ID:          0x{:08X}\n\
        Source Node ID:   {}\n\
        Dest Node ID:     {}\n\
        Payload:          {}\n\
        Payload Raw:      {:02X?}\n\n\
        Press Enter or Esc to close",
        frame.id,
        frame.timestamp,
        0, //frame.timestamp_delta,
        frame.speed,
        frame.rssi,
        frame.channel,
        0x12345678, //frame.home_id,
        1, //frame.src_node_id,
        2, //frame.dst_node_id,
        "frame.payload",
        frame.payload, //frame.payload_raw
    );

    let paragraph = Paragraph::new(detail_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Frame Details")
                .style(Style::default().bg(Color::Black))
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White).bg(Color::Black));

    // Clear the area and render the popup
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
