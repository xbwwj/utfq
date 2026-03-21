use std::{collections::HashMap, fs::read_to_string, path::absolute};

use chrono::{Days, NaiveDate};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat},
};
use either::Either;
use hyperrat::Link;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, KeyCode},
    layout::Rect,
    style::{Style, Stylize},
    text::Span,
};
use url::Url;

use crate::{cli::Cli, parse::parse_file, walk::build_walk_filtered};

pub struct App {
    cli: Cli,
    is_running: bool,
    date: NaiveDate,
    lines: Vec<Either<String, Link<'static>>>,
    offset: usize,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        Self {
            date: cli.date,
            cli,
            is_running: true,
            lines: Default::default(),
            offset: Default::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.reload()?;
        while self.is_running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event()?;
        }

        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        for (i, w) in (0..area.height).zip(self.lines.iter().skip(self.offset)) {
            match w {
                Either::Left(string) => {
                    frame.render_widget(
                        Span::from(string),
                        Rect {
                            x: area.x,
                            y: area.y + i,
                            width: area.width,
                            height: 1,
                        },
                    );
                }
                Either::Right(link) => {
                    frame.render_widget(
                        // PERF: but &Link does not impl widget
                        link.clone(),
                        Rect {
                            x: area.x,
                            y: area.y + i,
                            width: area.width,
                            height: 1,
                        },
                    );
                }
            }
        }
        if area.width > 10 {
            frame.render_widget(
                Span::from(self.date.to_string()).reversed(),
                Rect {
                    x: area.x + area.width - 10,
                    y: area.y,
                    width: 10,
                    height: 1,
                },
            );
        }
    }

    pub fn handle_event(&mut self) -> Result<()> {
        if let event::Event::Key(key_event) = event::read().context("event poll failed")? {
            match key_event.code {
                KeyCode::Char('q') => self.is_running = false,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.offset > 0 {
                        self.offset -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.offset += 1;
                }
                KeyCode::Char('r') => {
                    self.reload()?;
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.date = self
                        .date
                        .checked_sub_days(Days::new(1))
                        .context("date out of range")?;
                    self.reload()?;
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.date = self
                        .date
                        .checked_add_days(Days::new(1))
                        .context("date out of range")?;
                    self.reload()?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn reload(&mut self) -> Result<()> {
        let mut collected = HashMap::new();

        for result in build_walk_filtered() {
            match result {
                Ok(entry) => {
                    // only handle file
                    let path = entry.path();
                    let Ok(string) = read_to_string(path) else {
                        continue;
                    };
                    let new_items = parse_file(&self.cli, self.date, &string);
                    collected.insert(path.to_path_buf(), new_items);
                }
                Err(err) => eprintln!("ERROR: {}", err),
            }
        }

        self.lines.clear();

        let mut keys: Vec<_> = collected.keys().collect();
        keys.sort();

        for path in keys {
            let items = collected.get(path).unwrap();
            if items.is_empty() {
                continue;
            }
            let relative_path = path.strip_prefix(".").unwrap_or(path);
            let url = Url::from_file_path(absolute(path).unwrap()).unwrap();

            self.lines.push(Either::Right(
                Link::new(relative_path.display().to_string(), url.to_string())
                    .style(Style::default().bold()),
            ));

            for item in items {
                self.lines.push(Either::Left(format!("  {}", item.trim())));
            }
        }

        Ok(())
    }
}
