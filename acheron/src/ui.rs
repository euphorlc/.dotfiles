use acheron::{
    backend::{Backend, Container, Distrobox},
    config::{self, Paths, Resolved, Workspace},
    engine,
    environment::Environment,
    state,
};
use anyhow::{Result, ensure};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

const DEFAULT_IMAGE: &str = "ghcr.io/euphorlc/acheron-dev:preview";
const BG: Color = Color::Rgb(18, 23, 34);
const MUTED: Color = Color::Rgb(130, 146, 168);
const ACCENT: Color = Color::Rgb(149, 224, 204);

enum Modal {
    None,
    Create {
        names: String,
        image: String,
        theme: String,
        field: usize,
    },
    Confirm {
        reset: bool,
        factory: bool,
        name: String,
        typed: String,
    },
}
struct Snapshot {
    containers: Vec<Container>,
    specs: Vec<Resolved>,
}
type JobResult = Result<(Snapshot, String)>;
struct App {
    env: Environment,
    paths: Option<Paths>,
    profile: String,
    containers: Vec<Container>,
    specs: Vec<Resolved>,
    selected: usize,
    modal: Modal,
    log: String,
    job: Option<Receiver<JobResult>>,
    ticks: usize,
    demo: bool,
    log_scroll: u16,
}

fn snapshot(paths: &Paths, profile: &str) -> Result<Snapshot> {
    let specs = config::load(paths, profile)?;
    let live = Distrobox.list()?;
    let state = state::read(paths)?;
    let mut containers: Vec<_> = engine::plan(&specs, &live, &state)
        .iter()
        .map(|step| Container {
            id: live
                .iter()
                .find(|c| c.name == step.spec.name)
                .map(|c| c.id.clone())
                .unwrap_or_default(),
            name: step.spec.name.clone(),
            image: step.spec.image.clone(),
            status: format!(
                "{} / {}",
                step.action,
                live.iter()
                    .find(|c| c.name == step.spec.name)
                    .map(|c| c.status.as_str())
                    .unwrap_or("absent")
            ),
        })
        .collect();
    containers.extend(
        live.into_iter()
            .filter(|c| !specs.iter().any(|s| s.name == c.name)),
    );
    Ok(Snapshot { containers, specs })
}

impl App {
    fn new(paths: Option<Paths>, profile: String, demo: bool) -> Result<Self> {
        let mut app = Self {
            env: Environment::detect(),
            paths,
            profile,
            containers: vec![],
            specs: vec![],
            selected: 0,
            modal: Modal::None,
            log: "Welcome to Acheron.".into(),
            job: None,
            ticks: 0,
            demo,
            log_scroll: 0,
        };
        if demo {
            for (name, theme, status) in
                [("dev", "mars", "sync config"), ("web", "vulcan", "create")]
            {
                app.specs.push(Resolved {
                    name: name.into(),
                    image: DEFAULT_IMAGE.into(),
                    theme: theme.into(),
                    home: format!("/home/demo/.local/share/acheron/homes/{name}").into(),
                    repo: "/home/demo/.dotfiles".into(),
                    mounts: vec![],
                });
                app.containers.push(Container {
                    id: name.into(),
                    name: name.into(),
                    status: status.into(),
                    image: DEFAULT_IMAGE.into(),
                });
            }
            app.log = "DEMO · No container commands run or deployment files change.\nActions: define → review plan → apply. PageUp/PageDown scroll activity.".into();
        } else if app.env.in_container {
            app.log =
                "Run Acheron on the host to manage workspaces. Use --demo to preview here.".into();
        } else {
            app.refresh();
        }
        Ok(app)
    }
    fn spawn(&mut self, label: &str, task: impl FnOnce() -> JobResult + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.log = label.into();
        self.log_scroll = 0;
        std::thread::spawn(move || {
            let _ = tx.send(task());
        });
    }
    fn refresh(&mut self) {
        let paths = self.paths.clone().unwrap();
        let profile = self.profile.clone();
        self.spawn("Reading workspace plan…", move || Ok((snapshot(&paths, &profile)?, "Plan refreshed. Apply creates missing workspaces and syncs config; rebuilds remain explicit.".into())));
    }
    fn poll(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        let Some(rx) = &self.job else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.job = None;
                self.log_scroll = 0;
                match result {
                    Ok((snapshot, log)) => {
                        self.containers = snapshot.containers;
                        self.specs = snapshot.specs;
                        self.log = log;
                    }
                    Err(e) => {
                        self.log =
                            format!("Operation failed: {e:#}\nPress r to refresh before retrying.")
                    }
                }
                self.selected = self.selected.min(self.containers.len().saturating_sub(1));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                self.log = "Worker stopped unexpectedly. Refresh before retrying.".into();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    fn submit(&mut self) -> Result<()> {
        ensure!(
            !self.demo && !self.env.in_container,
            "Management requires a host session"
        );
        let paths = self.paths.clone().unwrap();
        let profile = self.profile.clone();
        match &self.modal {
            Modal::Create {
                names,
                image,
                theme,
                ..
            } => {
                let mut batch = vec![];
                for name in names.split(',').map(str::trim) {
                    ensure!(
                        !self.specs.iter().any(|s| s.name == name)
                            && !self.containers.iter().any(|s| s.name == name),
                        "Name already exists: {name}"
                    );
                    ensure!(
                        !batch.iter().any(|s: &Workspace| s.name == name),
                        "Repeated name: {name}"
                    );
                    let workspace = Workspace {
                        name: name.into(),
                        image: image.clone(),
                        theme: theme.clone(),
                        mounts: vec![],
                    };
                    workspace.resolve(&paths)?;
                    batch.push(workspace);
                }
                self.spawn("Saving Git-backed workspace definitions…", move || {
                    for workspace in batch { config::add_workspace(&paths, &profile, workspace)?; }
                    Ok((snapshot(&paths, &profile)?, "Definitions saved in the repository. Review the plan, then press a to apply. Commit these definitions to reproduce them on another computer.".into()))
                });
            }
            Modal::Confirm {
                reset,
                factory,
                name,
                typed,
            } => {
                ensure!(typed == name, "Type the exact workspace name");
                let name = name.clone();
                let reset = *reset;
                let factory = *factory;
                self.spawn("Updating workspace…", move || {
                    let log = if reset {
                        let specs = config::load(&paths, &profile)?;
                        let spec = specs.iter().find(|s| s.name == name).ok_or_else(|| {
                            anyhow::anyhow!(
                                "Workspace needs a Git-backed definition before rebuilding"
                            )
                        })?;
                        engine::rebuild(&paths, spec, factory, &mut Distrobox)?
                    } else {
                        engine::delete(&paths, &name, &mut Distrobox)?
                    };
                    Ok((snapshot(&paths, &profile)?, log))
                });
            }
            Modal::None => {}
        }
        self.modal = Modal::None;
        Ok(())
    }
    fn key(&mut self, code: KeyCode) -> bool {
        if code == KeyCode::PageDown {
            self.log_scroll = self.log_scroll.saturating_add(4);
            return false;
        }
        if code == KeyCode::PageUp {
            self.log_scroll = self.log_scroll.saturating_sub(4);
            return false;
        }
        if self.job.is_some() {
            return false;
        }
        if !matches!(self.modal, Modal::None) {
            match code {
                KeyCode::Esc => self.modal = Modal::None,
                KeyCode::Enter => {
                    if let Err(e) = self.submit() {
                        self.log = e.to_string();
                    }
                }
                _ => match &mut self.modal {
                    Modal::Create {
                        names,
                        image,
                        theme,
                        field,
                    } => match code {
                        KeyCode::Tab | KeyCode::Down => *field = (*field + 1) % 3,
                        KeyCode::BackTab | KeyCode::Up => *field = (*field + 2) % 3,
                        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if *field == 2 => {
                            *theme = if theme == "mars" { "vulcan" } else { "mars" }.into()
                        }
                        KeyCode::Backspace if *field < 2 => {
                            if *field == 0 {
                                names.pop();
                            } else {
                                image.pop();
                            }
                        }
                        KeyCode::Char(c) if *field < 2 => {
                            if *field == 0 {
                                names.push(c);
                            } else {
                                image.push(c);
                            }
                        }
                        _ => {}
                    },
                    Modal::Confirm { typed, .. } => match code {
                        KeyCode::Char(c) => typed.push(c),
                        KeyCode::Backspace => {
                            typed.pop();
                        }
                        _ => {}
                    },
                    Modal::None => {}
                },
            }
            return false;
        }
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(self.containers.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('r') if !self.demo && !self.env.in_container => self.refresh(),
            KeyCode::Char('n' | 'a' | 'x' | 'F' | 'd') if self.demo || self.env.in_container => {
                self.log = "Management is disabled in demo mode and inside containers.".into()
            }
            KeyCode::Char('a') => {
                let paths = self.paths.clone().unwrap();
                let profile = self.profile.clone();
                self.spawn("Applying workspace plan…", move || {
                    let specs = config::load(&paths, &profile)?;
                    let log = engine::apply(&paths, &specs, &mut Distrobox)?;
                    Ok((snapshot(&paths, &profile)?, log))
                });
            }
            KeyCode::Char('n') => {
                self.modal = Modal::Create {
                    names: String::new(),
                    image: DEFAULT_IMAGE.into(),
                    theme: "vulcan".into(),
                    field: 0,
                }
            }
            KeyCode::Char('x' | 'F' | 'd') => {
                if let Some(c) = self.containers.get(self.selected) {
                    self.modal = Modal::Confirm {
                        reset: code != KeyCode::Char('d'),
                        factory: code == KeyCode::Char('F'),
                        name: c.name.clone(),
                        typed: String::new(),
                    };
                }
            }
            _ => {}
        }
        false
    }
}

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(51, 66, 87)))
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(BG).fg(Color::Rgb(225, 233, 244))),
        area,
    );
    if area.width < 64 || area.height < 22 {
        f.render_widget(
            Paragraph::new("Acheron · Enlarge terminal to at least 64 × 22. Press q to quit."),
            area,
        );
        return;
    }
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(7),
        Constraint::Length(7),
        Constraint::Length(2),
    ])
    .margin(1)
    .split(area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" ◉ ACHERON ", Style::default().fg(ACCENT).bold()),
                Span::styled(" / your workspace, in focus", Style::default().fg(MUTED)),
            ]),
            Line::from(Span::styled(
                if app.demo {
                    "   DEMO PREVIEW"
                } else {
                    "   DISTROBOX WORKSPACE MANAGER"
                },
                Style::default().fg(MUTED),
            )),
        ]),
        chunks[0],
    );
    let environment = format!(
        " {}  •  {}  •  {}",
        app.env.os,
        if app.env.in_container {
            "inside container"
        } else {
            "host"
        },
        if app.env.immutable {
            "OSTree detected"
        } else {
            "standard filesystem"
        }
    );
    f.render_widget(
        Paragraph::new(environment)
            .block(panel(" Environment "))
            .wrap(Wrap { trim: true }),
        chunks[1],
    );
    let body = Layout::horizontal([Constraint::Percentage(56), Constraint::Percentage(44)])
        .split(chunks[2]);
    let rows: Vec<Row> = app
        .containers
        .iter()
        .map(|c| {
            let theme = app
                .specs
                .iter()
                .find(|s| s.name == c.name)
                .map(|s| s.theme.as_str())
                .unwrap_or("external");
            Row::new(vec![c.name.clone(), c.status.clone(), theme.into()]).style(
                Style::default().fg(if theme == "mars" {
                    Color::Rgb(242, 183, 5)
                } else {
                    ACCENT
                }),
            )
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(45),
            Constraint::Percentage(25),
        ],
    )
    .header(
        Row::new(["NAME", "STATUS", "THEME"])
            .style(Style::default().fg(MUTED))
            .bottom_margin(1),
    )
    .block(panel(" Workspaces "))
    .row_highlight_style(Style::default().bg(Color::Rgb(38, 53, 72)).bold())
    .highlight_symbol("▸ ");
    let mut state = TableState::default().with_selected(if app.containers.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    f.render_stateful_widget(table, body[0], &mut state);
    let detail = if let Some(c) = app.containers.get(app.selected) {
        let spec = app.specs.iter().find(|s| s.name == c.name);
        format!(
            "{}\n{}\n\nIMAGE\n{}\n\nHOME\n{}\n\nENTER\nacheron enter {}",
            c.name,
            c.status,
            c.image,
            spec.map(|s| s.home.display().to_string())
                .unwrap_or("External · not managed by Acheron".into()),
            c.name
        )
    } else {
        "A fresh start.\n\nPress n to create your first workspace.\n\nChoose Mars for warm amber or Vulcan for cool seafoam.\n\nYour existing dotfiles provide the themes.".into()
    };
    f.render_widget(
        Paragraph::new(detail)
            .block(panel(" Workspace details "))
            .wrap(Wrap { trim: true }),
        body[1],
    );
    let busy = if app.job.is_some() {
        format!(
            " {} Working · please wait ",
            ["◐", "◓", "◑", "◒"][app.ticks % 4]
        )
    } else {
        " Activity ".into()
    };
    f.render_widget(
        Paragraph::new(app.log.as_str())
            .block(panel(&busy))
            .wrap(Wrap { trim: true })
            .scroll((app.log_scroll, 0)),
        chunks[3],
    );
    f.render_widget(
        Paragraph::new(
            " n define  a apply  x rebuild  F wipe+rebuild  d delete  r refresh  q quit",
        )
        .style(Style::default().fg(ACCENT)),
        chunks[4],
    );
    let modal_text = match &app.modal {
        Modal::None => return,
        Modal::Create {
            names,
            image,
            theme,
            field,
        } => format!(
            "DEFINE WORKSPACES\n\n{} Names: {}\n  Comma-separated, e.g. rust-dev, web-dev\n\n{} Image: {}\n  Prebuilt image; separate persistent home\n\n{} Theme: ◂ {} ▸\n\nTab select · ← → theme · Enter save · Esc cancel",
            if *field == 0 { "▸" } else { " " },
            names,
            if *field == 1 { "▸" } else { " " },
            image,
            if *field == 2 { "▸" } else { " " },
            theme
        ),
        Modal::Confirm {
            reset,
            name,
            typed,
            factory,
        } => format!(
            "{} WORKSPACE\n\nContainer: {}\n\nThis removes the container and its installed tools.\nHome is preserved unless factory reset is selected.\n{}\n\nType the name to confirm: {}\n\nEnter confirm · Esc cancel",
            if *factory {
                "FACTORY RESET"
            } else if *reset {
                "REBUILD"
            } else {
                "DELETE"
            },
            name,
            if *factory {
                "FACTORY RESET: workspace home WILL BE DELETED."
            } else if *reset {
                "Recreated from the selected Git-backed definition."
            } else {
                "The container will not be recreated."
            },
            typed
        ),
    };
    let popup = Rect::new(
        area.x + (area.width - 60) / 2,
        area.y + (area.height - 16) / 2,
        60,
        16,
    );
    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(modal_text)
            .block(panel(" Acheron ").style(Style::default().bg(BG).fg(ACCENT)))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub fn run(paths: Option<Paths>, profile: String, demo: bool) -> Result<()> {
    let mut app = App::new(paths, profile, demo)?;
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        loop {
            app.poll();
            terminal.draw(|f| draw(f, &app))?;
            if event::poll(Duration::from_millis(120))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && app.key(key.code)
            {
                break;
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demo_is_read_only_and_renders_responsively() {
        let mut app = App::new(None, "default".into(), true).unwrap();
        for key in ['n', 'a', 'x', 'F', 'd'] {
            app.key(KeyCode::Char(key));
            assert!(app.job.is_none());
            assert!(matches!(app.modal, Modal::None));
        }
        for (w, h) in [(120, 36), (80, 24), (64, 22), (30, 10)] {
            let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| draw(f, &app)).unwrap();
            app.modal = Modal::Confirm {
                reset: true,
                factory: true,
                name: "dev".into(),
                typed: String::new(),
            };
            terminal.draw(|f| draw(f, &app)).unwrap();
            app.modal = Modal::None;
        }
    }
}
