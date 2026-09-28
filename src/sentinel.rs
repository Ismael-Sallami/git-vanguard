// ==============================================================================
// GitVanguard - Sentinel Shield: Escudo de Prevención de Fugas de Secretos
// Analizador estático de alta velocidad para la inspección de diffs y archivos
// preparados (staged) antes de confirmar o publicar commits.
// ==============================================================================

use regex::Regex;
use std::sync::OnceLock;

/// Representa un hallazgo de seguridad detectado en los cambios preparados.
#[derive(Debug, Clone)]
pub struct SecretFinding {
    pub file: String,
    pub line_number: usize,
    pub rule_name: &'static str,
    pub description: &'static str,
    pub matched_excerpt: String,
}

struct Rule {
    name: &'static str,
    description: &'static str,
    pattern: &'static str,
}

static RULES: &[Rule] = &[
    Rule {
        name: "Clave Privada RSA / SSH",
        description: "Cabecera de certificado o clave criptográfica privada expuesta.",
        pattern: r"-----BEGIN (?:RSA|EC|DSA|OPENSSH|PGP)? ?PRIVATE KEY-----",
    },
    Rule {
        name: "Token de Acceso de GitHub",
        description: "Token clásico o personal access token con permisos de repositorio.",
        pattern: r"(?:ghp_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})",
    },
    Rule {
        name: "API Key de OpenAI",
        description: "Clave de API de OpenAI o token de proyecto de plataforma.",
        pattern: r"(?:sk-[a-zA-Z0-9]{20,}|sk-proj-[a-zA-Z0-9_\-]{20,})",
    },
    Rule {
        name: "AWS Access Key ID",
        description: "Identificador de clave de acceso a infraestructura de Amazon Web Services.",
        pattern: r"(?:A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16}",
    },
    Rule {
        name: "Token de Slack",
        description: "Token de bot o usuario con privilegios de API en Slack.",
        pattern: r"xox[baprs]-[0-9a-zA-Z]{10,48}",
    },
    Rule {
        name: "Asignación de Secreto o Password",
        description: "Asignación explícita de secreto, password o token de autenticación.",
        pattern: r#"(?i)(?:api_key|api_secret|app_secret|auth_token|client_secret|db_password|password|secret_key)\s*[:=]\s*['"][a-zA-Z0-9_\-!@#$%^&*()+=]{8,}['"]"#,
    },
];

static COMPILED_RULES: OnceLock<Vec<(&'static Rule, Regex)>> = OnceLock::new();

fn get_compiled_rules() -> &'static Vec<(&'static Rule, Regex)> {
    COMPILED_RULES.get_or_init(|| {
        RULES
            .iter()
            .filter_map(|r| {
                Regex::new(r.pattern).ok().map(|re| (r, re))
            })
            .collect()
    })
}

/// Escanea los nombres de archivo para detectar extensiones o patrones sensibles críticos.
pub fn is_sensitive_filename(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower == ".env" || lower.ends_with("/.env") || lower.contains(".env.") && !lower.ends_with(".example") {
        return Some("Fichero de variables de entorno (.env)");
    }
    if lower.ends_with("id_rsa") || lower.ends_with("id_ed25519") || lower.ends_with(".pem") || lower.ends_with(".key") {
        return Some("Fichero de clave criptográfica privada");
    }
    if lower.ends_with("credentials.json") || lower.ends_with("service-account.json") {
        return Some("Fichero de credenciales o cuentas de servicio");
    }
    None
}

/// Analiza el contenido de un diff textual preparado (staged diff).
pub fn scan_diff(diff_content: &str) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    let compiled = get_compiled_rules();

    let mut current_file = String::from("desconocido");
    let mut current_line = 0;

    for line in diff_content.lines() {
        // Detectar cabeceras de archivo en formato git diff: "+++ b/ruta/al/fichero"
        if let Some(target) = line.strip_prefix("+++ b/") {
            current_file = target.trim().to_string();
            current_line = 0;

            if let Some(desc) = is_sensitive_filename(&current_file) {
                findings.push(SecretFinding {
                    file: current_file.clone(),
                    line_number: 1,
                    rule_name: "Fichero Crítico Protegido",
                    description: desc,
                    matched_excerpt: format!("Fichero sensible detectado para confirmación: {}", current_file),
                });
            }
            continue;
        }

        // Detectar chunk headers: "@@ -1,5 +1,10 @@"
        if line.starts_with("@@") {
            if let Some(pos) = line.find('+') {
                let rest = &line[pos + 1..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                current_line = num_str.parse().unwrap_or(1);
            }
            continue;
        }

        // Solo analizamos las líneas añadidas al commit (comienzan por '+', excepto '+++')
        if line.starts_with('+') && !line.starts_with("+++") {
            let content = &line[1..];
            current_line += 1;

            // Ignorar líneas de ejemplo o comentarios inocuos
            let lower = content.to_lowercase();
            if lower.contains("example") || lower.contains("dummy") || lower.contains("placeholder") || lower.contains("test") {
                continue;
            }

            for (rule, re) in compiled {
                if let Some(mat) = re.find(content) {
                    let matched_str = mat.as_str();
                    // Ocultar caracteres intermedios por seguridad visual
                    let masked = if matched_str.len() > 10 {
                        format!("{}...{}", &matched_str[..4], &matched_str[matched_str.len() - 4..])
                    } else {
                        "********".to_string()
                    };

                    findings.push(SecretFinding {
                        file: current_file.clone(),
                        line_number: current_line,
                        rule_name: rule.name,
                        description: rule.description,
                        matched_excerpt: format!("{}: {}", masked, content.trim()),
                    });
                    break;
                }
            }
        } else if !line.starts_with('-') {
            current_line += 1;
        }
    }

    findings
}
