// ==============================================================================
// GitVanguard - Capa de Abstracción e Integración con el Motor Git
// Arquitectura de alto rendimiento: comunicación asíncrona y directa con el CLI
// de Git del sistema para preservar 100% de compatibilidad con configuraciones,
// agentes SSH, claves GPG de firma y filtros de repositorio.
// ==============================================================================

use std::path::Path;
use std::process::Command;

/// Representa el estado de un archivo dentro del árbol de trabajo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Staged,
    Unstaged,
    Untracked,
    Conflicted,
}

/// Estado detallado de un fichero modificado o en seguimiento.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GitFile {
    pub path: String,
    pub index_status: char,
    pub worktree_status: char,
    pub state: FileState,
    pub is_staged: bool,
}

/// Información agregada de la rama y el estado de sincronización remota.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct RepoOverview {
    pub root_dir: String,
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub is_clean: bool,
}

/// Representa una rama local o remota.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit_hash: String,
    pub last_commit_msg: String,
}

/// Representa una entrada del historial de commits.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub is_head: bool,
    pub graph_prefix: String,
}

/// Representa un árbol de trabajo vinculado (Git Worktree).
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: String,
    pub head_hash: String,
    pub branch: String,
    pub is_main: bool,
    pub is_locked: bool,
}

/// Representa una entrada del stash.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StashInfo {
    pub index: usize,
    pub name: String,
    pub branch: String,
    pub message: String,
    pub date: String,
}

/// Representa un registro del historial de acciones temporales (Reflog).
#[derive(Debug, Clone)]
pub struct ReflogInfo {
    pub selector: String,
    pub hash: String,
    pub action: String,
    pub message: String,
    pub time_ago: String,
}

/// Objetivo para la generación de diffs en el viewport principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffTarget {
    WorkingFile { path: String, staged: bool },
    Commit(String),
    Stash(usize),
    Reflog(String),
    None,
}

/// Ejecuta un comando Git en el directorio de trabajo y retorna (éxito, stdout, stderr).
pub fn run_git_cmd(cwd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|e| format!("Error al ejecutar 'git {}': {}", args.join(" "), e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        if err_msg.trim().is_empty() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(err_msg.trim().to_string())
        }
    }
}

/// Verifica si la ruta actual es parte de un repositorio Git válido.
pub fn is_git_repository(cwd: &str) -> bool {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output();

    match output {
        Ok(out) => out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true",
        Err(_) => false,
    }
}

/// Obtiene la raíz absoluta del repositorio Git.
pub fn get_repository_root(cwd: &str) -> Result<String, String> {
    let out = run_git_cmd(cwd, &["rev-parse", "--show-toplevel"])?;
    Ok(out.trim().to_string())
}

/// Obtiene un resumen completo del estado general del repositorio.
pub fn get_repo_overview(cwd: &str) -> RepoOverview {
    let root_dir = get_repository_root(cwd).unwrap_or_else(|_| cwd.to_string());
    
    // Obtener rama activa
    let branch_out = run_git_cmd(cwd, &["branch", "--show-current"]).unwrap_or_default();
    let mut branch = branch_out.trim().to_string();
    if branch.is_empty() {
        // En caso de HEAD desacoplado
        let short_head = run_git_cmd(cwd, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
        branch = format!("(HEAD desacoplado: {})", short_head.trim());
    }

    // Comprobar upstream y commits por delante/detrás
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;

    if let Ok(rev_out) = run_git_cmd(cwd, &["rev-parse", "--abbrev-ref", "@{upstream}"]) {
        let ups = rev_out.trim().to_string();
        if !ups.is_empty() {
            upstream = Some(ups.clone());
            if let Ok(counts) = run_git_cmd(cwd, &["rev-list", "--left-right", "--count", &format!("HEAD...{}", ups)]) {
                let parts: Vec<&str> = counts.trim().split_whitespace().collect();
                if parts.len() == 2 {
                    ahead = parts[0].parse().unwrap_or(0);
                    behind = parts[1].parse().unwrap_or(0);
                }
            }
        }
    }

    // Comprobar si hay cambios de trabajo
    let status_out = run_git_cmd(cwd, &["status", "--porcelain=v2"]).unwrap_or_default();
    let is_clean = status_out.trim().is_empty();

    RepoOverview {
        root_dir,
        branch,
        upstream,
        ahead,
        behind,
        is_clean,
    }
}

/// Analiza el estado de los archivos mediante `git status --porcelain=v2`.
pub fn get_status_files(cwd: &str) -> Result<(Vec<GitFile>, Vec<GitFile>), String> {
    let output = run_git_cmd(cwd, &["status", "--porcelain=v2", "-uall"])?;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let entry_type = parts[0];
        match entry_type {
            // Ficheros normales rastreados: "1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>"
            "1" => {
                if parts.len() >= 9 {
                    let xy = parts[1];
                    let x = xy.chars().next().unwrap_or('.');
                    let y = xy.chars().nth(1).unwrap_or('.');
                    let path = parts[8..].join(" ");

                    if x != '.' {
                        staged.push(GitFile {
                            path: path.clone(),
                            index_status: x,
                            worktree_status: '.',
                            state: FileState::Staged,
                            is_staged: true,
                        });
                    }

                    if y != '.' {
                        unstaged.push(GitFile {
                            path,
                            index_status: '.',
                            worktree_status: y,
                            state: FileState::Unstaged,
                            is_staged: false,
                        });
                    }
                }
            }
            // Ficheros renombrados o copiados: "2 <XY> ... <path>\t<origPath>"
            "2" => {
                if parts.len() >= 10 {
                    let xy = parts[1];
                    let x = xy.chars().next().unwrap_or('.');
                    let y = xy.chars().nth(1).unwrap_or('.');
                    let path = parts[9..].join(" ");

                    if x != '.' {
                        staged.push(GitFile {
                            path: path.clone(),
                            index_status: x,
                            worktree_status: '.',
                            state: FileState::Staged,
                            is_staged: true,
                        });
                    }
                    if y != '.' {
                        unstaged.push(GitFile {
                            path,
                            index_status: '.',
                            worktree_status: y,
                            state: FileState::Unstaged,
                            is_staged: false,
                        });
                    }
                }
            }
            // Ficheros con conflicto de fusión: "u <XY> ..."
            "u" => {
                if parts.len() >= 11 {
                    let path = parts[10..].join(" ");
                    unstaged.push(GitFile {
                        path,
                        index_status: 'U',
                        worktree_status: 'U',
                        state: FileState::Conflicted,
                        is_staged: false,
                    });
                }
            }
            // Ficheros sin seguimiento (untracked): "? <path>"
            "?" => {
                if parts.len() >= 2 {
                    let path = parts[1..].join(" ");
                    unstaged.push(GitFile {
                        path,
                        index_status: '?',
                        worktree_status: '?',
                        state: FileState::Untracked,
                        is_staged: false,
                    });
                }
            }
            _ => {}
        }
    }

    Ok((staged, unstaged))
}

/// Obtiene la lista completa de ramas locales y remotas.
pub fn get_branches(cwd: &str) -> Result<Vec<BranchInfo>, String> {
    let format = "%(HEAD)|%(refname:short)|%(upstream:short)|%(upstream:track)|%(objectname:short)|%(subject)";
    let output = run_git_cmd(cwd, &["for-each-ref", &format!("--format={}", format), "refs/heads", "refs/remotes"])?;

    let mut branches = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 {
            continue;
        }

        let is_head = parts[0] == "*";
        let full_name = parts[1].to_string();
        let upstream = if parts[2].is_empty() { None } else { Some(parts[2].to_string()) };
        let track = parts[3];
        let hash = parts[4].to_string();
        let msg = parts[5].to_string();

        let is_remote = full_name.starts_with("origin/") || full_name.contains('/');

        let mut ahead = 0;
        let mut behind = 0;

        if !track.is_empty() {
            if let Some(pos) = track.find("ahead ") {
                let rest = &track[pos + 6..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                ahead = num_str.parse().unwrap_or(0);
            }
            if let Some(pos) = track.find("behind ") {
                let rest = &track[pos + 7..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                behind = num_str.parse().unwrap_or(0);
            }
        }

        branches.push(BranchInfo {
            name: full_name,
            is_head,
            is_remote,
            upstream,
            ahead,
            behind,
            last_commit_hash: hash,
            last_commit_msg: msg,
        });
    }

    // Ordenar: rama activa primero, luego locales, luego remotas
    branches.sort_by(|a, b| {
        b.is_head.cmp(&a.is_head)
            .then_with(|| a.is_remote.cmp(&b.is_remote))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(branches)
}

/// Obtiene el historial de commits recientes con información de autor y fecha.
pub fn get_commits(cwd: &str, limit: usize) -> Result<Vec<CommitInfo>, String> {
    let limit_arg = format!("-n{}", limit);
    let format = "%H|%h|%an|%cr|%s";
    let output = run_git_cmd(cwd, &[
        "log",
        &limit_arg,
        &format!("--format={}", format),
        "--topo-order",
    ])?;

    let mut commits = Vec::new();
    let head_hash = run_git_cmd(cwd, &["rev-parse", "HEAD"]).unwrap_or_default().trim().to_string();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            continue;
        }

        let hash = parts[0].to_string();
        let short_hash = parts[1].to_string();
        let author = parts[2].to_string();
        let date = parts[3].to_string();
        let message = parts[4..].join("|");
        let is_head = hash == head_hash;

        commits.push(CommitInfo {
            hash,
            short_hash,
            author,
            date,
            message,
            is_head,
            graph_prefix: String::new(),
        });
    }

    Ok(commits)
}

/// Obtiene la lista de Worktrees de Git configurados.
pub fn get_worktrees(cwd: &str) -> Result<Vec<WorktreeInfo>, String> {
    let output = run_git_cmd(cwd, &["worktree", "list", "--porcelain"])?;
    let mut worktrees = Vec::new();

    let mut current_path = String::new();
    let mut current_hash = String::new();
    let mut current_branch = String::new();
    let mut is_locked = false;
    let mut is_first = true;

    for line in output.lines() {
        if line.is_empty() {
            if !current_path.is_empty() {
                worktrees.push(WorktreeInfo {
                    path: current_path.clone(),
                    head_hash: current_hash.clone(),
                    branch: current_branch.clone(),
                    is_main: is_first,
                    is_locked,
                });
                is_first = false;
                current_path.clear();
                current_hash.clear();
                current_branch.clear();
                is_locked = false;
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = path.trim().to_string();
        } else if let Some(hash) = line.strip_prefix("HEAD ") {
            current_hash = hash.trim().to_string();
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = branch.trim().to_string();
        } else if line == "detached" {
            current_branch = "(detached)".to_string();
        } else if line.starts_with("locked") {
            is_locked = true;
        }
    }

    if !current_path.is_empty() {
        worktrees.push(WorktreeInfo {
            path: current_path,
            head_hash: current_hash,
            branch: current_branch,
            is_main: is_first,
            is_locked,
        });
    }

    Ok(worktrees)
}

/// Obtiene las entradas guardadas en el stash.
pub fn get_stashes(cwd: &str) -> Result<Vec<StashInfo>, String> {
    let format = "%gd|%gn|%cr|%gs";
    let output = run_git_cmd(cwd, &["stash", "list", &format!("--format={}", format)])?;
    let mut stashes = Vec::new();

    for (idx, line) in output.lines().enumerate() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0].to_string();
        let date = parts[2].to_string();
        let message = parts[3].to_string();

        stashes.push(StashInfo {
            index: idx,
            name,
            branch: String::new(),
            message,
            date,
        });
    }

    Ok(stashes)
}

/// Obtiene los registros del Time Machine (Reflog) con timestamps relativos.
pub fn get_reflog(cwd: &str, limit: usize) -> Result<Vec<ReflogInfo>, String> {
    let limit_arg = format!("-n{}", limit);
    let format = "%gD|%h|%gs|%cr";
    let output = run_git_cmd(cwd, &["reflog", &limit_arg, &format!("--format={}", format)])?;
    let mut reflogs = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 {
            continue;
        }

        let selector = parts[0].to_string();
        let hash = parts[1].to_string();
        let raw_msg = parts[2].to_string();
        let time_ago = parts[3].to_string();

        // Extraer la acción (e.g., "commit", "checkout", "rebase", "reset")
        let action = if let Some(idx) = raw_msg.find(':') {
            raw_msg[..idx].trim().to_string()
        } else {
            "action".to_string()
        };

        reflogs.push(ReflogInfo {
            selector,
            hash,
            action,
            message: raw_msg,
            time_ago,
        });
    }

    Ok(reflogs)
}

/// Genera el contenido de diferencias (diff) según el objetivo seleccionado.
pub fn get_diff_content(cwd: &str, target: &DiffTarget) -> Result<String, String> {
    match target {
        DiffTarget::WorkingFile { path, staged } => {
            if *staged {
                run_git_cmd(cwd, &["diff", "--cached", "--color=never", "--", path])
            } else {
                // Comprobar si es un archivo untracked
                let untracked_check = Command::new("git")
                    .current_dir(cwd)
                    .args(["ls-files", "--error-unmatch", path])
                    .output();

                if let Ok(out) = untracked_check {
                    if !out.status.success() {
                        // Archivo untracked nuevo: generar diff contra /dev/null
                        return run_git_cmd(cwd, &["diff", "--no-index", "--color=never", "/dev/null", path]);
                    }
                }
                run_git_cmd(cwd, &["diff", "--color=never", "--", path])
            }
        }
        DiffTarget::Commit(hash) => {
            run_git_cmd(cwd, &["show", "--stat", "--patch", "--color=never", hash])
        }
        DiffTarget::Stash(index) => {
            let stash_ref = format!("stash@{{{}}}", index);
            run_git_cmd(cwd, &["stash", "show", "-p", "--color=never", &stash_ref])
        }
        DiffTarget::Reflog(selector) => {
            run_git_cmd(cwd, &["show", "--stat", "--patch", "--color=never", selector])
        }
        DiffTarget::None => Ok("No hay ningún elemento seleccionado para visualizar el diff.".to_string()),
    }
}

// ==============================================================================
// Acciones y Mutaciones de Repositorio (Staging, Commits, Branches, Worktrees)
// ==============================================================================

pub fn stage_file(cwd: &str, path: &str) -> Result<(), String> {
    run_git_cmd(cwd, &["add", "--", path])?;
    Ok(())
}

pub fn unstage_file(cwd: &str, path: &str) -> Result<(), String> {
    run_git_cmd(cwd, &["restore", "--staged", "--", path])?;
    Ok(())
}

pub fn stage_all(cwd: &str) -> Result<(), String> {
    run_git_cmd(cwd, &["add", "-A"])?;
    Ok(())
}

pub fn unstage_all(cwd: &str) -> Result<(), String> {
    run_git_cmd(cwd, &["reset"])?;
    Ok(())
}

pub fn discard_changes(cwd: &str, path: &str) -> Result<(), String> {
    // Si el archivo no está rastreado, lo eliminamos con seguridad
    let ls_check = Command::new("git")
        .current_dir(cwd)
        .args(["ls-files", "--error-unmatch", path])
        .output();

    if let Ok(out) = ls_check {
        if !out.status.success() {
            let full_path = Path::new(cwd).join(path);
            if full_path.is_file() {
                std::fs::remove_file(full_path).map_err(|e| e.to_string())?;
                return Ok(());
            } else if full_path.is_dir() {
                std::fs::remove_dir_all(full_path).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    run_git_cmd(cwd, &["restore", "--", path])?;
    Ok(())
}

pub fn commit(cwd: &str, message: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["commit", "-m", message])
}

pub fn pull(cwd: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["pull"])
}

pub fn push(cwd: &str, force_with_lease: bool) -> Result<String, String> {
    if force_with_lease {
        run_git_cmd(cwd, &["push", "--force-with-lease"])
    } else {
        run_git_cmd(cwd, &["push"])
    }
}

pub fn fetch_all(cwd: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["fetch", "--all", "--prune"])
}

pub fn checkout_branch(cwd: &str, branch_name: &str) -> Result<String, String> {
    let clean_name = branch_name.trim_start_matches("origin/");
    run_git_cmd(cwd, &["checkout", clean_name])
}

pub fn create_branch(cwd: &str, branch_name: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["checkout", "-b", branch_name])
}

pub fn delete_branch(cwd: &str, branch_name: &str, force: bool) -> Result<String, String> {
    let flag = if force { "-D" } else { "-d" };
    run_git_cmd(cwd, &["branch", flag, branch_name])
}

pub fn merge_branch(cwd: &str, branch_name: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["merge", branch_name])
}

pub fn rebase_branch(cwd: &str, branch_name: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["rebase", branch_name])
}

#[allow(dead_code)]
pub fn abort_rebase(cwd: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["rebase", "--abort"])
}

#[allow(dead_code)]
pub fn continue_rebase(cwd: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["rebase", "--continue"])
}

/// Identifica ramas locales cuyos commits ya han sido incorporados en la rama principal.
pub fn find_merged_branches(cwd: &str, main_branch: &str) -> Result<Vec<String>, String> {
    let output = run_git_cmd(cwd, &["branch", "--merged", main_branch])?;
    let mut to_prune = Vec::new();

    for line in output.lines() {
        let name = line.trim().trim_start_matches('*').trim();
        if name != main_branch && name != "main" && name != "master" && !name.is_empty() {
            to_prune.push(name.to_string());
        }
    }

    Ok(to_prune)
}

pub fn add_worktree(cwd: &str, path: &str, branch: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["worktree", "add", path, branch])
}

pub fn remove_worktree(cwd: &str, path: &str, force: bool) -> Result<String, String> {
    if force {
        run_git_cmd(cwd, &["worktree", "remove", "--force", path])
    } else {
        run_git_cmd(cwd, &["worktree", "remove", path])
    }
}

pub fn stash_save(cwd: &str, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        run_git_cmd(cwd, &["stash", "save", "-u"])
    } else {
        run_git_cmd(cwd, &["stash", "save", "-u", message])
    }
}

#[allow(dead_code)]
pub fn stash_pop(cwd: &str, index: usize) -> Result<String, String> {
    let target = format!("stash@{{{}}}", index);
    run_git_cmd(cwd, &["stash", "pop", &target])
}

#[allow(dead_code)]
pub fn stash_apply(cwd: &str, index: usize) -> Result<String, String> {
    let target = format!("stash@{{{}}}", index);
    run_git_cmd(cwd, &["stash", "apply", &target])
}

pub fn stash_drop(cwd: &str, index: usize) -> Result<String, String> {
    let target = format!("stash@{{{}}}", index);
    run_git_cmd(cwd, &["stash", "drop", &target])
}

#[allow(dead_code)]
pub fn cherry_pick(cwd: &str, commit_hash: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["cherry-pick", commit_hash])
}

#[allow(dead_code)]
pub fn revert_commit(cwd: &str, commit_hash: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["revert", "--no-edit", commit_hash])
}

pub fn reset_to_commit(cwd: &str, commit_hash: &str, mode: &str) -> Result<String, String> {
    run_git_cmd(cwd, &["reset", mode, commit_hash])
}

/// Restaura de forma segura el árbol de trabajo a una entrada concreta del Reflog.
pub fn restore_reflog_point(cwd: &str, selector: &str) -> Result<String, String> {
    // Primero, verificamos que el selector existe
    let target_hash = run_git_cmd(cwd, &["rev-parse", selector])?;
    let clean_hash = target_hash.trim();
    run_git_cmd(cwd, &["reset", "--hard", clean_hash])
}
