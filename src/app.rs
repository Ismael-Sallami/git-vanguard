// ==============================================================================
// GitVanguard - Máquina de Estados y Controlador de la Aplicación
// Orquestación de eventos de teclado, cambios de foco, visualización de diffs,
// asistentes semánticos y ejecución de operaciones del ciclo de vida de Git.
// ==============================================================================

use crate::git::{
    self, BranchInfo, CommitInfo, DiffTarget, GitFile, ReflogInfo, RepoOverview, StashInfo,
    WorktreeInfo,
};
use crate::sentinel::{self, SecretFinding};

/// Pestañas principales de navegación en la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Files = 0,
    Branches = 1,
    Commits = 2,
    Worktrees = 3,
    Stashes = 4,
    TimeMachine = 5,
}

impl Tab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Files,
            1 => Tab::Branches,
            2 => Tab::Commits,
            3 => Tab::Worktrees,
            4 => Tab::Stashes,
            5 => Tab::TimeMachine,
            _ => Tab::Files,
        }
    }

    pub fn to_index(self) -> usize {
        self as usize
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Files => "1. Archivos (Staging)",
            Tab::Branches => "2. Ramas & Remotos",
            Tab::Commits => "3. Historial (DAG)",
            Tab::Worktrees => "4. Worktrees Hub",
            Tab::Stashes => "5. Stash & Shelves",
            Tab::TimeMachine => "6. Time Machine",
        }
    }
}

/// Acciones pendientes que requieren confirmación explícita del usuario.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PendingAction {
    DiscardFile(String),
    DeleteBranch(String, bool),
    PruneMerged(Vec<String>),
    RestoreReflog(String),
    ResetCommit(String, &'static str),
    ForcePush,
    RemoveWorktree(String),
}

/// Modales y asistentes interactivos superpuestos.
#[derive(Debug, Clone)]
pub enum ActiveModal {
    None,
    CommitInput {
        text: String,
    },
    ConventionalCommit {
        step: usize, // 0: tipo, 1: ámbito, 2: descripción, 3: breaking flag
        selected_type_idx: usize,
        scope: String,
        description: String,
        is_breaking: bool,
    },
    NewBranch {
        name: String,
    },
    NewWorktree {
        path: String,
        branch: String,
        focus_path: bool,
    },
    NewStash {
        message: String,
    },
    SentinelAlert {
        findings: Vec<SecretFinding>,
        pending_commit_msg: String,
    },
    Confirm {
        title: String,
        message: String,
        action: PendingAction,
    },
    Help,
    CommandLog {
        title: String,
        content: String,
        is_error: bool,
    },
}

/// Tipos de Conventional Commits estándar.
pub static CONVENTIONAL_TYPES: &[(&str, &str)] = &[
    ("feat", "Nueva funcionalidad o característica de usuario"),
    ("fix", "Corrección de un error o fallo (bugfix)"),
    ("refactor", "Refactorización de código sin alterar comportamiento"),
    ("perf", "Mejora sustancial de rendimiento"),
    ("docs", "Actualización o redacción de documentación"),
    ("style", "Ajustes de formato, espaciado o convenciones"),
    ("test", "Añadir o corregir pruebas unitarias o de integración"),
    ("chore", "Mantenimiento, dependencias o tareas auxiliares"),
    ("ci", "Modificaciones en pipelines de integración continua"),
];

/// Estado general de la aplicación.
pub struct App {
    pub cwd: String,
    pub overview: RepoOverview,
    pub active_tab: Tab,

    // Pestaña 1: Archivos
    pub staged_files: Vec<GitFile>,
    pub unstaged_files: Vec<GitFile>,
    pub files_in_staged_view: bool,
    pub selected_unstaged: usize,
    pub selected_staged: usize,

    // Pestaña 2: Ramas
    pub branches: Vec<BranchInfo>,
    pub selected_branch: usize,

    // Pestaña 3: Commits
    pub commits: Vec<CommitInfo>,
    pub selected_commit: usize,

    // Pestaña 4: Worktrees
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_worktree: usize,

    // Pestaña 5: Stash
    pub stashes: Vec<StashInfo>,
    pub selected_stash: usize,

    // Pestaña 6: Time Machine (Reflog)
    pub reflogs: Vec<ReflogInfo>,
    pub selected_reflog: usize,

    // Viewport de Diferencias (Diff)
    pub diff_content: String,
    pub diff_scroll: usize,
    pub current_target: DiffTarget,

    // Modales y retroalimentación
    pub modal: ActiveModal,
    pub status_toast: Option<(String, bool)>, // (mensaje, es_error)
    pub should_quit: bool,
}

impl App {
    pub fn new(cwd: String) -> Self {
        let mut app = Self {
            cwd,
            overview: RepoOverview::default(),
            active_tab: Tab::Files,

            staged_files: Vec::new(),
            unstaged_files: Vec::new(),
            files_in_staged_view: false,
            selected_unstaged: 0,
            selected_staged: 0,

            branches: Vec::new(),
            selected_branch: 0,

            commits: Vec::new(),
            selected_commit: 0,

            worktrees: Vec::new(),
            selected_worktree: 0,

            stashes: Vec::new(),
            selected_stash: 0,

            reflogs: Vec::new(),
            selected_reflog: 0,

            diff_content: String::new(),
            diff_scroll: 0,
            current_target: DiffTarget::None,

            modal: ActiveModal::None,
            status_toast: None,
            should_quit: false,
        };

        app.refresh_all();
        app
    }

    /// Recarga el estado completo del repositorio Git.
    pub fn refresh_all(&mut self) {
        self.overview = git::get_repo_overview(&self.cwd);

        // Recargar archivos
        if let Ok((staged, unstaged)) = git::get_status_files(&self.cwd) {
            self.staged_files = staged;
            self.unstaged_files = unstaged;
            if self.selected_staged >= self.staged_files.len() && !self.staged_files.is_empty() {
                self.selected_staged = self.staged_files.len() - 1;
            }
            if self.selected_unstaged >= self.unstaged_files.len() && !self.unstaged_files.is_empty() {
                self.selected_unstaged = self.unstaged_files.len() - 1;
            }
        }

        // Recargar ramas
        if let Ok(branches) = git::get_branches(&self.cwd) {
            self.branches = branches;
            if self.selected_branch >= self.branches.len() && !self.branches.is_empty() {
                self.selected_branch = self.branches.len() - 1;
            }
        }

        // Recargar commits
        if let Ok(commits) = git::get_commits(&self.cwd, 50) {
            self.commits = commits;
            if self.selected_commit >= self.commits.len() && !self.commits.is_empty() {
                self.selected_commit = self.commits.len() - 1;
            }
        }

        // Recargar worktrees
        if let Ok(wts) = git::get_worktrees(&self.cwd) {
            self.worktrees = wts;
            if self.selected_worktree >= self.worktrees.len() && !self.worktrees.is_empty() {
                self.selected_worktree = self.worktrees.len() - 1;
            }
        }

        // Recargar stashes
        if let Ok(stashes) = git::get_stashes(&self.cwd) {
            self.stashes = stashes;
            if self.selected_stash >= self.stashes.len() && !self.stashes.is_empty() {
                self.selected_stash = self.stashes.len() - 1;
            }
        }

        // Recargar reflog
        if let Ok(reflogs) = git::get_reflog(&self.cwd, 50) {
            self.reflogs = reflogs;
            if self.selected_reflog >= self.reflogs.len() && !self.reflogs.is_empty() {
                self.selected_reflog = self.reflogs.len() - 1;
            }
        }

        self.update_current_diff();
    }

    /// Actualiza el viewport de diff según el elemento seleccionado en la pestaña activa.
    pub fn update_current_diff(&mut self) {
        let target = match self.active_tab {
            Tab::Files => {
                if self.files_in_staged_view {
                    if let Some(file) = self.staged_files.get(self.selected_staged) {
                        DiffTarget::WorkingFile {
                            path: file.path.clone(),
                            staged: true,
                        }
                    } else {
                        DiffTarget::None
                    }
                } else if let Some(file) = self.unstaged_files.get(self.selected_unstaged) {
                    DiffTarget::WorkingFile {
                        path: file.path.clone(),
                        staged: false,
                    }
                } else {
                    DiffTarget::None
                }
            }
            Tab::Branches => {
                if let Some(branch) = self.branches.get(self.selected_branch) {
                    DiffTarget::Commit(branch.last_commit_hash.clone())
                } else {
                    DiffTarget::None
                }
            }
            Tab::Commits => {
                if let Some(commit) = self.commits.get(self.selected_commit) {
                    DiffTarget::Commit(commit.hash.clone())
                } else {
                    DiffTarget::None
                }
            }
            Tab::Worktrees => {
                if let Some(wt) = self.worktrees.get(self.selected_worktree) {
                    DiffTarget::Commit(wt.head_hash.clone())
                } else {
                    DiffTarget::None
                }
            }
            Tab::Stashes => {
                if let Some(st) = self.stashes.get(self.selected_stash) {
                    DiffTarget::Stash(st.index)
                } else {
                    DiffTarget::None
                }
            }
            Tab::TimeMachine => {
                if let Some(rf) = self.reflogs.get(self.selected_reflog) {
                    DiffTarget::Reflog(rf.selector.clone())
                } else {
                    DiffTarget::None
                }
            }
        };

        if target != self.current_target {
            self.diff_scroll = 0;
            self.current_target = target;
        }

        self.diff_content = git::get_diff_content(&self.cwd, &self.current_target)
            .unwrap_or_else(|e| format!("Error al cargar diff: {}", e));
    }

    /// Notifica un mensaje de éxito o advertencia en la barra de estado.
    pub fn set_toast(&mut self, msg: &str, is_error: bool) {
        self.status_toast = Some((msg.to_string(), is_error));
    }

    // ==========================================================================
    // Manejo de Navegación y Foco
    // ==========================================================================

    pub fn next_tab(&mut self) {
        let cur = self.active_tab.to_index();
        self.active_tab = Tab::from_index((cur + 1) % 6);
        self.update_current_diff();
    }

    pub fn prev_tab(&mut self) {
        let cur = self.active_tab.to_index();
        self.active_tab = Tab::from_index(if cur == 0 { 5 } else { cur - 1 });
        self.update_current_diff();
    }

    pub fn set_tab(&mut self, tab_idx: usize) {
        if tab_idx < 6 {
            self.active_tab = Tab::from_index(tab_idx);
            self.update_current_diff();
        }
    }

    pub fn move_down(&mut self) {
        match self.active_tab {
            Tab::Files => {
                if !self.files_in_staged_view {
                    if !self.unstaged_files.is_empty() && self.selected_unstaged + 1 < self.unstaged_files.len() {
                        self.selected_unstaged += 1;
                    } else if !self.staged_files.is_empty() {
                        self.files_in_staged_view = true;
                        self.selected_staged = 0;
                    }
                } else if !self.staged_files.is_empty() && self.selected_staged + 1 < self.staged_files.len() {
                    self.selected_staged += 1;
                }
            }
            Tab::Branches => {
                if !self.branches.is_empty() && self.selected_branch + 1 < self.branches.len() {
                    self.selected_branch += 1;
                }
            }
            Tab::Commits => {
                if !self.commits.is_empty() && self.selected_commit + 1 < self.commits.len() {
                    self.selected_commit += 1;
                }
            }
            Tab::Worktrees => {
                if !self.worktrees.is_empty() && self.selected_worktree + 1 < self.worktrees.len() {
                    self.selected_worktree += 1;
                }
            }
            Tab::Stashes => {
                if !self.stashes.is_empty() && self.selected_stash + 1 < self.stashes.len() {
                    self.selected_stash += 1;
                }
            }
            Tab::TimeMachine => {
                if !self.reflogs.is_empty() && self.selected_reflog + 1 < self.reflogs.len() {
                    self.selected_reflog += 1;
                }
            }
        }
        self.update_current_diff();
    }

    pub fn move_up(&mut self) {
        match self.active_tab {
            Tab::Files => {
                if self.files_in_staged_view {
                    if self.selected_staged > 0 {
                        self.selected_staged -= 1;
                    } else if !self.unstaged_files.is_empty() {
                        self.files_in_staged_view = false;
                        self.selected_unstaged = self.unstaged_files.len().saturating_sub(1);
                    }
                } else if self.selected_unstaged > 0 {
                    self.selected_unstaged -= 1;
                }
            }
            Tab::Branches => {
                if self.selected_branch > 0 {
                    self.selected_branch -= 1;
                }
            }
            Tab::Commits => {
                if self.selected_commit > 0 {
                    self.selected_commit -= 1;
                }
            }
            Tab::Worktrees => {
                if self.selected_worktree > 0 {
                    self.selected_worktree -= 1;
                }
            }
            Tab::Stashes => {
                if self.selected_stash > 0 {
                    self.selected_stash -= 1;
                }
            }
            Tab::TimeMachine => {
                if self.selected_reflog > 0 {
                    self.selected_reflog -= 1;
                }
            }
        }
        self.update_current_diff();
    }

    pub fn scroll_diff_down(&mut self, amount: usize) {
        let total_lines = self.diff_content.lines().count();
        if self.diff_scroll + amount < total_lines {
            self.diff_scroll += amount;
        }
    }

    pub fn scroll_diff_up(&mut self, amount: usize) {
        if self.diff_scroll >= amount {
            self.diff_scroll -= amount;
        } else {
            self.diff_scroll = 0;
        }
    }

    // ==========================================================================
    // Operaciones Directas de Git
    // ==========================================================================

    pub fn toggle_stage_current(&mut self) {
        if self.files_in_staged_view {
            if let Some(file) = self.staged_files.get(self.selected_staged) {
                let path = file.path.clone();
                match git::unstage_file(&self.cwd, &path) {
                    Ok(_) => self.set_toast(&format!("Despreparado: {}", path), false),
                    Err(e) => self.set_toast(&e, true),
                }
            }
        } else if let Some(file) = self.unstaged_files.get(self.selected_unstaged) {
            let path = file.path.clone();
            match git::stage_file(&self.cwd, &path) {
                Ok(_) => self.set_toast(&format!("Preparado: {}", path), false),
                Err(e) => self.set_toast(&e, true),
            }
        }
        self.refresh_all();
    }

    pub fn stage_all_files(&mut self) {
        match git::stage_all(&self.cwd) {
            Ok(_) => self.set_toast("Todos los cambios han sido preparados (staged).", false),
            Err(e) => self.set_toast(&e, true),
        }
        self.refresh_all();
    }

    pub fn unstage_all_files(&mut self) {
        match git::unstage_all(&self.cwd) {
            Ok(_) => self.set_toast("Se han despreparado todos los cambios.", false),
            Err(e) => self.set_toast(&e, true),
        }
        self.refresh_all();
    }

    pub fn git_pull(&mut self) {
        self.set_toast("Ejecutando git pull...", false);
        match git::pull(&self.cwd) {
            Ok(msg) => self.set_toast(&format!("Pull completado: {}", msg.trim()), false),
            Err(e) => self.set_toast(&format!("Fallo en git pull: {}", e), true),
        }
        self.refresh_all();
    }

    pub fn git_push(&mut self, force: bool) {
        self.set_toast("Publicando commits con git push...", false);
        match git::push(&self.cwd, force) {
            Ok(msg) => self.set_toast(&format!("Push exitoso: {}", msg.trim()), false),
            Err(e) => self.set_toast(&format!("Error en git push: {}", e), true),
        }
        self.refresh_all();
    }

    pub fn git_fetch(&mut self) {
        self.set_toast("Obteniendo actualizaciones remotas (fetch --all --prune)...", false);
        match git::fetch_all(&self.cwd) {
            Ok(_) => self.set_toast("Fetch remoto completado con éxito.", false),
            Err(e) => self.set_toast(&format!("Error al sincronizar remotos: {}", e), true),
        }
        self.refresh_all();
    }

    /// Intenta realizar un commit validando primero que no existan secretos expuestos.
    pub fn try_commit(&mut self, message: String) {
        if message.trim().is_empty() {
            self.set_toast("El mensaje del commit no puede estar vacío.", true);
            return;
        }

        // Obtener el diff completo preparado para inspección de seguridad
        let cached_diff = git::run_git_cmd(&self.cwd, &["diff", "--cached", "--color=never"])
            .unwrap_or_default();

        let findings = sentinel::scan_diff(&cached_diff);
        if !findings.is_empty() {
            // Activar Sentinel Shield Modal
            self.modal = ActiveModal::SentinelAlert {
                findings,
                pending_commit_msg: message,
            };
            return;
        }

        self.execute_commit(&message);
    }

    pub fn execute_commit(&mut self, message: &str) {
        match git::commit(&self.cwd, message) {
            Ok(_) => {
                self.modal = ActiveModal::None;
                self.set_toast("Commit confirmado exitosamente.", false);
                self.refresh_all();
            }
            Err(e) => {
                self.modal = ActiveModal::CommandLog {
                    title: "Error al realizar commit".to_string(),
                    content: e,
                    is_error: true,
                };
            }
        }
    }

    // ==========================================================================
    // Operaciones de Ramas y Worktrees
    // ==========================================================================

    pub fn checkout_selected_branch(&mut self) {
        if let Some(branch) = self.branches.get(self.selected_branch) {
            let name = branch.name.clone();
            match git::checkout_branch(&self.cwd, &name) {
                Ok(_) => {
                    self.set_toast(&format!("Cambiado a rama: {}", name), false);
                    self.refresh_all();
                }
                Err(e) => self.set_toast(&e, true),
            }
        }
    }

    pub fn prompt_prune_merged_branches(&mut self) {
        let main_branch = if self.branches.iter().any(|b| b.name == "main") {
            "main"
        } else {
            "master"
        };

        match git::find_merged_branches(&self.cwd, main_branch) {
            Ok(branches) => {
                if branches.is_empty() {
                    self.set_toast("No hay ramas huérfanas o ya integradas para purgar.", false);
                } else {
                    let count = branches.len();
                    self.modal = ActiveModal::Confirm {
                        title: "Zombie Branch Pruner: Purgar Ramas Integradas".to_string(),
                        message: format!(
                            "Se han detectado {} ramas locales ya fusionadas en '{}':\n{}\n\n¿Deseas eliminarlas de forma segura?",
                            count,
                            main_branch,
                            branches.join(", ")
                        ),
                        action: PendingAction::PruneMerged(branches),
                    };
                }
            }
            Err(e) => self.set_toast(&e, true),
        }
    }

    // ==========================================================================
    // Confirmación y Ejecución de Acciones Pendientes
    // ==========================================================================

    pub fn execute_pending_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::DiscardFile(path) => {
                match git::discard_changes(&self.cwd, &path) {
                    Ok(_) => self.set_toast(&format!("Cambios descartados en: {}", path), false),
                    Err(e) => self.set_toast(&e, true),
                }
            }
            PendingAction::DeleteBranch(name, force) => {
                match git::delete_branch(&self.cwd, &name, force) {
                    Ok(_) => self.set_toast(&format!("Rama eliminada: {}", name), false),
                    Err(e) => self.set_toast(&e, true),
                }
            }
            PendingAction::PruneMerged(branches) => {
                let mut deleted = 0;
                for b in &branches {
                    if git::delete_branch(&self.cwd, b, false).is_ok() {
                        deleted += 1;
                    }
                }
                self.set_toast(&format!("Se han purgado {} ramas integradas.", deleted), false);
            }
            PendingAction::RestoreReflog(selector) => {
                match git::restore_reflog_point(&self.cwd, &selector) {
                    Ok(_) => self.set_toast(&format!("Repositorio restaurado con éxito al estado {}", selector), false),
                    Err(e) => self.set_toast(&format!("Error al restaurar reflog: {}", e), true),
                }
            }
            PendingAction::ResetCommit(hash, mode) => {
                match git::reset_to_commit(&self.cwd, &hash, mode) {
                    Ok(_) => self.set_toast(&format!("Reset ({}) aplicado a {}", mode, hash), false),
                    Err(e) => self.set_toast(&e, true),
                }
            }
            PendingAction::ForcePush => {
                self.git_push(true);
            }
            PendingAction::RemoveWorktree(path) => {
                match git::remove_worktree(&self.cwd, &path, false) {
                    Ok(_) => self.set_toast(&format!("Worktree eliminado: {}", path), false),
                    Err(e) => self.set_toast(&e, true),
                }
            }
        }
        self.modal = ActiveModal::None;
        self.refresh_all();
    }
}
