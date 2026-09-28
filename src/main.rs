// ==============================================================================
// GitVanguard - Controlador Git TUI de Alto Rendimiento para Linux
// Autor: Ismael Sallami Moreno <ismEngineer23@gmail.com>
// Arquitectura: Rust + Ratatui + Crossterm con motor asíncrono y protección Sentinel.
// ==============================================================================

mod app;
mod git;
mod sentinel;
mod ui;

#[cfg(test)]
mod sentinel_tests;

#[cfg(test)]
mod git_tests;

use app::{ActiveModal, App, PendingAction, Tab, CONVENTIONAL_TYPES};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::panic;
use std::time::Duration;

fn print_help() {
    println!("\x1b[1;36mGitVanguard v0.1.0\x1b[0m - Controlador Git TUI de Alto Rendimiento para Linux");
    println!("Desarrollado por Ismael Sallami Moreno <ismEngineer23@gmail.com>\n");
    println!("\x1b[1mMODO DE USO:\x1b[0m");
    println!("    vanguard [OPCIONES] [RUTA]\n");
    println!("\x1b[1mARGUMENTOS:\x1b[0m");
    println!("    [RUTA]              Ruta al repositorio Git (por defecto: directorio actual)\n");
    println!("\x1b[1mOPCIONES:\x1b[0m");
    println!("    -h, --help          Muestra este manual de uso y opciones");
    println!("    -v, --version       Muestra la versión compilada de GitVanguard");
    println!("    --scan-only         Ejecuta el escáner Sentinel Shield sobre los cambios preparados sin abrir la TUI\n");
    println!("\x1b[1mTECLAS RÁPIDAS EN TUI:\x1b[0m");
    println!("    Tab / 1-6           Alternar entre las 6 pestañas principales");
    println!("    Space               Stage/Unstage en archivos, o Checkout en ramas");
    println!("    c / C               Commit estándar / Asistente Conventional Commits");
    println!("    p / P               Git Pull / Git Push");
    println!("    f                   Git Fetch (--all --prune)");
    println!("    n                   Nueva rama / Nuevo worktree / Nuevo stash");
    println!("    U                   Time Machine: Rebobinar repositorio al punto del reflog");
    println!("    ?                   Ayuda completa interactiva");
    println!("    q                   Salir de GitVanguard");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Comprobación de flags inmediatos
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("GitVanguard 0.1.0 (Linux x86_64)");
        return Ok(());
    }

    // --------------------------------------------------------------------------
    // 1. Verificación Estricta de Entorno Linux
    // --------------------------------------------------------------------------
    if !cfg!(target_os = "linux") {
        eprintln!("\x1b[1;31m✖ Error de compatibilidad:\x1b[0m");
        eprintln!("GitVanguard está diseñado y optimizado exclusivamente para entornos Linux");
        eprintln!("y terminales POSIX de alto rendimiento con arquitectura de kernel nativa.");
        std::process::exit(1);
    }

    // Determinar la ruta objetivo del repositorio
    let mut target_dir = std::env::current_dir()?.to_string_lossy().to_string();
    for arg in args.iter().skip(1) {
        if !arg.starts_with('-') {
            target_dir = arg.clone();
            break;
        }
    }

    // --------------------------------------------------------------------------
    // 2. Verificación de Binario Git y Repositorio Local
    // --------------------------------------------------------------------------
    if !git::is_git_repository(&target_dir) {
        eprintln!("\x1b[1;33m[GitVanguard]\x1b[0m");
        eprintln!("El directorio especificado no es un repositorio Git válido.");
        eprintln!("Ruta inspeccionada: {}", target_dir);
        eprintln!("Por favor, sitúate en un proyecto inicializado con 'git init' o 'git clone'.");
        std::process::exit(1);
    }

    // Modo CLI: sólo escaneo de seguridad Sentinel
    if args.iter().any(|a| a == "--scan-only") {
        println!("\x1b[1;36m[GitVanguard] Ejecutando Sentinel Shield en {}\x1b[0m...", target_dir);
        let diff = git::run_git_cmd(&target_dir, &["diff", "--cached", "--color=never"]).unwrap_or_default();
        let findings = sentinel::scan_diff(&diff);
        if findings.is_empty() {
            println!("\x1b[1;32m✔ Ningún secreto ni credencial expuesta en los cambios preparados (staged).\x1b[0m");
        } else {
            eprintln!("\x1b[1;31m✖ Se han detectado {} posibles secretos:\x1b[0m", findings.len());
            for f in findings {
                eprintln!("  - [{}] {}:{} -> {}", f.rule_name, f.file, f.line_number, f.matched_excerpt);
            }
            std::process::exit(2);
        }
        return Ok(());
    }

    // --------------------------------------------------------------------------
    // 3. Configuración de Terminal Raw Mode y Hook de Pánico Seguro
    // --------------------------------------------------------------------------
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Hook para restaurar la terminal en caso de fallo crítico imprevisto
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_panic(panic_info);
    }));

    // --------------------------------------------------------------------------
    // 4. Inicialización del Estado y Bucle Principal de Eventos
    // --------------------------------------------------------------------------
    let mut app = App::new(target_dir);

    let res = run_app(&mut terminal, &mut app);

    // --------------------------------------------------------------------------
    // 5. Restauración Impecable de la Terminal
    // --------------------------------------------------------------------------
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error en la ejecución de GitVanguard: {:?}", err);
    }

    Ok(())
}

/// Bucle reactivo de renderizado y recepción de eventos de teclado y ratón.
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if app.should_quit {
            return Ok(());
        }

        // Tasa de sondeo de 50 ms para respuesta instantánea y mínimo consumo de CPU
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(app, key.code, key.modifiers);
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => app.scroll_diff_down(3),
                        MouseEventKind::ScrollUp => app.scroll_diff_up(3),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

/// Enrutamiento inteligente de eventos de teclado según el contexto y modal activo.
fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Si hay un modal activo, el foco se desvía a la interacción del modal
    match &mut app.modal {
        ActiveModal::CommitInput { text } => match code {
            KeyCode::Enter => {
                let msg = text.clone();
                app.modal = ActiveModal::None;
                app.try_commit(msg);
            }
            KeyCode::Esc => {
                app.modal = ActiveModal::None;
            }
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Char(c) => {
                text.push(c);
            }
            _ => {}
        },

        ActiveModal::ConventionalCommit {
            step,
            selected_type_idx,
            scope,
            description,
            is_breaking,
        } => match code {
            KeyCode::Esc => {
                app.modal = ActiveModal::None;
            }
            KeyCode::Up | KeyCode::Char('k') if *step == 0 => {
                if *selected_type_idx > 0 {
                    *selected_type_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') if *step == 0 => {
                if *selected_type_idx + 1 < CONVENTIONAL_TYPES.len() {
                    *selected_type_idx += 1;
                }
            }
            KeyCode::Char(' ') if *step == 3 => {
                *is_breaking = !*is_breaking;
            }
            KeyCode::Enter => {
                if *step < 3 {
                    *step += 1;
                } else {
                    // Compilar el mensaje final de Conventional Commit
                    let (t, _) = CONVENTIONAL_TYPES[*selected_type_idx];
                    let scope_str = if scope.trim().is_empty() {
                        String::new()
                    } else {
                        format!("({})", scope.trim())
                    };
                    let break_str = if *is_breaking { "!" } else { "" };
                    let full_msg = format!("{}{}{}: {}", t, scope_str, break_str, description.trim());

                    app.modal = ActiveModal::None;
                    app.try_commit(full_msg);
                }
            }
            KeyCode::Backspace => {
                if *step == 1 {
                    scope.pop();
                } else if *step == 2 {
                    description.pop();
                }
            }
            KeyCode::Char(c) => {
                if *step == 1 {
                    scope.push(c);
                } else if *step == 2 {
                    description.push(c);
                }
            }
            _ => {}
        },

        ActiveModal::SentinelAlert {
            pending_commit_msg, ..
        } => match code {
            KeyCode::Esc => {
                app.modal = ActiveModal::None;
                app.set_toast("Commit cancelado preventivamente para sanear secretos.", false);
            }
            KeyCode::Char('F') | KeyCode::Char('f') => {
                let msg = pending_commit_msg.clone();
                app.modal = ActiveModal::None;
                app.execute_commit(&msg);
            }
            _ => {}
        },

        ActiveModal::NewBranch { name } => match code {
            KeyCode::Enter => {
                let branch_name = name.clone();
                app.modal = ActiveModal::None;
                if !branch_name.trim().is_empty() {
                    match git::create_branch(&app.cwd, &branch_name) {
                        Ok(_) => app.set_toast(&format!("Nueva rama creada: {}", branch_name), false),
                        Err(e) => app.set_toast(&e, true),
                    }
                    app.refresh_all();
                }
            }
            KeyCode::Esc => app.modal = ActiveModal::None,
            KeyCode::Backspace => {
                name.pop();
            }
            KeyCode::Char(c) => {
                name.push(c);
            }
            _ => {}
        },

        ActiveModal::NewWorktree {
            path,
            branch,
            focus_path,
        } => match code {
            KeyCode::Tab => {
                *focus_path = !*focus_path;
            }
            KeyCode::Enter => {
                let wt_path = path.clone();
                let wt_branch = branch.clone();
                app.modal = ActiveModal::None;
                if !wt_path.trim().is_empty() && !wt_branch.trim().is_empty() {
                    match git::add_worktree(&app.cwd, &wt_path, &wt_branch) {
                        Ok(_) => app.set_toast(&format!("Worktree vinculado en: {}", wt_path), false),
                        Err(e) => app.set_toast(&e, true),
                    }
                    app.refresh_all();
                }
            }
            KeyCode::Esc => app.modal = ActiveModal::None,
            KeyCode::Backspace => {
                if *focus_path {
                    path.pop();
                } else {
                    branch.pop();
                }
            }
            KeyCode::Char(c) => {
                if *focus_path {
                    path.push(c);
                } else {
                    branch.push(c);
                }
            }
            _ => {}
        },

        ActiveModal::NewStash { message } => match code {
            KeyCode::Enter => {
                let msg = message.clone();
                app.modal = ActiveModal::None;
                match git::stash_save(&app.cwd, &msg) {
                    Ok(_) => app.set_toast("Cambios guardados en el stash.", false),
                    Err(e) => app.set_toast(&e, true),
                }
                app.refresh_all();
            }
            KeyCode::Esc => app.modal = ActiveModal::None,
            KeyCode::Backspace => {
                message.pop();
            }
            KeyCode::Char(c) => {
                message.push(c);
            }
            _ => {}
        },

        ActiveModal::Confirm { action, .. } => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let act = action.clone();
                app.execute_pending_action(act);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.modal = ActiveModal::None;
            }
            _ => {}
        },

        ActiveModal::Help | ActiveModal::CommandLog { .. } => match code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                app.modal = ActiveModal::None;
            }
            _ => {}
        },

        // ----------------------------------------------------------------------
        // Vista Principal (Navegación Global y Acciones)
        // ----------------------------------------------------------------------
        ActiveModal::None => match code {
            // Salida
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('?') => app.modal = ActiveModal::Help,

            // Navegación de Pestañas
            KeyCode::Tab => app.next_tab(),
            KeyCode::BackTab => app.prev_tab(),
            KeyCode::Char('1') => app.set_tab(0),
            KeyCode::Char('2') => app.set_tab(1),
            KeyCode::Char('3') => app.set_tab(2),
            KeyCode::Char('4') => app.set_tab(3),
            KeyCode::Char('5') => app.set_tab(4),
            KeyCode::Char('6') => app.set_tab(5),

            // Movimiento vertical en listas
            KeyCode::Char('j') | KeyCode::Down => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up => app.move_up(),

            // Desplazamiento del visor de diferencias
            KeyCode::Char('J') => app.scroll_diff_down(5),
            KeyCode::Char('K') => app.scroll_diff_up(5),

            // Movimiento horizontal en pestaña de archivos
            KeyCode::Char('h') | KeyCode::Left if app.active_tab == Tab::Files => {
                app.files_in_staged_view = false;
                app.update_current_diff();
            }
            KeyCode::Char('l') | KeyCode::Right if app.active_tab == Tab::Files => {
                app.files_in_staged_view = true;
                app.update_current_diff();
            }

            // Staging individual o cambio de rama
            KeyCode::Char(' ') => match app.active_tab {
                Tab::Files => app.toggle_stage_current(),
                Tab::Branches => app.checkout_selected_branch(),
                _ => {}
            },

            // Staging global
            KeyCode::Char('a') if app.active_tab == Tab::Files => app.stage_all_files(),
            KeyCode::Char('u') if app.active_tab == Tab::Files => app.unstage_all_files(),

            // Descarte / Eliminación con confirmación
            KeyCode::Char('d') => match app.active_tab {
                Tab::Files => {
                    let file_to_discard = if app.files_in_staged_view {
                        app.staged_files.get(app.selected_staged).map(|f| f.path.clone())
                    } else {
                        app.unstaged_files.get(app.selected_unstaged).map(|f| f.path.clone())
                    };
                    if let Some(path) = file_to_discard {
                        app.modal = ActiveModal::Confirm {
                            title: "Descartar Cambios".to_string(),
                            message: format!("¿Seguro que deseas descartar todos los cambios locales en '{}'? Esta acción no se puede deshacer.", path),
                            action: PendingAction::DiscardFile(path),
                        };
                    }
                }
                Tab::Branches => {
                    if let Some(b) = app.branches.get(app.selected_branch) {
                        if !b.is_head && !b.is_remote {
                            let name = b.name.clone();
                            app.modal = ActiveModal::Confirm {
                                title: "Eliminar Rama".to_string(),
                                message: format!("¿Deseas eliminar la rama local '{}'?", name),
                                action: PendingAction::DeleteBranch(name, false),
                            };
                        }
                    }
                }
                Tab::Worktrees => {
                    if let Some(wt) = app.worktrees.get(app.selected_worktree) {
                        if !wt.is_main {
                            let path = wt.path.clone();
                            app.modal = ActiveModal::Confirm {
                                title: "Eliminar Git Worktree".to_string(),
                                message: format!("¿Deseas desvincular y eliminar el worktree en '{}'?", path),
                                action: PendingAction::RemoveWorktree(path),
                            };
                        }
                    }
                }
                Tab::Stashes => {
                    if let Some(st) = app.stashes.get(app.selected_stash) {
                        let idx = st.index;
                        let _ = git::stash_drop(&app.cwd, idx);
                        app.set_toast("Entrada de stash eliminada.", false);
                        app.refresh_all();
                    }
                }
                _ => {}
            },

            // Creación guiada de elementos
            KeyCode::Char('n') => match app.active_tab {
                Tab::Branches => {
                    app.modal = ActiveModal::NewBranch { name: String::new() };
                }
                Tab::Worktrees => {
                    app.modal = ActiveModal::NewWorktree {
                        path: String::new(),
                        branch: String::new(),
                        focus_path: true,
                    };
                }
                Tab::Stashes => {
                    app.modal = ActiveModal::NewStash { message: String::new() };
                }
                _ => {}
            },

            // Confirmación de cambios (Commit)
            KeyCode::Char('c') => {
                app.modal = ActiveModal::CommitInput { text: String::new() };
            }
            KeyCode::Char('C') => {
                app.modal = ActiveModal::ConventionalCommit {
                    step: 0,
                    selected_type_idx: 0,
                    scope: String::new(),
                    description: String::new(),
                    is_breaking: false,
                };
            }

            // Operaciones remotas
            KeyCode::Char('p') => app.git_pull(),
            KeyCode::Char('P') => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    app.git_push(false);
                } else {
                    app.git_push(false);
                }
            }
            KeyCode::Char('f') => app.git_fetch(),

            // Fusión y Rebase de ramas
            KeyCode::Char('m') if app.active_tab == Tab::Branches => {
                if let Some(b) = app.branches.get(app.selected_branch) {
                    let branch_name = b.name.clone();
                    match git::merge_branch(&app.cwd, &branch_name) {
                        Ok(msg) => app.set_toast(&format!("Merge: {}", msg.lines().next().unwrap_or("Completado")), false),
                        Err(e) => app.set_toast(&format!("Error en merge: {}", e), true),
                    }
                    app.refresh_all();
                }
            }
            KeyCode::Char('r') if app.active_tab == Tab::Branches => {
                if let Some(b) = app.branches.get(app.selected_branch) {
                    let branch_name = b.name.clone();
                    match git::rebase_branch(&app.cwd, &branch_name) {
                        Ok(msg) => app.set_toast(&format!("Rebase: {}", msg.lines().next().unwrap_or("Completado")), false),
                        Err(e) => app.set_toast(&format!("Error en rebase: {}", e), true),
                    }
                    app.refresh_all();
                }
            }

            // Zombie Branch Pruner
            KeyCode::Char('z') if app.active_tab == Tab::Branches => {
                app.prompt_prune_merged_branches();
            }

            // Time Machine Restore
            KeyCode::Char('U') if app.active_tab == Tab::TimeMachine => {
                if let Some(rf) = app.reflogs.get(app.selected_reflog) {
                    let sel = rf.selector.clone();
                    app.modal = ActiveModal::Confirm {
                        title: "Time Machine: Restaurar Repositorio".to_string(),
                        message: format!(
                            "¿Confirmas restaurar el repositorio al estado histórico '{}' ({})?\nTodos los archivos volverán exactamente a ese instante temporal.",
                            sel, rf.hash
                        ),
                        action: PendingAction::RestoreReflog(sel),
                    };
                }
            }

            // Refresco manual
            KeyCode::Char('R') => {
                app.refresh_all();
                app.set_toast("Estado del repositorio actualizado.", false);
            }

            _ => {}
        },
    }
}
