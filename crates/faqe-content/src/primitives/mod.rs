mod code_block;
mod declarative;
mod kind;

use std::collections::BTreeMap;

use faqe_model::ElementKind;

pub use declarative::PrimitiveError;

pub fn render_fenced(language: &str, source: &str) -> Result<String, PrimitiveError> {
    Ok(declarative::render(language, source)?
        .unwrap_or_else(|| code_block::render(language, source)))
}

pub fn render_code_block(language: &str, source: &str) -> String {
    code_block::render(language, source)
}

pub fn classify_element(tag: &str, attributes: &BTreeMap<String, String>) -> ElementKind {
    kind::classify_element(tag, attributes)
}
