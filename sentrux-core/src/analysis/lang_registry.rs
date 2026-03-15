//! Language registry — maps file extensions to tree-sitter grammars and queries.
//!
//! Static registry with compiled-in Rust, TypeScript, and JavaScript grammars.
//! All other file types are silently skipped (get_grammar_and_query returns None).

use std::collections::HashMap;
use std::sync::LazyLock;
use tree_sitter::{Language, Query};

/// Sentinel value returned by `detect_lang_from_ext` for unrecognized extensions.
pub const LANG_UNKNOWN: &str = "unknown";

/// Configuration for a compiled-in language.
pub struct LangConfig {
    /// Language name
    pub name: &'static str,
    /// Compiled tree-sitter grammar
    pub grammar: Language,
    /// Compiled tree-sitter query for structural extraction
    pub query: Query,
    /// File extensions (without dot)
    pub extensions: &'static [&'static str],
}

/// Static 3-language registry with compiled-in grammars.
pub struct LangRegistry {
    configs: Vec<LangConfig>,
    ext_map: HashMap<&'static str, usize>,
}

macro_rules! register_lang {
    ($configs:expr, $ext_map:expr, $grammar:expr, $query_src:expr, $name:expr, $exts:expr) => {{
        let grammar = Language::new($grammar);
        let query = Query::new(&grammar, $query_src)
            .expect(concat!("Failed to compile ", $name, " tags query"));
        let idx = $configs.len();
        $configs.push(LangConfig {
            name: $name,
            grammar,
            query,
            extensions: $exts,
        });
        for ext in $exts {
            $ext_map.insert(*ext, idx);
        }
    }};
}

impl LangRegistry {
    fn init() -> Self {
        let mut configs = Vec::new();
        let mut ext_map = HashMap::new();

        register_lang!(configs, ext_map,
            tree_sitter_rust::LANGUAGE,
            include_str!("../queries/rust/tags.scm"),
            "rust", &["rs"]);

        register_lang!(configs, ext_map,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            include_str!("../queries/typescript/tags.scm"),
            "typescript", &["ts", "tsx"]);

        register_lang!(configs, ext_map,
            tree_sitter_javascript::LANGUAGE,
            include_str!("../queries/javascript/tags.scm"),
            "javascript", &["js", "jsx", "mjs", "cjs"]);

        LangRegistry { configs, ext_map }
    }

    /// Look up by file extension (without dot).
    pub fn get_by_ext(&self, ext: &str) -> Option<&LangConfig> {
        self.ext_map.get(ext).map(|&idx| &self.configs[idx])
    }

    /// Look up by language name.
    fn get_by_name(&self, name: &str) -> Option<&LangConfig> {
        self.configs.iter().find(|c| c.name == name)
    }

    /// All registered file extensions.
    pub fn all_extensions(&self) -> Vec<&str> {
        self.ext_map.keys().copied().collect()
    }

    /// Number of loaded languages.
    pub fn count(&self) -> usize {
        self.configs.len()
    }
}

// ── Global singleton ──

static REGISTRY: LazyLock<LangRegistry> = LazyLock::new(LangRegistry::init);

// ── Public free functions delegating to global singleton ──

/// Get grammar + query for a language name.
pub fn get_grammar_and_query(name: &str) -> Option<(&'static Language, &'static Query)> {
    REGISTRY.get_by_name(name).map(|c| (&c.grammar, &c.query))
}

/// All registered extensions.
pub fn all_extensions() -> Vec<&'static str> {
    REGISTRY.all_extensions()
}

/// Number of loaded language configs.
pub fn lang_count() -> usize {
    REGISTRY.count()
}

/// Detect language name from file extension string.
///
/// Returns the language name for parseable types (rust, typescript, javascript),
/// or a display-only name for known-but-unparseable types (json, toml, etc.),
/// or `LANG_UNKNOWN` for unrecognized extensions.
pub fn detect_lang_from_ext(ext: &str) -> &'static str {
    if let Some(config) = REGISTRY.get_by_ext(ext) {
        return config.name;
    }
    // Fallback: languages we recognize for display but don't parse structurally
    match ext {
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "sql" => "sql",
        "dart" => "dart",
        "xml" => "xml",
        "vue" => "vue",
        "svelte" => "svelte",
        "pl" | "pm" => "perl",
        "sass" => "sass",
        "gd" => "gdscript",
        _ => LANG_UNKNOWN,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_lang_from_ext_fallbacks() {
        assert_eq!(detect_lang_from_ext("json"), "json");
        assert_eq!(detect_lang_from_ext("toml"), "toml");
        assert_eq!(detect_lang_from_ext("xyz"), LANG_UNKNOWN);
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
        assert_eq!(detect_lang_from_ext("py"), LANG_UNKNOWN);
    }

    #[test]
    fn test_detect_lang_from_ext_go_unknown() {
        assert_eq!(detect_lang_from_ext("go"), LANG_UNKNOWN);
    }
}
