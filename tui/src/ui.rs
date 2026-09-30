//! Desenho. Só lê o `App`; nenhuma decisão mora aqui.

use crate::app::{ARCHIVE_TYPES, App, ConfirmKind, Mode, NewTask, Pane, PickKind, PromptKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap};

const HELP: &[(&str, &str)] = &[
    ("j/k ↑/↓", "mover"),
    ("h/l ←/→ Tab", "trocar de painel"),
    ("g/G", "topo / fim"),
    ("Enter / e", "abrir a task no editor"),
    ("C-d / C-u", "rolar o preview"),
    ("/", "filtrar tasks (id e título)"),
    ("n", "nova task no projeto"),
    ("s", "trocar a fase"),
    ("t", "editar o título"),
    ("i", "editar a descrição"),
    ("r", "renomear o id (arquivo e links)"),
    ("a", "arquivar / desarquivar"),
    ("A", "mostrar / esconder arquivadas"),
    ("d", "apagar a task"),
    ("N", "novo projeto"),
    ("c", "abrir o CURRENT.md"),
    ("R", "reler o vault"),
    ("q / C-c", "sair"),
];

/// Os grupos de highlight do status.json viram cor de terminal.
fn hl_color(hl: &str) -> Color {
    match hl {
        "DiagnosticInfo" => Color::Blue,
        "DiagnosticHint" => Color::Cyan,
        "DiagnosticWarn" => Color::Yellow,
        "DiagnosticOk" => Color::Green,
        "DiagnosticError" => Color::Red,
        _ => Color::Reset,
    }
}

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn pane_block(title: String, focused: bool) -> Block<'static> {
    let (border, title_style) = if focused {
        (Style::default().fg(Color::Green), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        (dim(), Style::default())
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Span::styled(title, title_style))
}

fn highlight() -> Style {
    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
}

pub fn draw(f: &mut Frame, app: &App) {
    let [main, bar] = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
    let [left, mid, right] =
        Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(35), Constraint::Percentage(45)])
            .areas(main);
    draw_projects(f, app, left);
    draw_tasks(f, app, mid);
    draw_preview(f, app, right);
    draw_bar(f, app, bar);

    match &app.mode {
        Mode::NewTask(nt) => draw_new_task(f, app, nt),
        Mode::Prompt { kind, value } => draw_prompt(f, *kind, value),
        Mode::Pick { kind, options, idx } => draw_pick(f, app, *kind, options, *idx),
        Mode::Confirm { kind } => draw_confirm(f, app, *kind),
        Mode::Help => draw_help(f),
        Mode::Normal | Mode::Filter => {}
    }
}

fn draw_projects(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .data
        .projects
        .iter()
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled(" ", Style::default().fg(Color::Blue)),
                Span::raw(p.project.clone()),
                Span::styled(format!(" {}", p.active), dim()),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if !app.data.projects.is_empty() {
        state.select(Some(app.project_idx));
    }
    let list = List::new(items)
        .block(pane_block(" Projetos ".into(), app.pane == Pane::Projects))
        .highlight_style(highlight());
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_tasks(f: &mut Frame, app: &App, area: Rect) {
    let tasks = app.visible_tasks();
    let items: Vec<ListItem> = tasks
        .iter()
        .map(|t| {
            let st = app.status_of(t.status_num);
            let color = st.map_or(Color::Red, |s| hl_color(&s.hl_group));
            let icon = st.map_or("?", |s| s.icon.as_str());
            let mut spans = Vec::new();
            if let Some(a) = &t.archived {
                spans.push(Span::styled(format!("[{a}] "), dim()));
            }
            spans.push(Span::styled(format!("{icon} "), Style::default().fg(color)));
            spans.push(Span::styled(t.id.clone(), if t.archived.is_some() { dim() } else { Style::default() }));
            spans.push(Span::styled(format!("  {}", t.title), dim()));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let project = app.current_project().map_or("—", |p| p.project.as_str());
    let mut title = format!(" Tasks: {project} ");
    if app.show_archived {
        title.push_str("+arquivadas ");
    }
    if !app.filter.is_empty() {
        title.push_str(&format!("/{} ", app.filter));
    }
    let mut state = ListState::default();
    if !tasks.is_empty() {
        state.select(Some(app.task_idx));
    }
    let list = List::new(items).block(pane_block(title, app.pane == Pane::Tasks)).highlight_style(highlight());
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = pane_block(" Preview ".into(), false);
    let Some(t) = app.current_task() else {
        f.render_widget(Paragraph::new(Span::styled("sem task selecionada", dim())).block(block), area);
        return;
    };
    let st = app.status_of(t.status_num);
    let color = st.map_or(Color::Red, |s| hl_color(&s.hl_group));

    let mut lines = vec![
        Line::from(Span::styled(t.title.clone(), Style::default().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled(format!("{} {}", st.map_or("?", |s| s.icon.as_str()), t.status), Style::default().fg(color)),
            Span::styled(format!("  ·  {}", t.id), dim()),
            Span::styled(t.archived.as_ref().map(|a| format!("  ·  archived/{a}")).unwrap_or_default(), dim()),
        ]),
    ];
    if !t.desc.is_empty() {
        lines.push(Line::from(Span::styled(t.desc.clone(), Style::default().add_modifier(Modifier::ITALIC))));
    }
    lines.push(Line::from(Span::styled("─".repeat(area.width.saturating_sub(2) as usize), dim())));

    if let Some((path, content)) = &app.preview
        && *path == t.path
    {
        for l in content.lines() {
            let style = if l.starts_with('#') {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else if l.starts_with('>') {
                Style::default().fg(color)
            } else if l == "---" {
                dim()
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(l.to_string(), style)));
        }
    }

    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false }).scroll((app.preview_scroll, 0));
    f.render_widget(p, area);
}

fn draw_bar(f: &mut Frame, app: &App, area: Rect) {
    let line = if app.mode == Mode::Filter {
        let text = format!("/{}", app.filter);
        let w = Span::raw(text.as_str()).width() as u16;
        f.set_cursor_position((area.x + w, area.y));
        Line::from(Span::styled(text, Style::default().fg(Color::Yellow)))
    } else if let Some(m) = &app.message {
        let color = if m.error { Color::Red } else { Color::Green };
        Line::from(Span::styled(m.text.clone(), Style::default().fg(color)))
    } else {
        Line::from(Span::styled(
            "? ajuda · n nova · s fase · a arquivar · d apagar · e abrir · / filtro · q sair",
            dim(),
        ))
    };
    f.render_widget(Paragraph::new(line), area);
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

fn modal(title: String) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD)))
}

fn draw_new_task(f: &mut Frame, app: &App, nt: &NewTask) {
    let project = app.current_project().map_or("", |p| p.project.as_str());
    let area = centered(f.area(), 64, 5);
    f.render_widget(Clear, area);
    let block = modal(format!(" Nova task em {project} "))
        .title_bottom(Line::from(" Tab próximo · Enter no último cria · Esc cancela ").style(dim()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    const LABELS: [&str; 3] = ["Título ", "Id     ", "Estado "];
    let values = [&nt.title, &nt.id, &nt.desc];
    let lines: Vec<Line> = LABELS
        .iter()
        .zip(values)
        .enumerate()
        .map(|(i, (label, value))| {
            let ls = if i == nt.field {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                dim()
            };
            Line::from(vec![Span::styled(*label, ls), Span::raw(value.clone())])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
    let x = inner.x + Span::raw(LABELS[nt.field]).width() as u16 + Span::raw(values[nt.field].as_str()).width() as u16;
    f.set_cursor_position((x.min(inner.right().saturating_sub(1)), inner.y + nt.field as u16));
}

fn draw_prompt(f: &mut Frame, kind: PromptKind, value: &str) {
    let title = match kind {
        PromptKind::Title => " Título ",
        PromptKind::Desc => " Descrição do estado (vazio apaga) ",
        PromptKind::Rename => " Novo id (renomeia arquivo e links do vault) ",
        PromptKind::NewProject => " Novo projeto ",
    };
    let area = centered(f.area(), 64, 3);
    f.render_widget(Clear, area);
    let block = modal(title.into()).title_bottom(Line::from(" Enter confirma · Esc cancela ").style(dim()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(value.to_string()), inner);
    let x = inner.x + Span::raw(value).width() as u16;
    f.set_cursor_position((x.min(inner.right().saturating_sub(1)), inner.y));
}

fn draw_pick(f: &mut Frame, app: &App, kind: PickKind, options: &[(String, String)], idx: usize) {
    let id = app.current_task().map_or("", |t| t.id.as_str());
    let title = match kind {
        PickKind::Status => format!(" Fase de {id} "),
        PickKind::Archive => format!(" Arquivar {id} como "),
    };
    let area = centered(f.area(), 44, options.len() as u16 + 2);
    f.render_widget(Clear, area);
    let items: Vec<ListItem> = options
        .iter()
        .map(|(value, label)| {
            let color = match kind {
                PickKind::Status => app
                    .data
                    .statuses
                    .iter()
                    .find(|s| s.callout == *value)
                    .map_or(Color::Reset, |s| hl_color(&s.hl_group)),
                PickKind::Archive => {
                    if ARCHIVE_TYPES[0].0 == value { Color::Green } else { Color::Reset }
                }
            };
            ListItem::new(Span::styled(label.clone(), Style::default().fg(color)))
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(idx));
    let list = List::new(items).block(modal(title)).highlight_style(highlight()).highlight_symbol("› ");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_confirm(f: &mut Frame, app: &App, kind: ConfirmKind) {
    let id = app.current_task().map_or("", |t| t.id.as_str());
    let text = match kind {
        ConfirmKind::Delete => format!("Apagar {id}? Links para ela ficam pendurados."),
        ConfirmKind::Unarchive => format!("Desarquivar {id}?"),
    };
    let area = centered(f.area(), (text.chars().count() as u16 + 4).max(30), 3);
    f.render_widget(Clear, area);
    let block = modal(" Confirmar ".into()).title_bottom(Line::from(" y sim · qualquer outra não ").style(dim()));
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_help(f: &mut Frame) {
    let area = centered(f.area(), 50, HELP.len() as u16 + 2);
    f.render_widget(Clear, area);
    let lines: Vec<Line> = HELP
        .iter()
        .map(|(k, d)| {
            Line::from(vec![Span::styled(format!("{k:>14}  "), Style::default().fg(Color::Yellow)), Span::raw(*d)])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(modal(" Teclas ".into())), area);
}
