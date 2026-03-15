//! Language registry — maps file extensions to tree-sitter grammars and queries.
//!
//! Currently an empty registry (no grammars compiled in). Plan 02 will replace
//! this with a static registry of compiled-in Rust, TypeScript, and JavaScript
//! grammars using tree-sitter-rust, tree-sitter-typescript, and
//! tree-sitter-javascript crates.

use std::collections::HashMap;
use tree_sitter::{Language, Query};

/// Configuration for a language.
pub struct PluginLangConfig {
    /// Language name (owned)
    pub name: String,
    /// Compiled tree-sitter grammar
    pub grammar: Language,
    /// Compiled tree-sitter query for structural extraction
    pub query: Query,
    /// File extensions (owned)
    pub extensions: Vec<String>,
}

/// Central registry mapping language names and file extensions to configs.
pub struct LangRegistry {
    by_name: HashMap<String, usize>,
    by_ext: HashMap<String, usize>,
    configs: Vec<PluginLangConfig>,
}

/// Global singleton — currently empty; Plan 02 will populate with compiled-in grammars.
static REGISTRY: std::sync::LazyLock<LangRegistry> =
    std::sync::LazyLock::new(LangRegistry::init);

impl LangRegistry {
    fn init() -> Self {
        LangRegistry {
            by_name: HashMap::new(),
            by_ext: HashMap::new(),
            configs: Vec::new(),
        }
    }

    /// Look up by language name.
    pub fn get(&self, name: &str) -> Option<&PluginLangConfig> {
        self.by_name.get(name).map(|&idx| &self.configs[idx])
    }

    /// Look up by file extension (without dot).
    pub fn get_by_ext(&self, ext: &str) -> Option<&PluginLangConfig> {
        self.by_ext.get(ext).map(|&idx| &self.configs[idx])
    }

    /// All registered file extensions.
    pub fn all_extensions(&self) -> Vec<&str> {
        self.by_ext.keys().map(|s| s.as_str()).collect()
    }

    /// Number of loaded languages.
    pub fn count(&self) -> usize {
        self.configs.len()
    }

    /// Failed plugin descriptions (empty — no plugin system).
    pub fn failed(&self) -> &[String] {
        &[]
    }
}

// ── Public free functions delegating to global singleton ──

/// Get language config by name.
pub fn get(name: &str) -> Option<&'static PluginLangConfig> {
    REGISTRY.get(name)
}

/// Get grammar + query for a language name.
pub fn get_grammar_and_query(name: &str) -> Option<(&'static Language, &'static Query)> {
    REGISTRY.get(name).map(|c| (&c.grammar, &c.query))
}

/// All registered extensions.
pub fn all_extensions() -> Vec<&'static str> {
    REGISTRY.all_extensions()
}

/// Number of loaded language configs.
pub fn plugin_count() -> usize {
    REGISTRY.count()
}

/// Detect language name from file extension string.
pub fn detect_lang_from_ext(ext: &str) -> String {
    if let Some(config) = REGISTRY.get_by_ext(ext) {
        return config.name.clone();
    }
    // Fallback: languages we recognize for display but don't parse structurally
    match ext {
        "json" => "json".into(),
        "toml" => "toml".into(),
        "yaml" | "yml" => "yaml".into(),
        "md" => "markdown".into(),
        "sql" => "sql".into(),
        "dart" => "dart".into(),
        "xml" => "xml".into(),
        "vue" => "vue".into(),
        "svelte" => "svelte".into(),
        "pl" | "pm" => "perl".into(),
        "sass" => "sass".into(),
        "gd" => "gdscript".into(),
        _ => "unknown".into(),
    }
}

/// Detect language from the full filename (not just extension).
pub fn detect_lang_from_filename(filename: &str) -> Option<&'static str> {
    let base = filename.rsplit('/').next().unwrap_or(filename);
    match base {
        "Dockerfile" => Some("dockerfile"),
        "Makefile" | "GNUmakefile" => Some("bash"),
        "Rakefile" | "Gemfile" | "Guardfile" | "Vagrantfile" => Some("ruby"),
        "Justfile" => Some("bash"),
        _ if base.starts_with("Dockerfile.") => Some("dockerfile"),
        _ if base.starts_with("Makefile.") => Some("bash"),
        _ => None,
    }
}

/// Failed plugin descriptions.
pub fn failed_plugins() -> &'static [String] {
    REGISTRY.failed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_lang_from_ext_fallbacks() {
        assert_eq!(detect_lang_from_ext("json"), "json");
        assert_eq!(detect_lang_from_ext("toml"), "toml");
        assert_eq!(detect_lang_from_ext("xyz"), "unknown");
    }

    #[test]
    fn test_detect_lang_from_filename() {
        assert_eq!(detect_lang_from_filename("Dockerfile"), Some("dockerfile"));
        assert_eq!(detect_lang_from_filename("Makefile"), Some("bash"));
        assert_eq!(detect_lang_from_filename("random.txt"), None);
    }

    #[test]
    fn test_registry_loads() {
        let _ = &*REGISTRY;
    }

    // --- New tests for static grammar registry (Plan 02) ---

    #[test]
    fn test_get_grammar_and_query_rust() {
        assert!(get_grammar_and_query("rust").is_some(), "rust grammar must be present");
    }

    #[test]
    fn test_get_grammar_and_query_typescript() {
        assert!(get_grammar_and_query("typescript").is_some(), "typescript grammar must be present");
    }

    #[test]
    fn test_get_grammar_and_query_javascript() {
        assert!(get_grammar_and_query("javascript").is_some(), "javascript grammar must be present");
    }

    #[test]
    fn test_get_grammar_and_query_python_none() {
        assert!(get_grammar_and_query("python").is_none(), "python must not be registered");
    }

    #[test]
    fn test_get_grammar_and_query_go_none() {
        assert!(get_grammar_and_query("go").is_none(), "go must not be registered");
    }

    #[test]
    fn test_detect_lang_from_ext_rs() {
        assert_eq!(detect_lang_from_ext("rs"), "rust");
    }

    #[test]
    fn test_detect_lang_from_ext_ts() {
        assert_eq!(detect_lang_from_ext("ts"), "typescript");
    }

    #[test]
    fn test_detect_lang_from_ext_tsx() {
        assert_eq!(detect_lang_from_ext("tsx"), "typescript");
    }

    #[test]
    fn test_detect_lang_from_ext_js() {
        assert_eq!(detect_lang_from_ext("js"), "javascript");
    }

    #[test]
    fn test_detect_lang_from_ext_jsx() {
        assert_eq!(detect_lang_from_ext("jsx"), "javascript");
    }

    #[test]
    fn test_detect_lang_from_ext_mjs() {
        assert_eq!(detect_lang_from_ext("mjs"), "javascript");
    }

    #[test]
    fn test_detect_lang_from_ext_cjs() {
        assert_eq!(detect_lang_from_ext("cjs"), "javascript");
    }

    #[test]
    fn test_detect_lang_from_ext_py_unknown() {
        assert_eq!(detect_lang_from_ext("py"), "unknown");
    }

    #[test]
    fn test_detect_lang_from_ext_go_unknown() {
        assert_eq!(detect_lang_from_ext("go"), "unknown");
    }
}
