#[cfg(test)]
mod tests {
    #[test]
    fn test_secret_scanner_openai_key() {
        let diff = r#"
diff --git a/config.py b/config.py
--- a/config.py
+++ b/config.py
@@ -1,3 +1,4 @@
 import os
+OPENAI_API_KEY = "sk-proj-abc123456789012345678901234567890"
 def test_fn():
     pass
"#;
        let findings = crate::sentinel::scan_diff(diff);
        assert!(!findings.is_empty(), "Debería haber detectado la API key de OpenAI");
        assert_eq!(findings[0].rule_name, "API Key de OpenAI");
    }

    #[test]
    fn test_secret_scanner_aws_key() {
        let diff = r#"
diff --git a/deploy.sh b/deploy.sh
--- a/deploy.sh
+++ b/deploy.sh
@@ -1,2 +1,3 @@
 #!/bin/bash
+export AWS_ACCESS_KEY_ID="AKIA9876543210ZYXWVU"
"#;
        let findings = crate::sentinel::scan_diff(diff);
        assert!(!findings.is_empty(), "Debería haber detectado la AWS Access Key");
        assert_eq!(findings[0].rule_name, "AWS Access Key ID");
    }

    #[test]
    fn test_secret_scanner_github_token() {
        let diff = r#"
diff --git a/sync.js b/sync.js
--- a/sync.js
+++ b/sync.js
@@ -1,2 +1,3 @@
+const token = "ghp_123456789012345678901234567890123456";
"#;
        let findings = crate::sentinel::scan_diff(diff);
        assert!(!findings.is_empty(), "Debería haber detectado el token de GitHub");
        assert_eq!(findings[0].rule_name, "Token de Acceso de GitHub");
    }

    #[test]
    fn test_sensitive_file_detection() {
        assert!(crate::sentinel::is_sensitive_filename(".env").is_some());
        assert!(crate::sentinel::is_sensitive_filename("backend/.env.production").is_some());
        assert!(crate::sentinel::is_sensitive_filename("id_rsa").is_some());
        assert!(crate::sentinel::is_sensitive_filename(".env.example").is_none());
    }
}
