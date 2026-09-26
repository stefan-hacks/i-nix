//! Interactive TUI for i-nix using ratatui.
//!
//! Run with: i-nix tui

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    MainMenu,
    Packages,
    Services,
    DesktopEnvs,
    Generations,
    ApplyConfirm,
    RemoteDeploy,
    Output { title: String, content: String },
}

#[derive(Debug)]
struct App {
    state: AppState,
    selected_menu: usize,
    selected_pkg: usize,
    selected_svc: usize,
    selected_de: usize,
    selected_generation: usize,
    menu_items: Vec<String>,
    packages: Vec<String>,
    services: Vec<String>,
    desktop_envs: Vec<String>,
    generations: Vec<String>,
    output_scroll: u16,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::MainMenu,
            selected_menu: 0,
            selected_pkg: 0,
            selected_svc: 0,
            selected_de: 0,
            selected_generation: 0,
            menu_items: vec![
                "📦 Packages".to_string(),
                "🔧 Services".to_string(),
                "🖥️  Desktop Environments".to_string(),
                "🔄 Generations / Rollback".to_string(),
                "▶️  Apply Changes".to_string(),
                "🌐 Remote Deploy".to_string(),
                "📊 System Info".to_string(),
                "❌ Quit".to_string(),
            ],
            packages: vec![
                "firefox".to_string(),
                "git".to_string(),
                "vim".to_string(),
                "kitty".to_string(),
                "neovim".to_string(),
                "ripgrep".to_string(),
                "starship".to_string(),
                "tmux".to_string(),
            ],
            services: vec![
                "ssh (openssh)".to_string(),
                "docker".to_string(),
                "bluetooth".to_string(),
                "pipewire".to_string(),
                "printing (cups)".to_string(),
                "syncthing".to_string(),
            ],
            desktop_envs: vec![
                "GNOME".to_string(),
                "KDE Plasma".to_string(),
                "Hyprland".to_string(),
                "Sway".to_string(),
                "i3".to_string(),
                "Niri".to_string(),
                "XFCE".to_string(),
                "Cinnamon".to_string(),
                "Pop!_OS".to_string(),
            ],
            generations: vec![
                "Generation 42 (current)".to_string(),
                "Generation 41".to_string(),
                "Generation 40".to_string(),
                "Generation 39".to_string(),
                "Generation 38".to_string(),
            ],
            output_scroll: 0,
        }
    }
}

pub async fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::default();
    let result = run_app(&mut terminal, &mut app).await;
    ratatui::restore();
    result
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.state {
                AppState::MainMenu => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected_menu = (app.selected_menu + 1) % app.menu_items.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_menu == 0 {
                            app.selected_menu = app.menu_items.len() - 1;
                        } else {
                            app.selected_menu -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        match app.selected_menu {
                            0 => app.state = AppState::Packages,
                            1 => app.state = AppState::Services,
                            2 => app.state = AppState::DesktopEnvs,
                            3 => app.state = AppState::Generations,
                            4 => app.state = AppState::ApplyConfirm,
                            5 => app.state = AppState::RemoteDeploy,
                            6 => {
                                app.state = AppState::Output {
                                    title: "System Info".to_string(),
                                    content: get_system_info(),
                                };
                            }
                            7 => return Ok(()),
                            _ => {}
                        }
                    }
                    _ => {}
                },
                AppState::Packages => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected_pkg = (app.selected_pkg + 1) % app.packages.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_pkg == 0 {
                            app.selected_pkg = app.packages.len() - 1;
                        } else {
                            app.selected_pkg -= 1;
                        }
                    }
                    KeyCode::Char('i') => {
                        let pkg = &app.packages[app.selected_pkg];
                        app.state = AppState::Output {
                            title: format!("Install {}", pkg),
                            content: format!("Would run: i-nix install {}", pkg),
                        };
                    }
                    KeyCode::Char('r') => {
                        let pkg = &app.packages[app.selected_pkg];
                        app.state = AppState::Output {
                            title: format!("Remove {}", pkg),
                            content: format!("Would run: i-nix remove {}", pkg),
                        };
                    }
                    _ => {}
                },
                AppState::Services => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected_svc = (app.selected_svc + 1) % app.services.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_svc == 0 {
                            app.selected_svc = app.services.len() - 1;
                        } else {
                            app.selected_svc -= 1;
                        }
                    }
                    KeyCode::Char('e') => {
                        let svc = &app.services[app.selected_svc];
                        app.state = AppState::Output {
                            title: format!("Enable {}", svc),
                            content: format!("Would run: i-nix enable {}", svc.split_whitespace().next().unwrap_or("")),
                        };
                    }
                    _ => {}
                },
                AppState::DesktopEnvs => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected_de = (app.selected_de + 1) % app.desktop_envs.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_de == 0 {
                            app.selected_de = app.desktop_envs.len() - 1;
                        } else {
                            app.selected_de -= 1;
                        }
                    }
                    KeyCode::Char('g') => {
                        let de = &app.desktop_envs[app.selected_de];
                        app.state = AppState::Output {
                            title: format!("Generate {} config", de),
                            content: format!("Would run: i-nix desktop generate {}", de.to_lowercase().replace("!", "").replace(" ", "-")),
                        };
                    }
                    _ => {}
                },
                AppState::Generations => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.selected_generation = (app.selected_generation + 1) % app.generations.len();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_generation == 0 {
                            app.selected_generation = app.generations.len() - 1;
                        } else {
                            app.selected_generation -= 1;
                        }
                    }
                    KeyCode::Char('r') => {
                        let selected_item = &app.generations[app.selected_generation];
                        app.state = AppState::Output {
                            title: "Rollback".to_string(),
                            content: format!("Would run: i-nix rollback -g {}", selected_item.split_whitespace().nth(1).unwrap_or("")),
                        };
                    }
                    _ => {}
                },
                AppState::ApplyConfirm => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    KeyCode::Char('y') => {
                        app.state = AppState::Output {
                            title: "Apply Changes".to_string(),
                            content: "Would run: i-nix apply --yes".to_string(),
                        };
                    }
                    KeyCode::Char('t') => {
                        app.state = AppState::Output {
                            title: "Test Configuration".to_string(),
                            content: "Would run: i-nix apply --test".to_string(),
                        };
                    }
                    _ => {}
                },
                AppState::RemoteDeploy => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::MainMenu,
                    _ => {}
                },
                AppState::Output { .. } => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        app.state = AppState::MainMenu;
                        app.output_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.output_scroll += 1,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.output_scroll > 0 {
                            app.output_scroll -= 1;
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match app.state {
        AppState::MainMenu => draw_main_menu(frame, app, area),
        AppState::Packages => draw_packages(frame, app, area),
        AppState::Services => draw_services(frame, app, area),
        AppState::DesktopEnvs => draw_desktop_envs(frame, app, area),
        AppState::Generations => draw_generations(frame, app, area),
        AppState::ApplyConfirm => draw_apply_confirm(frame, app, area),
        AppState::RemoteDeploy => draw_remote_deploy(frame, app, area),
        AppState::Output { ref title, ref content } => draw_output(frame, app, area, title, content),
    }
}

fn draw_main_menu(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(area);

    let header = Paragraph::new("i-nix — Imperative UX for Declarative Nix/NixOS")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, layout[0]);

    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected_menu {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(item.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Main Menu").borders(Borders::ALL))
        .highlight_symbol("▶ ");
    frame.render_widget(list, layout[1]);

    let help = Paragraph::new("j/k or ↑/↓ to navigate • Enter to select • q to quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, area.inner(Margin { horizontal: 0, vertical: area.height - 1 }));
}

fn draw_packages(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let style = if i == app.selected_pkg {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(pkg.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("📦 Packages — i: install, r: remove, q: back").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_services(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .services
        .iter()
        .enumerate()
        .map(|(i, svc)| {
            let style = if i == app.selected_svc {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(svc.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("🔧 Services — e: enable/disable, q: back").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_desktop_envs(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .desktop_envs
        .iter()
        .enumerate()
        .map(|(i, de)| {
            let style = if i == app.selected_de {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(de.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("🖥️  Desktop Environments — g: generate config, q: back").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_generations(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .generations
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected_generation {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(item.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("🔄 Generations — r: rollback, q: back").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn draw_apply_confirm(frame: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("About to apply Nix configuration."),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" — Apply (nixos-rebuild switch)"),
        ]),
        Line::from(vec![
            Span::styled("t", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" — Test only (nixos-rebuild test)"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" — Cancel"),
        ]),
    ]);

    let popup = Paragraph::new(text)
        .block(Block::default().title("▶️  Apply Changes").borders(Borders::ALL))
        .alignment(Alignment::Center);
    frame.render_widget(popup, area);
}

fn draw_remote_deploy(frame: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("Remote Deployment"),
        Line::from(""),
        Line::from("Use: i-nix apply --remote user@host"),
        Line::from(""),
        Line::from("This will:"),
        Line::from("  1. Build configuration locally"),
        Line::from("  2. Copy closure to remote host"),
        Line::from("  3. Activate on remote host"),
        Line::from(""),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" — Back to menu"),
        ]),
    ]);

    let popup = Paragraph::new(text)
        .block(Block::default().title("🌐 Remote Deploy").borders(Borders::ALL))
        .alignment(Alignment::Center);
    frame.render_widget(popup, area);
}

fn draw_output(
    frame: &mut Frame,
    app: &App,
    area: ratatui::layout::Rect,
    title: &str,
    content: &str,
) {
    let text = Text::from(content);
    let popup = Paragraph::new(text)
        .block(Block::default().title(title.to_string()).borders(Borders::ALL))
        .wrap(Wrap { trim: true })
        .scroll((app.output_scroll, 0));
    frame.render_widget(popup, area);

    let help = Paragraph::new("q/Enter: back • j/k: scroll")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, area.inner(Margin { horizontal: 0, vertical: area.height - 1 }));
}

fn get_system_info() -> String {
    let mut info = String::from("i-nix System Information\n");
    info.push_str("========================\n\n");
    info.push_str(&format!("Hostname: {:?}\n", std::env::var("HOSTNAME")));
    info.push_str(&format!("User: {:?}\n", std::env::var("USER")));
    info.push_str(&format!("Home: {:?}\n", std::env::var("HOME")));
    info.push_str("\n");
    info.push_str("Available commands:\n");
    info.push_str("  i-nix init\n");
    info.push_str("  i-nix install <pkg>\n");
    info.push_str("  i-nix remove <pkg>\n");
    info.push_str("  i-nix enable <service>\n");
    info.push_str("  i-nix disable <service>\n");
    info.push_str("  i-nix apply\n");
    info.push_str("  i-nix rollback\n");
    info.push_str("  i-nix desktop detect\n");
    info.push_str("  i-nix sync\n");
    info
}
