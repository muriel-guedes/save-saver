use std::{path::{Path, PathBuf}, process::Command, env::set_current_dir, ffi::OsStr, sync::{mpsc::{Receiver, Sender, channel}},
    io::Write};

const PATH: &'static str = "./conf.txt";

use chrono::Utc;
use dirs::document_dir;
use fs_extra::dir::CopyOptions;
use tui::{Frame, backend::Backend, layout::{Rect, Constraint, Layout}, widgets::Paragraph, text::{Spans, Span}, style::{Style, Color}};

use crate::paths::BackupPath;

pub struct Backup {
    pub text_input: String,
    pub repo_url: Option<String>,
    pub uploading: bool,
    pub downloading: bool,
    pub receive_log: Option<Receiver<Option<String>>>,
    pub logs: Vec<String>
}
impl Backup {
    pub fn new() -> Self {
        let mut repo_url = None;
        if Path::new(PATH).exists() {
            let content = std::fs::read_to_string(PATH).unwrap();
            for line in content.lines() {
                if line.len() == 0 { continue }
                let mut line = line.split('=');
                let name = line.next().unwrap().trim();
                if name == "repo_url" {
                    let value = line.next().unwrap().trim().to_string();
                    if value.len() == 0 { continue }
                    repo_url = Some(value);
                }
            }
        }
        Self {
            text_input: String::new(),
            repo_url,
            uploading: false,
            downloading: false,
            receive_log: None,
            logs: Vec::new()
        }
    }
    pub fn render(&mut self, f: &mut Frame<impl Backend>, area: Rect) {
        if self.uploading || self.downloading {
            self.render_logs(f, area)
        } else if let Some(repo_url) = self.repo_url.clone() {
            self.render_menu(f, area, repo_url)
        }else {
            self.render_enter_repo_url(f, area)
        }
    }
    pub fn render_enter_repo_url(&self, f: &mut Frame<impl Backend>, area: Rect) {
        f.render_widget(Paragraph::new(vec![
            Spans::from("Please enter the repo url, then press \"Enter\" to continue."),
            Spans::from("Ex: https://github.com/muriel-guedes/game-saves"),
            Spans::from(""),
            Spans::from(vec![
                Span::raw("> "),
                Span::from(self.text_input.clone())
            ]),
        ]), Layout::default()
            .margin(2)
            .constraints([Constraint::Min(1)])
            .split(area)[0]);
    }
    pub fn render_menu(&self, f: &mut Frame<impl Backend>, area: Rect, repo_url: String) {
        f.render_widget(Paragraph::new(vec![
            Spans::from(vec![
                Span::raw("Repo URL: "),
                Span::from(repo_url.clone())
            ]),
            Spans::from("Press \"Enter\" to backup all your data, or \"R\" to restore."),
        ]), Layout::default()
            .margin(2)
            .constraints([Constraint::Min(1)])
            .split(area)[0]);
    }
    pub fn render_logs(&mut self, f: &mut Frame<impl Backend>, area: Rect) {
        if let Some(rx) = self.receive_log.as_ref() {
            if let Some(log) = rx.recv().unwrap() {
                self.logs.push(log)
            } else {
                self.receive_log = None
            }
        }
        let mut spans = Vec::new();
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .append(false)
            .open(get_uploading_log_path()).unwrap();
        for log in &self.logs {
            if log.len() == 0 { continue }
            let mut log = log.clone();
            let color = if log.starts_with('#') {
                log.remove(0);
                Color::Yellow
            } else { Color::Reset };
            spans.push(Spans::from(Span::styled(log.clone(), Style::default().fg(color))));
            writeln!(log_file, "{log}").unwrap();
        }
        let chunks = Layout::default()
            .margin(0)
            .constraints([
                Constraint::Min(2),
                Constraint::Percentage(100)
            ])
            .split(area);
        f.render_widget(Paragraph::new(
            format!("You can found full logs in: {}", get_uploading_log_path().display())
        ), chunks[0]);
        f.render_widget(Paragraph::new(spans), chunks[1]);
    }
    pub fn set_repo_url(&mut self) {
        if self.text_input.len() == 0 { return }
        self.repo_url = Some(self.text_input.clone());
        self.text_input = String::new();
        std::fs::write(PATH, format!("repo_url = {}", self.repo_url.as_ref().unwrap())).unwrap();
    }
    pub fn backup(&mut self, paths: Vec<BackupPath>) {
        self.uploading = true;
        self.logs.clear();
        let (tx, rx): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
        self.receive_log = Some(rx);
        let repo_url = self.repo_url.as_ref().unwrap().clone();
        std::thread::spawn(move || {
            send(&tx, "#Creating temp folder ...");
            std::fs::remove_dir_all("./temp").ok();
            std::fs::create_dir("./temp").unwrap();
            set_current_dir("./temp").unwrap();
            send(&tx, "#Initializing repo ...");
            run_command(&tx, "git", ["init"]);
            send(&tx, "#Adding origin ...");
            run_command(&tx, "git", ["remote", "add", "origin", &repo_url]);
            run_command(&tx, "git", ["fetch"]);
            run_command(&tx, "git", ["checkout", "--orphan", "master"]);
            run_command(&tx, "git", ["pull", "origin", "master", "-f"]);

            // Update repo README.md
            let readme_content = std::fs::read_to_string("README.md").unwrap_or_default();
            let readme_content_lines: Vec<&str> = readme_content.lines().collect();
            let mut readme = std::fs::OpenOptions::new()
                .truncate(false)
                .create(true)
                .write(true)
                .append(true)
                .open("README.md").unwrap();
            for readme_content_line in &readme_content_lines {
                writeln!(readme, "{}", readme_content_line).unwrap();
            }
            'p: for path in &paths {
                let line = format!("{} = {}<br>", path.name, path.relative_path.display());
                for readme_content_line in &readme_content_lines {
                    if **readme_content_line == *line.as_str() {
                        continue 'p
                    }
                }
                writeln!(readme, "{line}").unwrap();
            }
            run_command(&tx, "git", ["add", "."]);
            run_command(&tx, "git", ["commit", "-m", &format!("\"{}\"", Utc::now())]);
            run_command(&tx, "git", ["push", "origin", "master", "-f"]);
            
            for path in &paths {
                if !path.absolute_path.exists() {
                    send(&tx, format!("#Skiping unexisting path: \"{}\" ...", path.absolute_path.display()));
                    continue
                }
                
                send(&tx, format!("#Switching to branch: \"{}\" ...", path.branch_name));
                run_command(&tx, "git", ["checkout", "--orphan", &path.branch_name]);

                send(&tx, format!("#Copying files from \"{}\" to \"./temp/content\" ...", path.absolute_path.display()));
                copy_folder_files_to_folder(&path.absolute_path, "./content");

                std::fs::write("README.md", &format!("{}", path.absolute_path.display())).unwrap();
                
                send(&tx, "#Pushing to branch ...");
                run_command(&tx, "git", ["add", "."]);
                run_command(&tx, "git", ["commit", "-m", &format!("\"{}\"", Utc::now())]);
                run_command(&tx, "git", ["push", "origin", &path.branch_name, "-f"]);
            }
            set_current_dir("../").unwrap();
            send(&tx, "#Finished, press \"Enter\" to continue.");
            tx.send(None).unwrap();
            std::fs::remove_dir_all("./temp").ok();
        });
    }
    pub fn restore(&mut self, paths: Vec<BackupPath>) {
        self.downloading = true;
        self.logs.clear();
        let (tx, rx): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
        self.receive_log = Some(rx);
        let repo_url = self.repo_url.as_ref().unwrap().clone();
        std::thread::spawn(move || {
            send(&tx, "#Creating temp folder ...");
            std::fs::remove_dir_all("./temp").ok();
            std::fs::create_dir("./temp").unwrap();
            set_current_dir("./temp").unwrap();
            send(&tx, "#Initializing repo ...");
            run_command(&tx, "git", ["init"]);
            run_command(&tx, "git", ["remote", "add", "origin", &repo_url]);

            for path in paths {
                send(&tx, format!("#Downloading branch \"{}\" ...", path.branch_name));
                run_command(&tx, "git", ["checkout", &path.branch_name]);
                run_command(&tx, "git", ["pull", "origin", &path.branch_name, "--force"]);
                send(&tx, format!("#Copying to \"{}\" ...", path.absolute_path.display()));
                copy_folder_files_to_folder(&path.absolute_path, "./content");
            }

            set_current_dir("../").unwrap();
            std::fs::remove_dir_all("./temp").ok();
            send(&tx, "#Finished, press \"Enter\" to continue.");
            tx.send(None).unwrap();
        });
    }
}

fn send(tx: &Sender<Option<String>>, msg: impl AsRef<str>) {
    tx.send(Some(msg.as_ref().to_string())).unwrap();
}
fn run_command(
    tx: &Sender<Option<String>>,
    command: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>
) {
    let res = Command::new(command).args(args).output().unwrap();
    if res.status.success() {
        tx.send(Some(
            String::from_utf8_lossy(&res.stdout).into()
        )).unwrap()
    } else {
        tx.send(Some(
            format!("Error: {}", String::from_utf8_lossy(&res.stderr))
        )).unwrap()
    }
}

fn get_uploading_log_path() -> PathBuf {
    document_dir().unwrap().join("uploading.log")
}

fn copy_folder_files_to_folder(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    std::fs::remove_dir_all(to.as_ref()).ok();
    std::fs::create_dir(to.as_ref()).unwrap();
    let dir = match std::fs::read_dir(&from) { Ok(v) => v, Err(_) => return };
    for path in dir {
        let path = match path { Ok(v) => v, Err(_) => return };
        fs_extra::copy_items(&vec![path.path()], to.as_ref(), &CopyOptions {
            overwrite: true,
            ..Default::default()
        }).unwrap();
    }
}