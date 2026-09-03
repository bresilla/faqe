use std::cell::RefCell;
use std::sync::OnceLock;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

const TREE_SITTER_HIGHLIGHTS: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "escape",
    "function",
    "function.builtin",
    "keyword",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.special",
    "string.special.key",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

thread_local! {
    static TREE_SITTER_HIGHLIGHTER: RefCell<Highlighter> = RefCell::new(Highlighter::new());
}

#[derive(Clone, Copy)]
enum HighlightLanguage {
    Bash,
    Json,
    Toml,
}

pub fn render(language: &str, source: &str) -> String {
    let syntax = highlight_language(language);
    let highlighted = syntax
        .and_then(|syntax| tree_sitter_highlight(syntax, source))
        .unwrap_or_else(|| escape_html(source));
    let language = escape_html(language);
    let label = if language.is_empty() {
        "TEXT"
    } else {
        language.as_str()
    };
    let engine = if syntax.is_some() {
        "TREE-SITTER"
    } else {
        "PLAIN"
    };
    format!(
        "<div class=\"highlight ts-highlight\" data-faqe-primitive=\"code-block\" data-highlighter=\"{engine}\"><div class=\"code-toolbar\" aria-hidden=\"true\"><span class=\"code-language\">{label}</span><span class=\"code-engine\">{engine}</span></div><pre tabindex=\"0\" class=\"chroma\"><code class=\"language-{language}\" data-lang=\"{language}\">{highlighted}</code></pre></div>"
    )
}

fn highlight_language(language: &str) -> Option<HighlightLanguage> {
    match language.trim().to_ascii_lowercase().as_str() {
        "bash" | "console" | "sh" | "shell" | "zsh" => Some(HighlightLanguage::Bash),
        "json" => Some(HighlightLanguage::Json),
        "toml" => Some(HighlightLanguage::Toml),
        _ => None,
    }
}

fn bash_highlight_configuration() -> &'static HighlightConfiguration {
    static CONFIGURATION: OnceLock<HighlightConfiguration> = OnceLock::new();
    CONFIGURATION.get_or_init(|| {
        let mut configuration = HighlightConfiguration::new(
            tree_sitter_bash::LANGUAGE.into(),
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        )
        .expect("bundled Bash highlight query must compile");
        configuration.configure(TREE_SITTER_HIGHLIGHTS);
        configuration
    })
}

fn json_highlight_configuration() -> &'static HighlightConfiguration {
    static CONFIGURATION: OnceLock<HighlightConfiguration> = OnceLock::new();
    CONFIGURATION.get_or_init(|| {
        let mut configuration = HighlightConfiguration::new(
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .expect("bundled JSON highlight query must compile");
        configuration.configure(TREE_SITTER_HIGHLIGHTS);
        configuration
    })
}

fn toml_highlight_configuration() -> &'static HighlightConfiguration {
    static CONFIGURATION: OnceLock<HighlightConfiguration> = OnceLock::new();
    CONFIGURATION.get_or_init(|| {
        let mut configuration = HighlightConfiguration::new(
            tree_sitter_toml_ng::LANGUAGE.into(),
            "toml",
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .expect("bundled TOML highlight query must compile");
        configuration.configure(TREE_SITTER_HIGHLIGHTS);
        configuration
    })
}

fn highlight_configuration(language: HighlightLanguage) -> &'static HighlightConfiguration {
    match language {
        HighlightLanguage::Bash => bash_highlight_configuration(),
        HighlightLanguage::Json => json_highlight_configuration(),
        HighlightLanguage::Toml => toml_highlight_configuration(),
    }
}

fn tree_sitter_highlight(language: HighlightLanguage, source: &str) -> Option<String> {
    TREE_SITTER_HIGHLIGHTER.with(|highlighter| {
        let mut highlighter = highlighter.try_borrow_mut().ok()?;
        let events = highlighter
            .highlight(
                highlight_configuration(language),
                source.as_bytes(),
                None,
                |_| None,
            )
            .ok()?;
        let mut highlighted = String::with_capacity(source.len() + source.len() / 2);
        for event in events {
            match event.ok()? {
                HighlightEvent::Source { start, end } => {
                    highlighted.push_str(&escape_html(source.get(start..end)?));
                }
                HighlightEvent::HighlightStart(highlight) => {
                    let class = TREE_SITTER_HIGHLIGHTS.get(highlight.0)?.replace('.', "-");
                    highlighted.push_str("<span class=\"ts-");
                    highlighted.push_str(&class);
                    highlighted.push_str("\">");
                }
                HighlightEvent::HighlightEnd => highlighted.push_str("</span>"),
            }
        }
        Some(highlighted)
    })
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_blocks_identify_themselves_as_primitives() {
        let html = render("rust", "fn main() {}\n");
        assert!(html.contains("data-faqe-primitive=\"code-block\""));
        assert!(html.contains("data-highlighter=\"PLAIN\""));
    }
}
