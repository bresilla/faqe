use std::collections::BTreeMap;

use faqe_model::ElementKind;

pub fn classify_element(tag: &str, attributes: &BTreeMap<String, String>) -> ElementKind {
    let has_class = |name: &str| {
        attributes
            .get("class")
            .is_some_and(|classes| classes.split_whitespace().any(|class| class == name))
    };
    if has_class("btn") {
        ElementKind::Button
    } else if has_class("command") || has_class("commandframeholder") {
        ElementKind::Command
    } else if has_class("textframeholder") || has_class("tipframeholder") {
        ElementKind::Callout
    } else if has_class("hideframeholder") || tag == "details" || tag == "summary" {
        ElementKind::Disclosure
    } else if has_class("sidenote") || has_class("sidenote-number") {
        ElementKind::SideNote
    } else if has_class("marginnote") {
        ElementKind::SideImage
    } else if has_class("progress_hr") {
        ElementKind::ReadingBreak
    } else if primitive_is(attributes, "graph") {
        ElementKind::Graph
    } else if primitive_is(attributes, "timeline") {
        ElementKind::Timeline
    } else if primitive_is(attributes, "progress") {
        ElementKind::Progress
    } else if primitive_matches(attributes, &["grid", "table"]) {
        ElementKind::DataGrid
    } else if primitive_matches(attributes, &["hero", "prose"]) {
        ElementKind::Feature
    } else if primitive_is(attributes, "code-block") {
        ElementKind::CodeBlock
    } else if attributes.contains_key("data-shortcode-slide")
        || attributes.contains_key("data-shortcode-section")
    {
        ElementKind::Slide
    } else {
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => ElementKind::Heading,
            "p" => ElementKind::Paragraph,
            "ol" | "ul" | "li" | "dl" | "dt" | "dd" => ElementKind::List,
            "blockquote" | "q" | "aside" => ElementKind::Quote,
            "pre" | "code" => ElementKind::CodeBlock,
            "table" | "caption" | "colgroup" | "col" | "thead" | "tbody" | "tfoot" | "tr"
            | "th" | "td" => ElementKind::Table,
            "img" | "figure" | "figcaption" => ElementKind::Image,
            _ => ElementKind::AllowedHtml,
        }
    }
}

fn primitive_is(attributes: &BTreeMap<String, String>, name: &str) -> bool {
    attributes
        .get("data-faqe-primitive")
        .is_some_and(|value| value == name)
}

fn primitive_matches(attributes: &BTreeMap<String, String>, names: &[&str]) -> bool {
    attributes
        .get("data-faqe-primitive")
        .is_some_and(|value| names.contains(&value.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_markdown_elements_are_primitives() {
        assert_eq!(
            classify_element("code", &BTreeMap::new()),
            ElementKind::CodeBlock
        );
        assert_eq!(
            classify_element("blockquote", &BTreeMap::new()),
            ElementKind::Quote
        );
        assert_eq!(
            classify_element("table", &BTreeMap::new()),
            ElementKind::Table
        );
    }
}
