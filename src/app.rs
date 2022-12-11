use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    widgets::Paragraph,
    Frame, text::{Span, Spans}, style::{Style, Color}
};

use crate::{paths::Paths, backup::Backup};

pub struct App {
    pub tabs: Vec<&'static str>,
    pub current_tab: usize,
    pub paths: Paths,
    pub backup: Backup
}
impl App {
    pub fn new() -> Self {
        Self {
            tabs: vec!["Menu","Paths","Backup"],
            current_tab: 0,
            paths: Paths::read(),
            backup: Backup::new()
        }
    }
    pub fn next(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tabs.len()
    }
    pub fn previous(&mut self) {
        if self.current_tab > 0 { self.current_tab -= 1 }
        else { self.current_tab = self.tabs.len() - 1 }
    }
    pub fn topbar(&self, f: &mut Frame<impl Backend>, area: Rect) {
        let mut constraints = Vec::new();
        let size = (100 / self.tabs.len()) as u16;
        for _ in 0..self.tabs.len() {
            constraints.push(Constraint::Percentage(size))
        }
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .vertical_margin(1)
            .constraints(constraints)
            .split(area);
        for (i, tab) in self.tabs.iter().enumerate() {
            const C: u8 = 50;
            let color = if i == self.current_tab { Color::Rgb(C, C, C) } else { Color::Reset };
            f.render_widget(
                Paragraph::new(Span::raw(*tab))
                    .style(Style::default().bg(color).fg(Color::White))
                    .alignment(Alignment::Center),
                chunks[i]
            )
        }
    }
    pub fn render(&mut self, f: &mut Frame<impl Backend>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(100)
            ])
            .split(f.size());
        self.topbar(f, chunks[0]);
        match self.current_tab {
            0 => self.menu(f, chunks[1]),
            1 => self.paths.render(f, chunks[1]),
            2 => self.backup.render(f, chunks[1]),
            _ => unreachable!()
        }
    }
    pub fn menu(&self, f: &mut Frame<impl Backend>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(1)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(1)
            ])
            .split(area);
        f.render_widget(
            Paragraph::new(vec![
                Spans::from(Span::raw("╔═╗┌─┐┬  ┬┌─┐  ╔═╗┌─┐┬  ┬┌─┐┬─┐")),
                Spans::from(Span::raw("╚═╗├─┤└┐┌┘├┤   ╚═╗├─┤└┐┌┘├┤ ├┬┘")),
                Spans::from(Span::raw("╚═╝┴ ┴ └┘ └─┘  ╚═╝┴ ┴ └┘ └─┘┴└─")),
            ])
            .style(Style::default().fg(Color::LightMagenta))
            .alignment(Alignment::Center),
            chunks[0]
        );
        f.render_widget(
            Paragraph::new(vec![
                Spans::from(Span::raw("Use WASD keys or Arrows to move around.")),
                Spans::from(Span::raw("Press \"Q\" to exit.")),
                Spans::from(Span::raw("")),
                Spans::from(Span::raw("This program backup all your game saves to an private github repo;")),
                Spans::from(Span::raw("With an unique branch to each game.")),
                Spans::from(Span::raw("")),
                Spans::from(Span::raw("The source code can be found at: https://github.com/muriel-guedes/save-saver.")),
            ]),
            chunks[1]
        );
    }
}