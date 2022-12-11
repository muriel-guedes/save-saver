use std::{path::{PathBuf, Path}, io::Write};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    widgets::{Paragraph, Borders, Block, Clear},
    Frame, text::{Span, Spans}, style::{Style, Color}
};

const PATH: &'static str = "./paths.txt";

#[derive(Clone)]
pub struct BackupPath {
    pub name: String,
    pub branch_name: String,
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf
}
impl BackupPath {
    pub fn new(name: impl AsRef<str>, path: impl AsRef<Path>) -> Self {
        let name = name.as_ref().to_string();
        let path = path.as_ref().to_path_buf();
        Self {
            branch_name: name.replace(' ', "-").to_lowercase(),
            name,
            absolute_path: format_path_to_absolute(path.clone()),
            relative_path: format_path_to_relative(path)
        }
    }
}

pub struct Paths {
    pub paths: Vec<BackupPath>,
    pub selected_item: usize,
    pub add_new_dialog_folder: Option<PathBuf>,
    pub capturing_input: Option<String>
}
impl Paths {
    pub fn read() -> Self {
        let paths = if Path::new(PATH).exists() {
            let mut paths = Vec::new();
            for line in std::fs::read_to_string(PATH).unwrap().lines() {
                if line.len() == 0 { continue }
                let mut line = line.split('=');
                let name = line.next().unwrap().trim();
                let path = PathBuf::from(line.next().unwrap().trim());
                paths.push(BackupPath::new(name, path))
            }
            paths
        } else { 
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(PATH).unwrap()
                .write_all(b"").unwrap();
            vec![]
        };
        Self {
            paths,
            selected_item: 0,
            add_new_dialog_folder: None,
            capturing_input: None
        }
    }
    pub fn render(&self, f: &mut Frame<impl Backend>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(2),
                Constraint::Percentage(100)
            ])
            .split(area);

        f.render_widget(
            Paragraph::new("Press \"N\" to add a new path, \"R\" to remove the selected path, or \"F\" to reload."),
            chunks[0]
        );

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(2)
            .horizontal_margin(4)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];
        let mut spans = vec![];
        let mut iter = self.paths.iter();
        let mut scroll = 0;
        let length = self.paths.len();
        if length > layout.height as usize {
            let h2 = (layout.height / 2) as usize;
            if self.selected_item > h2 {
                if self.selected_item < length - h2 {
                    scroll = self.selected_item - h2;
                } else {
                    scroll = length - layout.height as usize;
                }
                iter.advance_by(scroll).unwrap();
            }
        }
        for (i, path) in iter.enumerate() {
            const C: u8 = 50;
            let color = if i + scroll == self.selected_item { Color::Rgb(C, C, C) } else { Color::Reset };
            spans.push(Spans::from(
                Span::styled(
                    format!(" {}: {} ", path.name, path.absolute_path.display()),
                    Style::default().bg(color).fg(Color::White)
                )
            ));
        }
        f.render_widget(Paragraph::new(spans).alignment(Alignment::Left), layout);

        f.render_widget(Block::default().title("Paths to backup").borders(Borders::ALL), chunks[1]);
        self.render_add_new_dialog(f);
    }
    pub fn dialog_add_new(&mut self) {
        let folder = match rfd::FileDialog::new().set_directory("/").pick_folder() {
            Some(v) => format_path_to_relative(v),
            None => return
        };
        self.add_new_dialog_folder = Some(folder);
        self.capturing_input = Some(String::new())
    }
    pub fn render_add_new_dialog(&self, f: &mut Frame<impl Backend>) {
        if self.add_new_dialog_folder.is_none() { return }
        let area = centered_rect(50, 50, f.size());
        
        let block = Block::default().title("Add new folder").borders(Borders::ALL);
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let input = match &self.capturing_input {
            Some(v) => v.clone(),
            None => String::new()
        };
        f.render_widget(
            Paragraph::new(vec![
                Spans::from("Type the game name and then press \"Enter\" to exit."),
                Spans::from(vec![
                    Span::from("> "),
                    Span::from(input)
                ])
            ]),
            Layout::default()
                .direction(Direction::Vertical)
                .margin(3)
                .constraints([
                    Constraint::Min(1),
                ])
                .split(area)[0]
        );
    }
    pub fn add_new(&mut self) {
        let path = self.add_new_dialog_folder.take().expect("No folder selected");
        let name = self.capturing_input.take().expect("Name can not be empty");
        if name.len() == 0 { panic!("Name can not be empty.") }
        
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .write(true)
            .create(true)
            .open(PATH).unwrap();
        write!(file, "\r\n{} = {}", name, path.display()).unwrap();

        self.paths.push(BackupPath::new(name, path));
    }
    pub fn scroll_down(&mut self) {
        if self.selected_item < self.paths.len() - 1 { self.selected_item += 1 }
        else { self.selected_item = 0 }
    }
    pub fn scroll_up(&mut self) {
        if self.selected_item > 0 { self.selected_item -= 1 }
        else { self.selected_item = self.paths.len() - 1 }
    }
    pub fn delete_selected(&mut self) {
        if self.paths.len() == 0 { return }
        self.paths.remove(self.selected_item);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(false)
            .truncate(true)
            .open(PATH).unwrap();
        for path in &self.paths {
            write!(file, "{} = {}\r\n", path.name, path.relative_path.display()).unwrap();
        }
    }
}

fn format_path_to_relative(path: impl AsRef<Path>) -> PathBuf {
    let mut path = path.as_ref().to_path_buf();
    let home_dir = dirs::home_dir().unwrap();
    if path.starts_with(&home_dir) {
        path = PathBuf::from("$HOME").join(path.strip_prefix(&home_dir).unwrap());
    }
    path
}
pub fn format_path_to_absolute(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.starts_with("$HOME") {
        let home_dir = dirs::home_dir().unwrap();
        return home_dir.join(path.strip_prefix("$HOME").unwrap())
    }
    path.to_path_buf()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical_area[1])[1]
}