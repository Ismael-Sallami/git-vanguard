// ==============================================================================
// GitVanguard - Capa de Renderizado TUI (Terminal User Interface)
// Diseño Neo-Brutalista y Minimalista de alto contraste optimizado para Linux.
// Bordes Unicode redondeados, jerarquía visual nítida y visualización de diffs.
// ==============================================================================

use crate::app::{ActiveModal, App, Tab, CONVENTIONAL_TYPES};
use crate::git::FileState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Paleta de color curada para alto contraste y elegancia en terminales modernas.
pub struct Palette;
impl Palette {
    pub const ACCENT: Color = Color::Rgb(6, 182, 212); // Cyan 500 (Vibrante)
    pub const SUCCESS: Color = Color::Rgb(34, 197, 94); // Emerald 500
    pub const WARNING: Color = Color::Rgb(234, 179, 8); // Amber 500
    pub const DANGER: Color = Color::Rgb(239, 68, 68);  // Rose 500
    pub const MUTED: Color = Color::Rgb(100, 116, 139); // Slate 500 (Legible)
    pub const TEXT: Color = Color::Rgb(248, 250, 252);  // Slate 50
    pub const TEXT_DIM: Color = Color::Rgb(148, 163, 184); // Slate 400
    pub const HIGHLIGHT_BG: Color = Color::Rgb(30, 41, 59); // Slate 800
    pub const BORDER: Color = Color::Rgb(51, 65, 85);    // Slate 700
    pub const BORDER_ACTIVE: Color = Color::Rgb(6, 182, 212); // Cyan 500
}

/// Dibuja la interfaz completa en el frame activo.
pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Layout principal: Barra superior (3), Contenido central (Columnas), Barra de atajos inferior (3)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Cabecera & Tabs
            Constraint::Min(10),   // Paneles centrales
            Constraint::Length(3), // Barra de estado / Shortcuts contextuales (3 líneas para borde + texto)
        ])
        .split(size);

    render_header(f, app, main_chunks[0]);
    render_body(f, app, main_chunks[1]);
    render_footer(f, app, main_chunks[2]);

    // Renderizado de modales superpuestos si existen
    if !matches!(app.modal, ActiveModal::None) {
        render_modal_overlay(f, app, size);
    }
}

/// Renderiza la barra superior con información de la rama, upstream y pestañas.
fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16), // Logo compacto: " GV VANGUARD "
            Constraint::Min(40),    // Pestañas (reciben todo el ancho disponible)
            Constraint::Length(28), // Estado de repo / upstream compacto
        ])
        .split(area);

    // Bloque 1: Brand / Logo
    let brand_spans = vec![
        Span::styled(" GV ", Style::default().bg(Palette::ACCENT).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD)),
        Span::styled(" VANGUARD", Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
    ];
    let brand = Paragraph::new(Line::from(brand_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::BORDER))
                .title(" Motor "),
        );
    f.render_widget(brand, header_chunks[0]);

    // Bloque 2: Tabs horizontales con ventana deslizante (Garantiza visibilidad en cualquier resolución)
    let inner_width = header_chunks[1].width.saturating_sub(4) as usize;
    let active_idx = app.active_tab.to_index();

    let tab_items = [
        (0, " 1. Archivos "),
        (1, " 2. Ramas "),
        (2, " 3. Historial "),
        (3, " 4. Worktrees "),
        (4, " 5. Stash "),
        (5, " 6. Reflog "),
    ];

    let total_len: usize = tab_items.iter().map(|(_, t)| t.len()).sum::<usize>() + (tab_items.len() - 1);

    let (start_idx, end_idx) = if total_len <= inner_width {
        (0, 6)
    } else {
        let mut best_start = 0;
        let mut best_end = 6;
        for s in 0..6 {
            for e in (s + 1)..=6 {
                if s <= active_idx && active_idx < e {
                    let sub_len = tab_items[s..e].iter().map(|(_, t)| t.len()).sum::<usize>() + (e - s - 1);
                    if sub_len <= inner_width && (e - s > best_end - best_start || (best_end == 6 && best_start == 0)) {
                        best_start = s;
                        best_end = e;
                    }
                }
            }
        }
        (best_start, best_end)
    };

    let mut tab_spans = Vec::new();
    if start_idx > 0 {
        tab_spans.push(Span::styled("◀ ", Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)));
    }

    for (pos, i) in (start_idx..end_idx).enumerate() {
        let tab = Tab::from_index(i);
        let is_active = i == active_idx;

        if is_active {
            tab_spans.push(Span::styled(
                format!(" {} ", tab.short_title()),
                Style::default()
                    .bg(Palette::ACCENT)
                    .fg(Color::Rgb(15, 23, 42))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!(" {} ", tab.short_title()),
                Style::default()
                    .bg(Color::Rgb(24, 32, 47))
                    .fg(Palette::TEXT_DIM),
            ));
        }

        if pos + 1 < (end_idx - start_idx) {
            tab_spans.push(Span::raw(" "));
        }
    }

    if end_idx < 6 {
        tab_spans.push(Span::styled(" ▶", Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)));
    }

    let tabs_title = format!(" Pestaña [{}/6: {}] ", active_idx + 1, app.active_tab.short_title());
    let tabs_widget = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
                .title(tabs_title),
        );
    f.render_widget(tabs_widget, header_chunks[1]);

    // Bloque 3: Resumen de Sincronización Remota
    let branch_name = &app.overview.branch;
    let sync_info = if let Some(ref _ups) = app.overview.upstream {
        format!("↑{} ↓{}", app.overview.ahead, app.overview.behind)
    } else {
        "local".to_string()
    };

    let status_badge = if app.overview.is_clean {
        Span::styled(" OK ", Style::default().bg(Palette::SUCCESS).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" MOD ", Style::default().bg(Palette::WARNING).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD))
    };

    let sync_spans = vec![
        status_badge,
        Span::styled(format!(" * {} ", branch_name), Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(format!("({})", sync_info), Style::default().fg(Palette::MUTED)),
    ];
    let sync_widget = Paragraph::new(Line::from(sync_spans))
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::BORDER))
                .title(" Repositorio "),
        );
    f.render_widget(sync_widget, header_chunks[2]);
}

/// Renderiza el cuerpo principal dividido en dos columnas: navegador izquierdo y visor de diff derecho.
fn render_body(f: &mut Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42), // Columna izquierda: Navegador según pestaña activa
            Constraint::Percentage(58), // Columna derecha: Viewport de Diferencias / Inspección
        ])
        .split(area);

    // Renderizar panel izquierdo dinámico
    match app.active_tab {
        Tab::Files => render_files_panel(f, app, body_chunks[0]),
        Tab::Branches => render_branches_panel(f, app, body_chunks[0]),
        Tab::Commits => render_commits_panel(f, app, body_chunks[0]),
        Tab::Worktrees => render_worktrees_panel(f, app, body_chunks[0]),
        Tab::Stashes => render_stashes_panel(f, app, body_chunks[0]),
        Tab::TimeMachine => render_timemachine_panel(f, app, body_chunks[0]),
    }

    // Renderizar visor de diffs derecho
    render_diff_viewport(f, app, body_chunks[1]);
}

/// Panel 1: Lista de archivos staged y unstaged.
fn render_files_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Unstaged / Untracked
            Constraint::Percentage(50), // Staged
        ])
        .split(area);

    // 1. Archivos No Preparados (Unstaged / Untracked)
    let unstaged_items: Vec<ListItem> = app
        .unstaged_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = !app.files_in_staged_view && idx == app.selected_unstaged;
            let (icon, color) = match file.state {
                FileState::Untracked => ("?", Palette::WARNING),
                FileState::Conflicted => ("!", Palette::DANGER),
                _ => ("M", Palette::ACCENT),
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[{}] ", icon), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(&file.path, style),
            ])).style(row_style)
        })
        .collect();

    let unstaged_border_color = if !app.files_in_staged_view {
        Palette::BORDER_ACTIVE
    } else {
        Palette::BORDER
    };
    let unstaged_title = if !app.files_in_staged_view {
        format!(" ● Cambios de Trabajo [Unstaged: {}] (Space: Stage, a: Todo) ", app.unstaged_files.len())
    } else {
        format!(" ○ Cambios de Trabajo [Unstaged: {}] ", app.unstaged_files.len())
    };

    let unstaged_widget = List::new(unstaged_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(unstaged_border_color))
            .title(unstaged_title),
    );
    f.render_widget(unstaged_widget, chunks[0]);

    // 2. Archivos Preparados (Staged)
    let staged_items: Vec<ListItem> = app
        .staged_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = app.files_in_staged_view && idx == app.selected_staged;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[{}] ", file.index_status), Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(&file.path, style),
            ])).style(row_style)
        })
        .collect();

    let staged_border_color = if app.files_in_staged_view {
        Palette::SUCCESS
    } else {
        Palette::BORDER
    };
    let staged_title = if app.files_in_staged_view {
        format!(" ● Cambios Preparados [Staged: {}] (Space: Unstage, c: Commit) ", app.staged_files.len())
    } else {
        format!(" ○ Cambios Preparados [Staged: {}] ", app.staged_files.len())
    };

    let staged_widget = List::new(staged_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(staged_border_color))
            .title(staged_title),
    );
    f.render_widget(staged_widget, chunks[1]);
}

/// Panel 2: Lista de ramas locales y remotas.
fn render_branches_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .branches
        .iter()
        .enumerate()
        .map(|(idx, b)| {
            let is_selected = idx == app.selected_branch;
            let prefix = if is_selected { "▶ " } else { "  " };
            let head_marker = if b.is_head { "* " } else { "  " };

            let branch_color = if b.is_head {
                Palette::SUCCESS
            } else if b.is_remote {
                Palette::MUTED
            } else {
                Palette::ACCENT
            };

            let sync_tag = if b.ahead > 0 || b.behind > 0 {
                format!(" [↑{} ↓{}]", b.ahead, b.behind)
            } else if let Some(ref u) = b.upstream {
                format!(" [{}]", u)
            } else {
                String::new()
            };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(head_marker, Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(&b.name, Style::default().fg(branch_color).add_modifier(if b.is_head || is_selected { Modifier::BOLD } else { Modifier::empty() })),
                Span::styled(sync_tag, Style::default().fg(Palette::WARNING)),
                Span::styled(format!(" - {}", b.last_commit_msg), Style::default().fg(Palette::MUTED)),
            ])).style(row_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" ● Ramas Locales y Remotas [Total: {}] (Space: Checkout, m: Merge, r: Rebase) ", app.branches.len())),
    );
    f.render_widget(list, area);
}

/// Panel 3: Historial de commits (DAG Graph).
fn render_commits_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .commits
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let is_selected = idx == app.selected_commit;
            let prefix = if is_selected { "▶ " } else { "  " };
            let head_icon = if c.is_head { "● " } else { "○ " };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(head_icon, Style::default().fg(if c.is_head { Palette::SUCCESS } else { Palette::MUTED })),
                Span::styled(format!("{} ", c.short_hash), Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(&c.message, Style::default().fg(Palette::TEXT).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() })),
                Span::styled(format!(" ({}, {})", c.author, c.date), Style::default().fg(Palette::MUTED)),
            ])).style(row_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" ● Historial de Commits [Recientes: {}] (j/k: Navegar) ", app.commits.len())),
    );
    f.render_widget(list, area);
}

/// Panel 4: Gestor de Worktrees.
fn render_worktrees_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(idx, wt)| {
            let is_selected = idx == app.selected_worktree;
            let prefix = if is_selected { "▶ " } else { "  " };
            let main_tag = if wt.is_main { " [Principal]" } else { "" };
            let locked_tag = if wt.is_locked { " [Bloqueado]" } else { "" };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled("[WT] ", Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD)),
                Span::styled(&wt.branch, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({})", wt.path), Style::default().fg(Palette::TEXT_DIM)),
                Span::styled(main_tag, Style::default().fg(Palette::WARNING)),
                Span::styled(locked_tag, Style::default().fg(Palette::DANGER)),
            ])).style(row_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" ● Worktrees Hub [Activos: {}] (n: Crear, d: Eliminar) ", app.worktrees.len())),
    );
    f.render_widget(list, area);
}

/// Panel 5: Stash & Shelves.
fn render_stashes_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .stashes
        .iter()
        .enumerate()
        .map(|(idx, st)| {
            let is_selected = idx == app.selected_stash;
            let prefix = if is_selected { "▶ " } else { "  " };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled("[STASH] ", Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[{}] ", st.name), Style::default().fg(Palette::WARNING)),
                Span::styled(&st.message, Style::default().fg(Palette::TEXT).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() })),
                Span::styled(format!(" ({})", st.date), Style::default().fg(Palette::MUTED)),
            ])).style(row_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" ● Stash & Shelves [Guardados: {}] (s: Guardar, p: Pop, a: Apply, d: Drop) ", app.stashes.len())),
    );
    f.render_widget(list, area);
}

/// Panel 6: Time Machine (Reflog Visualizer).
fn render_timemachine_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .reflogs
        .iter()
        .enumerate()
        .map(|(idx, rf)| {
            let is_selected = idx == app.selected_reflog;
            let prefix = if is_selected { "▶ " } else { "  " };

            let action_color = match rf.action.as_str() {
                "commit" => Palette::SUCCESS,
                "checkout" => Palette::ACCENT,
                "rebase" => Palette::WARNING,
                "reset" => Palette::DANGER,
                _ => Palette::TEXT,
            };

            let row_style = if is_selected {
                Style::default().bg(Palette::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled("[LOG] ", Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<8} ", rf.selector), Style::default().fg(Palette::WARNING)),
                Span::styled(format!("{:<10} ", rf.action), Style::default().fg(action_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} ", rf.hash), Style::default().fg(Palette::MUTED)),
                Span::styled(&rf.message, Style::default().fg(Palette::TEXT).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() })),
                Span::styled(format!(" ({})", rf.time_ago), Style::default().fg(Palette::MUTED)),
            ])).style(row_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" ● Time Machine (Reflog) [Historial: {}] (U: Rebobinar Repositorio) ", app.reflogs.len())),
    );
    f.render_widget(list, area);
}

/// Viewport de diferencias (Diff Viewer) con sintaxis y resaltado contextual.
fn render_diff_viewport(f: &mut Frame, app: &App, area: Rect) {
    let lines = app.diff_content.lines().skip(app.diff_scroll);
    let mut formatted_lines = Vec::new();

    if app.diff_content.trim().is_empty() {
        formatted_lines.push(Line::from(""));
        formatted_lines.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("[i] ", Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("No hay cambios pendientes o diferencias seleccionadas en esta vista.", Style::default().fg(Palette::TEXT_DIM)),
        ]));
        formatted_lines.push(Line::from(vec![
            Span::styled("       ", Style::default()),
            Span::styled("Selecciona un archivo modificado, commit, stash o entrada de reflog para inspeccionar.", Style::default().fg(Palette::MUTED)),
        ]));
    } else {
        for (rel_idx, line) in lines.enumerate() {
            let abs_line_num = app.diff_scroll + rel_idx + 1;
            let line_gutter = format!("{:4} │ ", abs_line_num);

            let (gutter_style, text_style) = if line.starts_with('+') && !line.starts_with("+++") {
                (
                    Style::default().fg(Palette::SUCCESS),
                    Style::default().fg(Palette::SUCCESS).bg(Color::Rgb(6, 44, 25)),
                )
            } else if line.starts_with('-') && !line.starts_with("---") {
                (
                    Style::default().fg(Palette::DANGER),
                    Style::default().fg(Palette::DANGER).bg(Color::Rgb(50, 15, 15)),
                )
            } else if line.starts_with("@@") {
                (
                    Style::default().fg(Palette::WARNING),
                    Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD),
                )
            } else if line.starts_with("diff --git") || line.starts_with("commit ") {
                (
                    Style::default().fg(Palette::ACCENT),
                    Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    Style::default().fg(Palette::MUTED),
                    Style::default().fg(Palette::TEXT),
                )
            };

            formatted_lines.push(Line::from(vec![
                Span::styled(line_gutter, gutter_style),
                Span::styled(line, text_style),
            ]));
        }
    }

    let target_desc = match &app.current_target {
        crate::git::DiffTarget::WorkingFile { path, staged } => {
            if *staged {
                format!("Preparado (Staged): {}", path)
            } else {
                format!("Sin Preparar (Unstaged): {}", path)
            }
        }
        crate::git::DiffTarget::Commit(hash) => {
            let short = if hash.len() > 8 { &hash[..8] } else { hash };
            format!("Commit: {}", short)
        }
        crate::git::DiffTarget::Stash(idx) => format!("Stash: stash@{{{}}}", idx),
        crate::git::DiffTarget::Reflog(sel) => format!("Reflog: {}", sel),
        crate::git::DiffTarget::None => "Ninguno".to_string(),
    };

    let total_lines = app.diff_content.lines().count();
    let scroll_info = if total_lines > 0 {
        format!(" [Línea {}/{}]", app.diff_scroll + 1, total_lines)
    } else {
        String::new()
    };

    let diff_widget = Paragraph::new(formatted_lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER))
            .title(format!(" Inspección de Diferencias │ {}{} (J/K o Ratón) ", target_desc, scroll_info)),
    );
    f.render_widget(diff_widget, area);
}

/// Renderiza la barra inferior con atajos contextuales dinámicos por pestaña activa.
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    if let Some((ref msg, is_error)) = app.status_toast {
        let (icon, badge_color) = if is_error {
            (" ✖ ERROR ", Palette::DANGER)
        } else {
            (" ✔ ÉXITO ", Palette::SUCCESS)
        };
        let text = Line::from(vec![
            Span::styled(icon, Style::default().bg(badge_color).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(msg, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
            Span::styled("   (Cualquier acción actualizará este estado)", Style::default().fg(Palette::MUTED)),
        ]);
        let footer = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if is_error { Palette::DANGER } else { Palette::SUCCESS }))
                .title(" Notificación del Sistema "),
        );
        f.render_widget(footer, area);
        return;
    }

    // Atajos contextuales enriquecidos según la pestaña activa
    let (tab_label, shortcuts): (&str, Vec<(&str, &str)>) = match app.active_tab {
        Tab::Files => (
            "1. Archivos",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("Space", "Stage/Unstage"),
                ("h/l o ←/→", "Alternar Panel"),
                ("a", "Stage Todo"),
                ("u", "Unstage Todo"),
                ("c", "Commit"),
                ("C", "Conv. Commit"),
                ("d", "Descartar"),
                ("?", "Ayuda"),
                ("q", "Salir"),
            ],
        ),
        Tab::Branches => (
            "2. Ramas & Remotos",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("Space", "Checkout"),
                ("n", "Nueva Rama"),
                ("d", "Eliminar Rama"),
                ("m", "Merge"),
                ("r", "Rebase"),
                ("z", "Zombie Pruner"),
                ("p/P", "Pull/Push"),
                ("f", "Fetch"),
                ("q", "Salir"),
            ],
        ),
        Tab::Commits => (
            "3. Historial (DAG)",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("j/k", "Navegar"),
                ("J/K", "Scroll Diff"),
                ("p", "Pull"),
                ("P", "Push"),
                ("R", "Refrescar"),
                ("?", "Ayuda"),
                ("q", "Salir"),
            ],
        ),
        Tab::Worktrees => (
            "4. Worktrees Hub",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("n", "Crear Worktree"),
                ("d", "Eliminar Worktree"),
                ("Enter", "Inspeccionar HEAD"),
                ("R", "Refrescar"),
                ("?", "Ayuda"),
                ("q", "Salir"),
            ],
        ),
        Tab::Stashes => (
            "5. Stash & Shelves",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("s", "Guardar Stash"),
                ("p", "Pop"),
                ("a", "Apply"),
                ("d", "Drop"),
                ("Enter", "Inspeccionar"),
                ("?", "Ayuda"),
                ("q", "Salir"),
            ],
        ),
        Tab::TimeMachine => (
            "6. Time Machine",
            vec![
                ("Tab", "Siguiente Pestaña"),
                ("U", "Rebobinar Repositorio"),
                ("j/k", "Navegar Reflog"),
                ("J/K", "Scroll Diff"),
                ("R", "Refrescar"),
                ("?", "Ayuda"),
                ("q", "Salir"),
            ],
        ),
    };

    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!(" [{}] ", tab_label),
        Style::default().bg(Palette::ACCENT).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(" "));

    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {} ", desc),
            Style::default().fg(Palette::TEXT),
        ));
        if i + 1 < shortcuts.len() {
            spans.push(Span::styled("│ ", Style::default().fg(Palette::MUTED)));
        }
    }

    let footer = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Palette::BORDER_ACTIVE))
            .title(format!(" Atajos Contextuales [Pestaña {}] ", tab_label)),
    );
    f.render_widget(footer, area);
}

/// Renderiza modales interactivos centrados en pantalla.
fn render_modal_overlay(f: &mut Frame, app: &App, screen: Rect) {
    match &app.modal {
        ActiveModal::CommitInput { text } => {
            let area = centered_rect(60, 25, screen);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::ACCENT))
                .title(" Confirmar Cambios (Commit) ");
            let content = vec![
                Line::from(Span::styled("Introduce el mensaje para el commit (Enter para confirmar, Esc para cancelar):", Style::default().fg(Palette::MUTED))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled(text, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Palette::ACCENT)),
                ]),
            ];
            let p = Paragraph::new(content).block(block);
            f.render_widget(p, area);
        }

        ActiveModal::ConventionalCommit {
            step,
            selected_type_idx,
            scope,
            description,
            is_breaking,
        } => {
            let area = centered_rect(75, 65, screen);
            f.render_widget(Clear, area);

            let (cur_type, _) = CONVENTIONAL_TYPES[*selected_type_idx];
            let scope_part = if scope.is_empty() { String::new() } else { format!("({})", scope) };
            let breaking_part = if *is_breaking { "!" } else { "" };
            let preview = format!("{}{}{}: {}", cur_type, scope_part, breaking_part, description);
            let len_color = if preview.len() <= 50 {
                Palette::SUCCESS
            } else if preview.len() <= 72 {
                Palette::WARNING
            } else {
                Palette::DANGER
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::ACCENT))
                .title(" Asistente Semántico Conventional Commits ");

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Paso 1: Tipo  ", Style::default().fg(if *step == 0 { Palette::ACCENT } else { Palette::MUTED })),
                    Span::raw("->  "),
                    Span::styled("Paso 2: Ámbito  ", Style::default().fg(if *step == 1 { Palette::ACCENT } else { Palette::MUTED })),
                    Span::raw("->  "),
                    Span::styled("Paso 3: Descripción  ", Style::default().fg(if *step == 2 { Palette::ACCENT } else { Palette::MUTED })),
                    Span::raw("->  "),
                    Span::styled("Paso 4: Breaking Change", Style::default().fg(if *step == 3 { Palette::ACCENT } else { Palette::MUTED })),
                ]),
                Line::from("─────────────────────────────────────────────────────────────────"),
            ];

            if *step == 0 {
                lines.push(Line::from(Span::styled("Selecciona el tipo de cambio (↑/↓ o j/k, Enter para avanzar):", Style::default().fg(Palette::WARNING))));
                for (idx, (t, desc)) in CONVENTIONAL_TYPES.iter().enumerate() {
                    let is_sel = idx == *selected_type_idx;
                    let prefix = if is_sel { "▶ " } else { "  " };
                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(Palette::ACCENT)),
                        Span::styled(format!("{:<10} ", t), Style::default().fg(if is_sel { Palette::ACCENT } else { Palette::TEXT }).add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() })),
                        Span::styled(*desc, Style::default().fg(Palette::MUTED)),
                    ]));
                }
            } else if *step == 1 {
                lines.push(Line::from(Span::styled("Introduce el ámbito o módulo afectado (opcional, ej. 'auth', 'ui', 'core'):", Style::default().fg(Palette::WARNING))));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Ámbito: ", Style::default().fg(Palette::ACCENT)),
                    Span::styled(scope, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Palette::ACCENT)),
                ]));
            } else if *step == 2 {
                lines.push(Line::from(Span::styled("Escribe la descripción corta en modo imperativo (ej. 'add zero-latency diff caching'):", Style::default().fg(Palette::WARNING))));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Descripción: ", Style::default().fg(Palette::ACCENT)),
                    Span::styled(description, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Palette::ACCENT)),
                ]));
            } else {
                lines.push(Line::from(Span::styled("¿Introduce este commit un cambio incompatible (Breaking Change)? (Espacio para conmutar):", Style::default().fg(Palette::WARNING))));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Breaking Change: ", Style::default().fg(Palette::ACCENT)),
                    Span::styled(if *is_breaking { "[X] SÍ" } else { "[ ] NO" }, Style::default().fg(if *is_breaking { Palette::DANGER } else { Palette::SUCCESS }).add_modifier(Modifier::BOLD)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from("── Vista Previa del Commit ──────────────────────────────────────"));
            lines.push(Line::from(vec![
                Span::styled(&preview, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  ({}/72 chars)", preview.len()), Style::default().fg(len_color)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Enter: Siguiente/Confirmar │ Esc: Cancelar", Style::default().fg(Palette::MUTED))));

            let p = Paragraph::new(lines).block(block);
            f.render_widget(p, area);
        }

        ActiveModal::SentinelAlert { findings, .. } => {
            let area = centered_rect(80, 60, screen);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::DANGER))
                .title(" SENTINEL SHIELD: ALERTA DE SEGURIDAD PRE-COMMIT ");

            let mut lines = vec![
                Line::from(Span::styled("Se han detectado posibles credenciales, claves criptográficas o tokens en los cambios:", Style::default().fg(Palette::DANGER).add_modifier(Modifier::BOLD))),
                Line::from(""),
            ];

            for finding in findings {
                lines.push(Line::from(vec![
                    Span::styled("✖ ", Style::default().fg(Palette::DANGER).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{}:{} ", finding.file, finding.line_number), Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("[{}] ", finding.rule_name), Style::default().fg(Palette::DANGER)),
                    Span::styled(format!("({}): ", finding.description), Style::default().fg(Palette::MUTED)),
                    Span::styled(&finding.matched_excerpt, Style::default().fg(Palette::TEXT)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from("─────────────────────────────────────────────────────────────────"));
            lines.push(Line::from(Span::styled("Por seguridad, el commit ha sido bloqueado preventivamente.", Style::default().fg(Palette::TEXT))));
            lines.push(Line::from(Span::styled("Para abortar y corregir los secretos: Pulsa 'Esc'.", Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD))));
            lines.push(Line::from(Span::styled("Para ignorar bajo tu responsabilidad y forzar commit: Pulsa 'F'.", Style::default().fg(Palette::DANGER))));

            let p = Paragraph::new(lines).block(block);
            f.render_widget(p, area);
        }

        ActiveModal::NewBranch { name } => {
            let area = centered_rect(50, 20, screen);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::ACCENT)).title(" Crear Nueva Rama ");
            let content = vec![
                Line::from("Introduce el nombre de la nueva rama:"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Rama: ", Style::default().fg(Palette::ACCENT)),
                    Span::styled(name, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Palette::ACCENT)),
                ]),
            ];
            f.render_widget(Paragraph::new(content).block(block), area);
        }

        ActiveModal::NewWorktree { path, branch, focus_path } => {
            let area = centered_rect(65, 30, screen);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::ACCENT)).title(" Crear Nuevo Git Worktree ");
            let content = vec![
                Line::from(Span::styled("Configura la ruta de trabajo y la rama destino (Tab para alternar campo):", Style::default().fg(Palette::MUTED))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Ruta (Path): ", Style::default().fg(if *focus_path { Palette::ACCENT } else { Palette::MUTED })),
                    Span::styled(path, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(if *focus_path { "█" } else { "" }, Style::default().fg(Palette::ACCENT)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Rama (Branch): ", Style::default().fg(if !*focus_path { Palette::ACCENT } else { Palette::MUTED })),
                    Span::styled(branch, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(if !*focus_path { "█" } else { "" }, Style::default().fg(Palette::ACCENT)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Enter: Crear Worktree │ Esc: Cancelar", Style::default().fg(Palette::MUTED))),
            ];
            f.render_widget(Paragraph::new(content).block(block), area);
        }

        ActiveModal::NewStash { message } => {
            let area = centered_rect(55, 20, screen);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::WARNING)).title(" Guardar Cambios en Stash ");
            let content = vec![
                Line::from("Mensaje descriptivo para el stash (opcional):"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Palette::WARNING)),
                    Span::styled(message, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Palette::WARNING)),
                ]),
            ];
            f.render_widget(Paragraph::new(content).block(block), area);
        }

        ActiveModal::Confirm { title, message, .. } => {
            let area = centered_rect(55, 30, screen);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::DANGER)).title(format!(" {} ", title));
            let mut lines = Vec::new();
            for l in message.lines() {
                lines.push(Line::from(l));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Y / Enter: Confirmar  ", Style::default().fg(Palette::DANGER).add_modifier(Modifier::BOLD)),
                Span::styled("N / Esc: Cancelar", Style::default().fg(Palette::SUCCESS)),
            ]));
            f.render_widget(Paragraph::new(lines).block(block), area);
        }

        ActiveModal::Help => {
            let area = centered_rect(80, 80, screen);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::ACCENT)).title(" Manual de Atajos de Teclado - GitVanguard ");
            let text = vec![
                Line::from(Span::styled("Navegación y Vistas", Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD))),
                Line::from("  Tab / 1-6      : Cambiar entre las 6 pestañas principales"),
                Line::from("  j / k (↑ / ↓)  : Mover el cursor en la lista activa"),
                Line::from("  h / l (← / →)  : Alternar entre cambios Unstaged y Staged en Archivos"),
                Line::from("  J / K          : Desplazar el visor de diferencias (Diff scroll)"),
                Line::from(""),
                Line::from(Span::styled("Operaciones de Archivos (Pestaña 1)", Style::default().fg(Palette::SUCCESS).add_modifier(Modifier::BOLD))),
                Line::from("  Space          : Preparar o despreparar archivo (Stage / Unstage)"),
                Line::from("  a              : Preparar todos los cambios (git add -A)"),
                Line::from("  u              : Despreparar todos los cambios (git reset)"),
                Line::from("  d              : Descartar cambios del archivo con confirmación"),
                Line::from("  c              : Abrir diálogo de commit rápido"),
                Line::from("  C              : Abrir Asistente Semántico Conventional Commits"),
                Line::from(""),
                Line::from(Span::styled("Ramas y Remotos (Pestaña 2)", Style::default().fg(Palette::WARNING).add_modifier(Modifier::BOLD))),
                Line::from("  Space / Enter  : Cambiar de rama activa (Checkout)"),
                Line::from("  n              : Crear nueva rama"),
                Line::from("  d              : Eliminar rama seleccionada"),
                Line::from("  m              : Fusionar rama (Merge) en la rama activa"),
                Line::from("  r              : Rebase de la rama actual sobre la seleccionada"),
                Line::from("  z              : Zombie Branch Pruner (Purgar ramas ya integradas)"),
                Line::from("  p / P          : Git Pull / Git Push"),
                Line::from("  f              : Git Fetch (--all --prune)"),
                Line::from(""),
                Line::from(Span::styled("Worktrees y Time Machine (Pestañas 4 y 6)", Style::default().fg(Palette::DANGER).add_modifier(Modifier::BOLD))),
                Line::from("  n / d          : Crear / eliminar Git Worktrees"),
                Line::from("  U              : Time Machine Restore (Rebobinar repositorio al punto del reflog)"),
                Line::from(""),
                Line::from(Span::styled("Pulsa Esc o ? para cerrar esta ventana", Style::default().fg(Palette::MUTED))),
            ];
            f.render_widget(Paragraph::new(text).block(block), area);
        }

        ActiveModal::CommandLog { title, content, is_error } => {
            let area = centered_rect(70, 50, screen);
            f.render_widget(Clear, area);
            let color = if *is_error { Palette::DANGER } else { Palette::ACCENT };
            let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(color)).title(format!(" {} ", title));
            let mut lines = Vec::new();
            for l in content.lines() {
                lines.push(Line::from(l));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Pulsa Esc para cerrar", Style::default().fg(Palette::MUTED))));
            f.render_widget(Paragraph::new(lines).block(block), area);
        }

        ActiveModal::None => {}
    }
}

/// Función auxiliar para centrar ventanas rectangulares en la pantalla.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
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
        .split(popup_layout[1])[1]
}
