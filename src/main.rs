#![feature(iter_advance_by)]

use std::error::Error;
use app::App;
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen}, execute, event::{Event, KeyCode, self}};
use paths::Paths;
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod paths;
mod backup;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.hide_cursor().unwrap();

    let mut app = App::new();
    
    loop {
        while app.backup.receive_log.is_some() {
            terminal.draw(|f| app.render(f))?;
        }
        terminal.draw(|f| app.render(f))?;
        let key = if let Event::Key(key) = event::read()? { key } else { continue };
        match app.current_tab {
            0 => match key.code {
                KeyCode::Char('q') | KeyCode::Char('c') => return Ok(()),
                KeyCode::Char('w') | KeyCode::Up => app.paths.scroll_up(),
                KeyCode::Char('a') | KeyCode::Left => app.previous(),
                KeyCode::Char('s') | KeyCode::Down => app.paths.scroll_down(),
                KeyCode::Char('d') | KeyCode::Right => app.next(),
                _ => {}
            },
            1 => if app.paths.capturing_input.is_some() {
                match key.code {
                    KeyCode::Char(c) => app.paths.capturing_input.as_mut().unwrap().push(c),
                    KeyCode::Backspace => {app.paths.capturing_input.as_mut().unwrap().pop();},
                    KeyCode::Enter => app.paths.add_new(),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c') => return Ok(()),
                    KeyCode::Char('w') | KeyCode::Up => app.paths.scroll_up(),
                    KeyCode::Char('a') | KeyCode::Left => app.previous(),
                    KeyCode::Char('s') | KeyCode::Down => app.paths.scroll_down(),
                    KeyCode::Char('d') | KeyCode::Right => app.next(),
                    KeyCode::Char('n') => if app.current_tab == 1 {
                        app.paths.dialog_add_new()
                    },
                    KeyCode::Char('r') => app.paths.delete_selected(),
                    KeyCode::Char('f') => app.paths = Paths::read(),
                    _ => {}
                }
            },
            2 => if app.backup.repo_url.is_none() {
                match key.code {
                    KeyCode::Char(c) => app.backup.text_input.push(c),
                    KeyCode::Backspace => {app.backup.text_input.pop();},
                    KeyCode::Enter => app.backup.set_repo_url(),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c') => return Ok(()),
                    KeyCode::Char('w') | KeyCode::Up => app.paths.scroll_up(),
                    KeyCode::Char('a') | KeyCode::Left => app.previous(),
                    KeyCode::Char('s') | KeyCode::Down => app.paths.scroll_down(),
                    KeyCode::Char('d') | KeyCode::Right => app.next(),
                    KeyCode::Char('r') => if !app.backup.downloading {
                        app.backup.restore(app.paths.paths.clone())
                    },
                    KeyCode::Enter | KeyCode::Char('e') => if app.backup.uploading {
                        app.backup.uploading = false
                    } else if app.backup.downloading {
                        app.backup.downloading = false
                    } else {
                        app.backup.backup(app.paths.paths.clone())
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }
}
