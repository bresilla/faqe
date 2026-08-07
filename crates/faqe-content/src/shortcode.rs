use std::collections::BTreeMap;
use std::fmt;

const MAX_SHORTCODE_DEPTH: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct ShortcodeParser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortcodeError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ShortcodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "shortcode error at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for ShortcodeError {}

#[derive(Clone, Debug)]
enum Node {
    Text(String),
    Shortcode {
        name: String,
        args: Arguments,
        children: Vec<Node>,
    },
}

#[derive(Clone, Debug, Default)]
struct Arguments {
    positional: Vec<String>,
    named: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct Frame {
    name: String,
    args: Arguments,
    children: Vec<Node>,
    offset: usize,
}

impl ShortcodeParser {
    pub fn render(&self, source: &str) -> Result<String, ShortcodeError> {
        let nodes = parse(source)?;
        let mut output = String::with_capacity(source.len());
        let mut state = RenderState::default();
        for node in &nodes {
            render_node(node, &mut output, &mut state)?;
        }
        Ok(output)
    }
}

#[derive(Default)]
struct RenderState {
    disclosure: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Markup {
    Raw(String),
    Text(String),
    Element(Element),
    Sequence(Vec<Markup>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Element {
    tag: &'static str,
    attributes: Vec<Attribute>,
    children: Vec<Markup>,
    void: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Attribute {
    Value(String, String),
    Boolean(String),
}

impl Markup {
    fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    fn write_to(&self, output: &mut String) {
        match self {
            Self::Raw(value) => output.push_str(value),
            Self::Text(value) => output.push_str(&escape_html(value)),
            Self::Element(element) => element.write_to(output),
            Self::Sequence(children) => {
                for child in children {
                    child.write_to(output);
                }
            }
        }
    }
}

impl From<Element> for Markup {
    fn from(element: Element) -> Self {
        Self::Element(element)
    }
}

impl Element {
    fn new(tag: &'static str) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            children: Vec::new(),
            void: false,
        }
    }

    fn void(tag: &'static str) -> Self {
        Self {
            void: true,
            ..Self::new(tag)
        }
    }

    fn attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes
            .push(Attribute::Value(name.into(), value.into()));
        self
    }

    fn boolean_attr(mut self, name: impl Into<String>) -> Self {
        self.attributes.push(Attribute::Boolean(name.into()));
        self
    }

    fn child(mut self, child: impl Into<Markup>) -> Self {
        self.children.push(child.into());
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = Markup>) -> Self {
        self.children.extend(children);
        self
    }

    fn write_to(&self, output: &mut String) {
        output.push('<');
        output.push_str(self.tag);
        for attribute in &self.attributes {
            match attribute {
                Attribute::Value(name, value) => {
                    output.push(' ');
                    output.push_str(name);
                    output.push_str("=\"");
                    output.push_str(&escape_attribute(value));
                    output.push('"');
                }
                Attribute::Boolean(name) => {
                    output.push(' ');
                    output.push_str(name);
                }
            }
        }
        output.push('>');
        if self.void {
            return;
        }
        for child in &self.children {
            child.write_to(output);
        }
        output.push_str("</");
        output.push_str(self.tag);
        output.push('>');
    }
}

fn parse(source: &str) -> Result<Vec<Node>, ShortcodeError> {
    let mut root = Vec::new();
    let mut stack = Vec::<Frame>::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("{{") {
        let start = cursor + relative;
        let prefix = &source[start..];
        let (closing_delimiter, delimiter_len) = if prefix.starts_with("{{<") {
            (">}}", 3)
        } else if prefix.starts_with("{{%") {
            ("%}}", 3)
        } else {
            cursor = start + 2;
            continue;
        };
        let Some(relative_end) = source[start + delimiter_len..].find(closing_delimiter) else {
            return Err(ShortcodeError {
                offset: start,
                message: "unterminated shortcode".into(),
            });
        };
        let end = start + delimiter_len + relative_end;
        push_text(&mut root, &mut stack, &source[cursor..start]);
        let expression = source[start + delimiter_len..end].trim();
        let token_end = end + closing_delimiter.len();

        if let Some(name) = expression.strip_prefix('/') {
            let name = name.trim();
            let Some(frame) = stack.pop() else {
                return Err(ShortcodeError {
                    offset: start,
                    message: format!("unexpected closing shortcode {name:?}"),
                });
            };
            if frame.name != name {
                return Err(ShortcodeError {
                    offset: start,
                    message: format!(
                        "closing shortcode {name:?} does not match open shortcode {:?} at byte {}",
                        frame.name, frame.offset
                    ),
                });
            }
            push_node(
                &mut root,
                &mut stack,
                Node::Shortcode {
                    name: frame.name,
                    args: frame.args,
                    children: frame.children,
                },
            );
        } else {
            let (name, args) = parse_expression(expression, start)?;
            if is_self_closing(&name) {
                push_node(
                    &mut root,
                    &mut stack,
                    Node::Shortcode {
                        name,
                        args,
                        children: Vec::new(),
                    },
                );
            } else {
                if stack.len() >= MAX_SHORTCODE_DEPTH {
                    return Err(ShortcodeError {
                        offset: start,
                        message: format!(
                            "shortcode nesting exceeds the maximum depth of {MAX_SHORTCODE_DEPTH}"
                        ),
                    });
                }
                stack.push(Frame {
                    name,
                    args,
                    children: Vec::new(),
                    offset: start,
                });
            }
        }
        cursor = token_end;
    }
    push_text(&mut root, &mut stack, &source[cursor..]);
    if let Some(frame) = stack.last() {
        return Err(ShortcodeError {
            offset: frame.offset,
            message: format!("shortcode {:?} is missing its closing tag", frame.name),
        });
    }
    Ok(root)
}

fn parse_expression(
    expression: &str,
    offset: usize,
) -> Result<(String, Arguments), ShortcodeError> {
    let words = shell_words(expression, offset)?;
    let Some(name) = words.first() else {
        return Err(ShortcodeError {
            offset,
            message: "empty shortcode".into(),
        });
    };
    let mut args = Arguments::default();
    for word in words.iter().skip(1) {
        if let Some((key, value)) = word.split_once('=') {
            args.named.insert(key.to_owned(), value.to_owned());
        } else {
            args.positional.push(word.to_owned());
        }
    }
    Ok((name.to_owned(), args))
}

fn shell_words(expression: &str, offset: usize) -> Result<Vec<String>, ShortcodeError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in expression.chars() {
        if escaped {
            word.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
            } else {
                word.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if quote.is_some() {
        return Err(ShortcodeError {
            offset,
            message: "unterminated quoted argument".into(),
        });
    }
    if escaped {
        word.push('\\');
    }
    if !word.is_empty() {
        words.push(word);
    }
    Ok(words)
}

fn is_self_closing(name: &str) -> bool {
    matches!(name, "button" | "hr" | "slide")
}

fn push_text(root: &mut Vec<Node>, stack: &mut [Frame], text: &str) {
    if !text.is_empty() {
        push_node(root, stack, Node::Text(text.to_owned()));
    }
}

fn push_node(root: &mut Vec<Node>, stack: &mut [Frame], node: Node) {
    if let Some(frame) = stack.last_mut() {
        frame.children.push(node);
    } else {
        root.push(node);
    }
}

fn render_node(
    node: &Node,
    output: &mut String,
    state: &mut RenderState,
) -> Result<(), ShortcodeError> {
    let Node::Shortcode {
        name,
        args,
        children,
    } = node
    else {
        if let Node::Text(text) = node {
            output.push_str(text);
        }
        return Ok(());
    };

    let inner = render_children(children, state)?;
    let block = is_block_level(name);
    if block {
        begin_block(output);
    }
    let markup = match name.as_str() {
        "button" => {
            let url = args.named.get("url").map(String::as_str).unwrap_or("#");
            let text = args
                .named
                .get("text")
                .map(String::as_str)
                .unwrap_or("BUTTON");
            let position = safe_alignment(args.named.get("position").map(String::as_str));
            Element::new("div")
                .attr("class", "box-1 titleflex")
                .attr("style", format!("justify-content:{position}"))
                .child(
                    Element::new("a").attr("href", url).child(
                        Element::new("div")
                            .attr("class", "btn btn-two")
                            .child(Element::new("span").child(Markup::text(text))),
                    ),
                )
                .into()
        }
        "progressbar" => {
            let value = args
                .positional
                .first()
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(0)
                .min(100);
            let color = match args.positional.get(1) {
                Some(value) if is_hex_color(value) => format!("#{value}"),
                Some(value) => {
                    return Err(ShortcodeError {
                        offset: 0,
                        message: format!("progressbar color {value:?} must contain 3, 4, 6, or 8 hexadecimal digits"),
                    });
                }
                None => "var(--accent-color)".into(),
            };
            Element::new("div")
                .attr("class", "progress_main")
                .children([
                    Element::new("div")
                        .attr("class", "progress_one")
                        .child(Markup::raw(inner))
                        .into(),
                    Element::new("div")
                        .attr("class", "progress_two")
                        .attr("role", "progressbar")
                        .attr("aria-label", "Progress")
                        .attr("aria-valuemin", "0")
                        .attr("aria-valuemax", "100")
                        .attr("aria-valuenow", value.to_string())
                        .child(Element::new("div").attr(
                            "style",
                            format!("height:2em;width:{value}%;background-color:{color}"),
                        ))
                        .into(),
                ])
                .into()
        }
        "command" => Element::new("div")
            .attr("class", "commandframeholder")
            .child(
                Element::new("div")
                    .attr("class", "textframe command")
                    .children([Markup::raw("&nbsp;&nbsp;$&nbsp;&nbsp;"), Markup::raw(inner)]),
            )
            .into(),
        "hr" => Element::new("div")
            .attr("class", "imageframeholder")
            .child(
                Element::void("hr")
                    .attr("class", "progress_hr")
                    .attr("role", "progressbar")
                    .attr("aria-label", "Reading progress")
                    .attr("aria-valuemin", "0")
                    .attr("aria-valuemax", "100")
                    .attr("aria-valuenow", "0"),
            )
            .into(),
        "image" => {
            let url = args.named.get("url").map(String::as_str).unwrap_or("");
            // Omission and `alt=""` have different authoring semantics. Keep
            // an omitted alternative absent so the typed-document pass may
            // derive it from an unambiguous plain-text caption; preserve an
            // explicitly empty alternative as decorative intent.
            let width = numeric(args.named.get("width").map(String::as_str), 48, 1, 100);
            let radius = numeric(args.named.get("radius").map(String::as_str), 10, 0, 100);
            let border = numeric(args.named.get("border").map(String::as_str), 1, 0, 20);
            let mut image = Element::void("img").attr("src", url);
            if let Some(alt) = args.named.get("alt") {
                image = image.attr("alt", alt);
            }
            Element::new("figure")
                .attr("class", "imageframeholder")
                .children([
                    Element::new("div")
                        .attr("class", "imageimageframe")
                        .attr(
                            "style",
                            format!(
                                "width:{width}%;border-radius:{radius}px;border:{border}px solid var(--accent-color)"
                            ),
                        )
                        .child(image)
                        .into(),
                    Element::new("figcaption")
                        .attr("class", "imagetextframe")
                        .child(Markup::raw(inner))
                        .into(),
                ])
                .into()
        }
        "block" | "note" => Element::new("div")
            .attr("class", "textframeholder")
            .child(
                Element::new("div")
                    .attr(
                        "class",
                        if args.named.contains_key("type") {
                            "textframefill"
                        } else {
                            "textframe"
                        },
                    )
                    .child(Markup::raw(inner)),
            )
            .into(),
        "hide" => {
            let title = args
                .named
                .get("title")
                .map(String::as_str)
                .unwrap_or("note");
            Element::new("div")
                .attr("class", "hideframeholder")
                .child(
                    Element::new("div").attr("class", "hide_box").child(
                        Element::new("div").attr("class", "hide_inner").child(
                            Element::new("details").children([
                                Element::new("summary").child(Markup::text(title)).into(),
                                Element::new("div")
                                    .attr("class", "hide_content")
                                    .child(Markup::raw(inner))
                                    .into(),
                            ]),
                        ),
                    ),
                )
                .into()
        }
        "tip" => {
            let kind = args.named.get("type").map(String::as_str).unwrap_or("NOTE");
            Element::new("div")
                .attr("class", "tipframeholder")
                .child(
                    Element::new("div")
                        .attr("class", format!("tip tip-{kind}"))
                        .child(
                            Element::new("div").attr("class", "tip-inner").children([
                                Element::new("strong").child(Markup::text(kind)).into(),
                                Element::new("div")
                                    .attr("class", "tip-content")
                                    .child(Markup::raw(inner))
                                    .into(),
                            ]),
                        ),
                )
                .into()
        }
        "sidenote" => {
            state.disclosure += 1;
            let id = format!("sn-faqe-{}", state.disclosure);
            let body = if let Some(link) = args.named.get("link") {
                Element::new("a")
                    .attr("href", link)
                    .child(Markup::raw(inner))
                    .into()
            } else {
                Markup::raw(inner)
            };
            Markup::Sequence(vec![
                Element::new("label")
                    .attr("for", &id)
                    .attr("class", "margin-toggle sidenote-number")
                    .attr("aria-label", "Toggle sidenote")
                    .into(),
                Element::void("input")
                    .attr("type", "checkbox")
                    .attr("id", &id)
                    .attr("class", "margin-toggle")
                    .attr("aria-label", "Toggle sidenote")
                    .into(),
                Element::new("span")
                    .attr("class", "sidenote")
                    .attr("style", "padding:0 2%")
                    .child(body)
                    .into(),
            ])
        }
        "sideimage" => {
            state.disclosure += 1;
            let id = format!("mn-faqe-{}", state.disclosure);
            let url = args.named.get("url").map(String::as_str).unwrap_or("");
            Markup::Sequence(vec![
                Element::new("label")
                    .attr("for", &id)
                    .attr("class", "margin-toggle")
                    .attr("aria-label", "Toggle margin image")
                    .child(Markup::text("⊕"))
                    .into(),
                Element::void("input")
                    .attr("type", "checkbox")
                    .attr("id", &id)
                    .attr("class", "margin-toggle")
                    .attr("aria-label", "Toggle margin image")
                    .into(),
                Element::new("span")
                    .attr("class", "marginnote")
                    .children([
                        Element::void("img").attr("src", url).attr("alt", "").into(),
                        Markup::raw(inner),
                    ])
                    .into(),
            ])
        }
        "section" => Element::new("section")
            .boolean_attr("data-shortcode-section")
            .child(Markup::raw(inner))
            .into(),
        "slide" => {
            let mut slide = Element::new("section").boolean_attr("data-shortcode-slide");
            for (key, value) in &args.named {
                if !is_attribute_name(key) {
                    return Err(ShortcodeError {
                        offset: 0,
                        message: format!("slide attribute name {key:?} is invalid"),
                    });
                }
                slide = slide.attr(
                    if key == "class" || key == "id" {
                        key.clone()
                    } else {
                        format!("data-{key}")
                    },
                    value,
                );
            }
            slide.child(Markup::raw(inner)).into()
        }
        "highlight" => {
            let language = escape_attribute(
                args.positional
                    .first()
                    .map(String::as_str)
                    .unwrap_or("text"),
            );
            let longest_fence = inner
                .split(|character| character != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .max(2)
                + 1;
            let fence = "`".repeat(longest_fence);
            Markup::raw(format!("\n{fence}{language}\n{inner}\n{fence}\n"))
        }
        "flexbox" => Element::new("div")
            .attr("class", "mainflex")
            .child(
                Element::new("div")
                    .attr("class", "innerflex")
                    .child(Markup::raw(inner)),
            )
            .into(),
        other => {
            return Err(ShortcodeError {
                offset: 0,
                message: format!("unsupported shortcode {other:?}"),
            })
        }
    };
    markup.write_to(output);
    if block {
        end_block(output);
    }
    Ok(())
}

fn is_block_level(name: &str) -> bool {
    !matches!(name, "sidenote" | "sideimage")
}

fn begin_block(output: &mut String) {
    if !output.ends_with("\n\n") {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
}

fn end_block(output: &mut String) {
    if !output.ends_with("\n\n") {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
}

fn render_children(children: &[Node], state: &mut RenderState) -> Result<String, ShortcodeError> {
    let mut output = String::new();
    for child in children {
        render_node(child, &mut output, state)?;
    }
    Ok(output)
}

fn numeric(value: Option<&str>, default: u16, minimum: u16, maximum: u16) -> u16 {
    value
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
        .clamp(minimum, maximum)
}

fn is_hex_color(value: &str) -> bool {
    matches!(value.len(), 3 | 4 | 6 | 8) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_attribute_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn safe_alignment(value: Option<&str>) -> &'static str {
    match value {
        Some("left") | Some("start") | Some("flex-start") => "flex-start",
        Some("right") | Some("end") | Some("flex-end") => "flex-end",
        _ => "center",
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attribute(value: &str) -> String {
    escape_html(value)
}
