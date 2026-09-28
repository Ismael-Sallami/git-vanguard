#[cfg(test)]
mod tests {
    use crate::git;

    #[test]
    fn test_is_git_repository() {
        let cwd = std::env::current_dir().unwrap().to_string_lossy().to_string();
        assert!(git::is_git_repository(&cwd), "El directorio actual debe ser reconocido como repositorio Git");
        assert!(!git::is_git_repository("/tmp"), "/tmp no debería ser un repositorio Git a menos que se inicialice explícitamente");
    }

    #[test]
    fn test_repo_overview() {
        let cwd = std::env::current_dir().unwrap().to_string_lossy().to_string();
        let overview = git::get_repo_overview(&cwd);
        assert!(!overview.branch.is_empty(), "La rama actual no debe ser vacía");
    }

    #[test]
    fn test_status_files() {
        let cwd = std::env::current_dir().unwrap().to_string_lossy().to_string();
        let res = git::get_status_files(&cwd);
        assert!(res.is_ok(), "El comando status porcelain v2 debe ejecutarse sin errores");
    }
}
