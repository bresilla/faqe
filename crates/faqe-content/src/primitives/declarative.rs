use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use pulldown_cmark::{html, Event, Options, Parser};
use serde::Deserialize;

const MAX_GRAPH_COLUMNS: u8 = 12;
const MAX_GRAPH_ROWS: u8 = 12;
const MAX_GRAPH_NODES: usize = 64;
const MAX_GRAPH_EDGES: usize = 128;
const MAX_ITEMS: usize = 100;
const MAX_TABLE_COLUMNS: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveError {
    pub kind: String,
    pub message: String,
}

impl fmt::Display for PrimitiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} primitive: {}", self.kind, self.message)
    }
}

impl std::error::Error for PrimitiveError {}

struct PrimitiveSpec {
    name: &'static str,
    render: fn(&str) -> Result<String, String>,
}

const PRIMITIVES: &[PrimitiveSpec] = &[
    PrimitiveSpec {
        name: "hero",
        render: render_hero,
    },
    PrimitiveSpec {
        name: "prose",
        render: render_prose,
    },
    PrimitiveSpec {
        name: "graph",
        render: render_graph,
    },
    PrimitiveSpec {
        name: "timeline",
        render: render_timeline,
    },
    PrimitiveSpec {
        name: "progress",
        render: render_progress,
    },
    PrimitiveSpec {
        name: "grid",
        render: render_grid,
    },
    PrimitiveSpec {
        name: "table",
        render: render_table,
    },
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hero {
    #[serde(default)]
    eyebrow: String,
    title: String,
    #[serde(default)]
    standfirst: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    image_alt: String,
    #[serde(default)]
    image_caption: String,
    #[serde(default)]
    stats: Vec<HeroStat>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroStat {
    label: String,
    value: String,
}

fn render_hero(source: &str) -> Result<String, String> {
    let hero: Hero = parse_toml(source)?;
    if hero.title.trim().is_empty() {
        return Err("title cannot be empty".into());
    }
    if hero.stats.len() > 8 {
        return Err("hero may contain at most 8 stats".into());
    }
    if hero.image.trim().is_empty() != hero.image_alt.trim().is_empty() {
        return Err("image and image_alt must be provided together".into());
    }
    if hero.image.trim().is_empty() && !hero.image_caption.trim().is_empty() {
        return Err("image_caption requires an image".into());
    }
    let mut output = String::from(
        "<header class=\"faqe-primitive faqe-feature-hero\" data-faqe-primitive=\"hero\">",
    );
    if !hero.eyebrow.is_empty() {
        fmt_write(
            &mut output,
            format_args!("<small>{}</small>", inline_markdown(&hero.eyebrow)),
        );
    }
    fmt_write(
        &mut output,
        format_args!("<strong>{}</strong>", inline_markdown(&hero.title)),
    );
    if !hero.standfirst.is_empty() {
        fmt_write(
            &mut output,
            format_args!("<p>{}</p>", inline_markdown(&hero.standfirst)),
        );
    }
    if !hero.image.is_empty() {
        fmt_write(
            &mut output,
            format_args!(
                "<figure class=\"faqe-feature-hero-media\"><img src=\"{}\" alt=\"{}\" loading=\"eager\">",
                escape_html(&hero.image),
                escape_html(&hero.image_alt)
            ),
        );
        if !hero.image_caption.is_empty() {
            fmt_write(
                &mut output,
                format_args!(
                    "<figcaption>{}</figcaption>",
                    inline_markdown(&hero.image_caption)
                ),
            );
        }
        output.push_str("</figure>");
    }
    if !hero.stats.is_empty() {
        output.push_str("<dl>");
        for stat in &hero.stats {
            if stat.label.trim().is_empty() || stat.value.trim().is_empty() {
                return Err("every hero stat requires a non-empty label and value".into());
            }
            fmt_write(
                &mut output,
                format_args!(
                    "<div><dt>{}</dt><dd>{}</dd></div>",
                    inline_markdown(&stat.label),
                    inline_markdown(&stat.value)
                ),
            );
        }
        output.push_str("</dl>");
    }
    output.push_str("</header>");
    Ok(output)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProseColumns {
    columns: Vec<String>,
}

fn render_prose(source: &str) -> Result<String, String> {
    let prose: ProseColumns = parse_toml(source)?;
    if !(2..=4).contains(&prose.columns.len()) {
        return Err("prose columns must contain 2 to 4 entries".into());
    }
    let mut output = String::from(
        "<div class=\"faqe-primitive faqe-prose-columns\" data-faqe-primitive=\"prose\">",
    );
    for column in &prose.columns {
        if column.trim().is_empty() {
            return Err("prose columns cannot be empty".into());
        }
        fmt_write(
            &mut output,
            format_args!("<div>{}</div>", block_markdown(column)),
        );
    }
    output.push_str("</div>");
    Ok(output)
}

pub fn render(language: &str, source: &str) -> Result<Option<String>, PrimitiveError> {
    let Some(name) = language.trim().strip_prefix("faqe:") else {
        return Ok(None);
    };
    let Some(spec) = PRIMITIVES.iter().find(|spec| spec.name == name) else {
        let available = PRIMITIVES
            .iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(PrimitiveError {
            kind: name.to_owned(),
            message: format!("unknown primitive; available primitives: {available}"),
        });
    };
    (spec.render)(source)
        .map(Some)
        .map_err(|message| PrimitiveError {
            kind: name.to_owned(),
            message,
        })
}

fn parse_toml<T>(source: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    toml::from_str(source).map_err(|error| error.to_string())
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Tone {
    #[default]
    Accent,
    Positive,
    Warning,
    Negative,
    Muted,
    Neutral,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::Positive => "positive",
            Self::Warning => "warning",
            Self::Negative => "negative",
            Self::Muted => "muted",
            Self::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Graph {
    #[serde(default)]
    title: String,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    description: String,
    columns: u8,
    rows: u8,
    #[serde(default)]
    groups: Vec<GraphGroup>,
    nodes: Vec<GraphNode>,
    #[serde(default)]
    edges: Vec<GraphEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphGroup {
    label: String,
    column: u8,
    row: u8,
    #[serde(default = "one")]
    width: u8,
    #[serde(default = "one")]
    height: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphNode {
    id: String,
    title: String,
    #[serde(default)]
    subtitle: String,
    column: u8,
    row: u8,
    #[serde(default)]
    tone: Tone,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphEdge {
    from: String,
    to: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    tone: Tone,
    #[serde(default)]
    dashed: bool,
}

fn one() -> u8 {
    1
}

fn render_graph(source: &str) -> Result<String, String> {
    let graph: Graph = parse_toml(source)?;
    validate_graph(&graph)?;
    let positions = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), graph_position(node)))
        .collect::<BTreeMap<_, _>>();
    let width = u16::from(graph.columns) * 160;
    let height = u16::from(graph.rows) * 100;
    let min_width = u16::from(graph.columns) * 160;
    let ratio = f64::from(width) / f64::from(height);
    let label = if graph.description.is_empty() {
        graph.title.as_str()
    } else {
        graph.description.as_str()
    };
    let label = if label.is_empty() { "Diagram" } else { label };

    let mut output =
        String::from("<figure class=\"faqe-primitive faqe-graph\" data-faqe-primitive=\"graph\">");
    render_caption(&graph.title, &graph.caption, &mut output);
    output.push_str("<div class=\"faqe-graph-scroll\"><div class=\"faqe-graph-canvas\" aria-hidden=\"true\" style=\"");
    fmt_write(
        &mut output,
        format_args!(
            "--faqe-graph-columns:{};--faqe-graph-rows:{};--faqe-graph-ratio:{ratio:.4};--faqe-graph-min-width:{min_width}px",
            graph.columns, graph.rows
        ),
    );
    output.push_str("\">");
    fmt_write(
        &mut output,
        format_args!(
            "<svg class=\"faqe-graph-lines\" viewBox=\"0 0 {width} {height}\" aria-hidden=\"true\">"
        ),
    );
    for edge in &graph.edges {
        let start = positions[edge.from.as_str()];
        let end = positions[edge.to.as_str()];
        let segment = clipped_segment(start, end);
        let dash = if edge.dashed { " dashed" } else { "" };
        fmt_write(
            &mut output,
            format_args!(
                "<path class=\"faqe-graph-edge tone-{}{}\" d=\"M {:.1} {:.1} L {:.1} {:.1}\"></path>",
                edge.tone.class(),
                dash,
                segment.0,
                segment.1,
                segment.2,
                segment.3
            ),
        );
        let arrow = arrow_path(segment.0, segment.1, segment.2, segment.3);
        fmt_write(
            &mut output,
            format_args!(
                "<path class=\"faqe-graph-arrow tone-{}\" d=\"{}\"></path>",
                edge.tone.class(),
                arrow
            ),
        );
        if !edge.label.is_empty() {
            let x = (segment.0 + segment.2) / 2.0;
            let y = (segment.1 + segment.3) / 2.0 - 6.0;
            fmt_write(
                &mut output,
                format_args!(
                    "<text class=\"faqe-graph-edge-label\" x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"middle\">{}</text>",
                    escape_html(&edge.label)
                ),
            );
        }
    }
    output.push_str("</svg>");
    for group in &graph.groups {
        fmt_write(
            &mut output,
            format_args!(
                "<div class=\"faqe-graph-group\" style=\"--faqe-graph-column:{};--faqe-graph-row:{};--faqe-graph-width:{};--faqe-graph-height:{}\"><span>{}</span></div>",
                group.column,
                group.row,
                group.width,
                group.height,
                escape_html(&group.label)
            ),
        );
    }
    output.push_str("<ol class=\"faqe-graph-nodes\">");
    for node in &graph.nodes {
        fmt_write(
            &mut output,
            format_args!(
                "<li class=\"faqe-graph-node tone-{}\" style=\"--faqe-graph-column:{};--faqe-graph-row:{}\"><strong>{}</strong>",
                node.tone.class(),
                node.column,
                node.row,
                inline_markdown(&node.title)
            ),
        );
        if !node.subtitle.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<small>{}</small>", inline_markdown(&node.subtitle)),
            );
        }
        output.push_str("</li>");
    }
    output.push_str("</ol></div></div><div class=\"faqe-visually-hidden\">");
    fmt_write(&mut output, format_args!("<p>{}</p>", escape_html(label)));
    output.push_str("<ol>");
    for node in &graph.nodes {
        fmt_write(
            &mut output,
            format_args!(
                "<li>{}{}</li>",
                escape_html(&node.title),
                if node.subtitle.is_empty() {
                    String::new()
                } else {
                    format!(": {}", escape_html(&node.subtitle))
                }
            ),
        );
    }
    output.push_str("</ol><ul>");
    for edge in &graph.edges {
        fmt_write(
            &mut output,
            format_args!(
                "<li>{} to {}{}</li>",
                escape_html(&edge.from),
                escape_html(&edge.to),
                if edge.label.is_empty() {
                    String::new()
                } else {
                    format!(": {}", escape_html(&edge.label))
                }
            ),
        );
    }
    output.push_str("</ul></div></figure>");
    Ok(output)
}

fn validate_graph(graph: &Graph) -> Result<(), String> {
    if !(1..=MAX_GRAPH_COLUMNS).contains(&graph.columns) {
        return Err(format!("columns must be between 1 and {MAX_GRAPH_COLUMNS}"));
    }
    if !(1..=MAX_GRAPH_ROWS).contains(&graph.rows) {
        return Err(format!("rows must be between 1 and {MAX_GRAPH_ROWS}"));
    }
    if graph.nodes.is_empty() || graph.nodes.len() > MAX_GRAPH_NODES {
        return Err(format!("nodes must contain 1 to {MAX_GRAPH_NODES} entries"));
    }
    if graph.edges.len() > MAX_GRAPH_EDGES {
        return Err(format!(
            "edges may contain at most {MAX_GRAPH_EDGES} entries"
        ));
    }
    if graph.groups.len() > MAX_GRAPH_NODES {
        return Err(format!(
            "groups may contain at most {MAX_GRAPH_NODES} entries"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut cells = BTreeSet::new();
    for node in &graph.nodes {
        if node.id.trim().is_empty() || node.title.trim().is_empty() {
            return Err("every node requires a non-empty id and title".into());
        }
        if !node
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(format!(
                "node id {:?} must be ASCII letters, digits, - or _",
                node.id
            ));
        }
        if !ids.insert(node.id.as_str()) {
            return Err(format!("node id {:?} is duplicated", node.id));
        }
        validate_cell(node.column, node.row, graph.columns, graph.rows, "node")?;
        if !cells.insert((node.column, node.row)) {
            return Err(format!(
                "more than one node occupies column {}, row {}",
                node.column, node.row
            ));
        }
    }
    for group in &graph.groups {
        if group.label.trim().is_empty() {
            return Err("every graph group requires a non-empty label".into());
        }
        validate_cell(group.column, group.row, graph.columns, graph.rows, "group")?;
        if group.width == 0
            || group.height == 0
            || u16::from(group.column) + u16::from(group.width) - 1 > u16::from(graph.columns)
            || u16::from(group.row) + u16::from(group.height) - 1 > u16::from(graph.rows)
        {
            return Err(format!(
                "graph group {:?} exceeds the graph bounds",
                group.label
            ));
        }
    }
    for edge in &graph.edges {
        if !ids.contains(edge.from.as_str()) {
            return Err(format!(
                "edge references unknown source node {:?}",
                edge.from
            ));
        }
        if !ids.contains(edge.to.as_str()) {
            return Err(format!("edge references unknown target node {:?}", edge.to));
        }
        if edge.from == edge.to {
            return Err(format!(
                "edge on node {:?} cannot point to itself",
                edge.from
            ));
        }
    }
    Ok(())
}

fn validate_cell(column: u8, row: u8, columns: u8, rows: u8, subject: &str) -> Result<(), String> {
    if column == 0 || column > columns || row == 0 || row > rows {
        return Err(format!(
            "{subject} position column {column}, row {row} is outside the {columns} by {rows} graph"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Position {
    x: f64,
    y: f64,
}

fn graph_position(node: &GraphNode) -> Position {
    Position {
        x: (f64::from(node.column) - 0.5) * 160.0,
        y: (f64::from(node.row) - 0.5) * 100.0,
    }
}

fn clipped_segment(start: Position, end: Position) -> (f64, f64, f64, f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let scale = |half_width: f64, half_height: f64| {
        let x = if dx.abs() < f64::EPSILON {
            f64::INFINITY
        } else {
            half_width / dx.abs()
        };
        let y = if dy.abs() < f64::EPSILON {
            f64::INFINITY
        } else {
            half_height / dy.abs()
        };
        x.min(y)
    };
    let start_scale = scale(64.0, 29.0);
    let end_scale = scale(64.0, 29.0);
    (
        start.x + dx * start_scale,
        start.y + dy * start_scale,
        end.x - dx * end_scale,
        end.y - dy * end_scale,
    )
}

fn arrow_path(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> String {
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let length = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
    let ux = dx / length;
    let uy = dy / length;
    let base_x = end_x - ux * 9.0;
    let base_y = end_y - uy * 9.0;
    let left_x = base_x - uy * 4.5;
    let left_y = base_y + ux * 4.5;
    let right_x = base_x + uy * 4.5;
    let right_y = base_y - ux * 4.5;
    format!("M {end_x:.1} {end_y:.1} L {left_x:.1} {left_y:.1} L {right_x:.1} {right_y:.1} Z")
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Timeline {
    #[serde(default)]
    title: String,
    #[serde(default)]
    caption: String,
    items: Vec<TimelineItem>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineItem {
    title: String,
    #[serde(default)]
    meta: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    tone: Tone,
}

fn render_timeline(source: &str) -> Result<String, String> {
    let timeline: Timeline = parse_toml(source)?;
    validate_items(&timeline.items, "timeline")?;
    let mut output = String::from(
        "<figure class=\"faqe-primitive faqe-timeline\" data-faqe-primitive=\"timeline\">",
    );
    render_caption(&timeline.title, &timeline.caption, &mut output);
    output.push_str("<ol class=\"faqe-timeline-list\">");
    for item in &timeline.items {
        if item.title.trim().is_empty() {
            return Err("every timeline item requires a non-empty title".into());
        }
        fmt_write(
            &mut output,
            format_args!(
                "<li class=\"faqe-timeline-item tone-{}\"><div class=\"faqe-timeline-marker\" aria-hidden=\"true\"></div><div class=\"faqe-timeline-content\"><strong>{}</strong>",
                item.tone.class(),
                inline_markdown(&item.title)
            ),
        );
        if !item.meta.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<small>{}</small>", inline_markdown(&item.meta)),
            );
        }
        if !item.body.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<div>{}</div>", inline_markdown(&item.body)),
            );
        }
        output.push_str("</div></li>");
    }
    output.push_str("</ol></figure>");
    Ok(output)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    #[serde(default)]
    title: String,
    #[serde(default)]
    caption: String,
    #[serde(default = "hundred")]
    max: f64,
    items: Vec<ProgressItem>,
    #[serde(default)]
    markers: Vec<ProgressMarker>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressItem {
    label: String,
    value: f64,
    #[serde(default)]
    text: String,
    #[serde(default)]
    display: String,
    #[serde(default)]
    tone: Tone,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressMarker {
    label: String,
    value: f64,
    #[serde(default)]
    tone: Tone,
}

fn hundred() -> f64 {
    100.0
}

fn render_progress(source: &str) -> Result<String, String> {
    let progress: Progress = parse_toml(source)?;
    validate_items(&progress.items, "progress")?;
    if progress.markers.len() > MAX_ITEMS {
        return Err(format!("progress may contain at most {MAX_ITEMS} markers"));
    }
    if !progress.max.is_finite() || progress.max <= 0.0 {
        return Err("max must be a finite number greater than zero".into());
    }
    let mut output = String::from(
        "<figure class=\"faqe-primitive faqe-progress\" data-faqe-primitive=\"progress\">",
    );
    render_caption(&progress.title, &progress.caption, &mut output);
    output.push_str("<div class=\"faqe-progress-list\">");
    for item in &progress.items {
        if item.label.trim().is_empty() {
            return Err("every progress item requires a non-empty label".into());
        }
        validate_measure(item.value, progress.max, "progress item")?;
        let percent = item.value / progress.max * 100.0;
        let display = if item.display.is_empty() {
            format_number(item.value)
        } else {
            item.display.clone()
        };
        fmt_write(
            &mut output,
            format_args!(
                "<div class=\"faqe-progress-item tone-{}\"><strong class=\"faqe-progress-label\">{}</strong><div class=\"faqe-progress-track\" role=\"progressbar\" aria-label=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"{}\" aria-valuenow=\"{}\"><span class=\"faqe-progress-bar\" style=\"--faqe-progress-value:{percent:.4}%\">{}</span></div><span class=\"faqe-progress-value\">{}</span></div>",
                item.tone.class(),
                inline_markdown(&item.label),
                escape_html(&item.label),
                format_number(progress.max),
                format_number(item.value),
                inline_markdown(&item.text),
                escape_html(&display)
            ),
        );
    }
    output.push_str("</div>");
    if !progress.markers.is_empty() {
        output.push_str("<div class=\"faqe-progress-markers\" aria-label=\"Progress thresholds\">");
        for marker in &progress.markers {
            if marker.label.trim().is_empty() {
                return Err("every progress marker requires a non-empty label".into());
            }
            validate_measure(marker.value, progress.max, "progress marker")?;
            let percent = marker.value / progress.max * 100.0;
            fmt_write(
                &mut output,
                format_args!(
                    "<span class=\"faqe-progress-marker tone-{}\" style=\"--faqe-progress-marker:{percent:.4}%\">{}</span>",
                    marker.tone.class(),
                    inline_markdown(&marker.label)
                ),
            );
        }
        output.push_str("</div>");
    }
    output.push_str("</figure>");
    Ok(output)
}

fn validate_measure(value: f64, max: f64, subject: &str) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 || value > max {
        return Err(format!(
            "{subject} value {value} must be between 0 and {max}"
        ));
    }
    Ok(())
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GridVariant {
    #[default]
    Cards,
    Metrics,
    Compact,
    Strip,
}

impl GridVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Cards => "cards",
            Self::Metrics => "metrics",
            Self::Compact => "compact",
            Self::Strip => "strip",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grid {
    #[serde(default)]
    title: String,
    #[serde(default)]
    caption: String,
    #[serde(default = "default_grid_columns")]
    columns: u8,
    #[serde(default)]
    variant: GridVariant,
    items: Vec<GridItem>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GridItem {
    #[serde(default)]
    eyebrow: String,
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    badge: String,
    #[serde(default)]
    tone: Tone,
    #[serde(default)]
    lines: Vec<GridLine>,
    #[serde(default)]
    bullets: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GridLine {
    label: String,
    #[serde(default)]
    meta: String,
    value: String,
    #[serde(default)]
    tone: Tone,
}

fn default_grid_columns() -> u8 {
    3
}

fn render_grid(source: &str) -> Result<String, String> {
    let grid: Grid = parse_toml(source)?;
    validate_items(&grid.items, "grid")?;
    if !(1..=6).contains(&grid.columns) {
        return Err("columns must be between 1 and 6".into());
    }
    let mut output =
        String::from("<figure class=\"faqe-primitive faqe-grid\" data-faqe-primitive=\"grid\">");
    render_caption(&grid.title, &grid.caption, &mut output);
    fmt_write(
        &mut output,
        format_args!(
            "<div class=\"faqe-grid-items variant-{}\" style=\"--faqe-grid-columns:{}\">",
            grid.variant.class(),
            grid.columns
        ),
    );
    for item in &grid.items {
        if item.title.trim().is_empty() {
            return Err("every grid item requires a non-empty title".into());
        }
        fmt_write(
            &mut output,
            format_args!(
                "<article class=\"faqe-grid-item tone-{}\">",
                item.tone.class()
            ),
        );
        if !item.eyebrow.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<small>{}</small>", inline_markdown(&item.eyebrow)),
            );
        }
        fmt_write(
            &mut output,
            format_args!("<strong>{}</strong>", inline_markdown(&item.title)),
        );
        if !item.body.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<div>{}</div>", inline_markdown(&item.body)),
            );
        }
        if !item.badge.is_empty() {
            fmt_write(
                &mut output,
                format_args!("<span>{}</span>", inline_markdown(&item.badge)),
            );
        }
        if item.lines.len() > MAX_ITEMS || item.bullets.len() > MAX_ITEMS {
            return Err("grid items may contain at most 100 lines or bullets".into());
        }
        if !item.lines.is_empty() {
            output.push_str("<dl class=\"faqe-grid-readings\">");
            for line in &item.lines {
                if line.label.trim().is_empty() || line.value.trim().is_empty() {
                    return Err("grid lines require a non-empty label and value".into());
                }
                fmt_write(
                    &mut output,
                    format_args!(
                        "<div class=\"tone-{}\"><dt>{}{}</dt><dd>{}</dd></div>",
                        line.tone.class(),
                        inline_markdown(&line.label),
                        if line.meta.is_empty() {
                            String::new()
                        } else {
                            format!("<small>{}</small>", inline_markdown(&line.meta))
                        },
                        inline_markdown(&line.value)
                    ),
                );
            }
            output.push_str("</dl>");
        }
        if !item.bullets.is_empty() {
            output.push_str("<ul class=\"faqe-grid-bullets\">");
            for bullet in &item.bullets {
                fmt_write(
                    &mut output,
                    format_args!("<li>{}</li>", inline_markdown(bullet)),
                );
            }
            output.push_str("</ul>");
        }
        output.push_str("</article>");
    }
    output.push_str("</div></figure>");
    Ok(output)
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TableVariant {
    #[default]
    Plain,
    Striped,
    Comparison,
    Matrix,
    Metrics,
}

impl TableVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Striped => "striped",
            Self::Comparison => "comparison",
            Self::Matrix => "matrix",
            Self::Metrics => "metrics",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataTable {
    #[serde(default)]
    title: String,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    variant: TableVariant,
    columns: Vec<String>,
    rows: Vec<TableRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableRow {
    cells: Vec<String>,
    #[serde(default)]
    tones: Vec<Tone>,
}

fn render_table(source: &str) -> Result<String, String> {
    let table: DataTable = parse_toml(source)?;
    if table.columns.is_empty() || table.columns.len() > MAX_TABLE_COLUMNS {
        return Err(format!(
            "columns must contain 1 to {MAX_TABLE_COLUMNS} entries"
        ));
    }
    if table.columns.iter().any(|column| column.trim().is_empty()) {
        return Err("table column labels cannot be empty".into());
    }
    validate_items(&table.rows, "table")?;
    for (index, row) in table.rows.iter().enumerate() {
        if row.cells.len() != table.columns.len() {
            return Err(format!(
                "row {} has {} cells but the table has {} columns",
                index + 1,
                row.cells.len(),
                table.columns.len()
            ));
        }
        if !row.tones.is_empty() && row.tones.len() != row.cells.len() {
            return Err(format!(
                "row {} tones must be empty or match its {} cells",
                index + 1,
                row.cells.len()
            ));
        }
    }
    let mut output = String::from(
        "<figure class=\"faqe-primitive faqe-data-table\" data-faqe-primitive=\"table\">",
    );
    render_caption(&table.title, "", &mut output);
    output.push_str("<div class=\"faqe-table-scroll\"><table class=\"variant-");
    output.push_str(table.variant.class());
    output.push_str("\">");
    if !table.caption.is_empty() {
        fmt_write(
            &mut output,
            format_args!("<caption>{}</caption>", inline_markdown(&table.caption)),
        );
    }
    output.push_str("<thead><tr>");
    for column in &table.columns {
        fmt_write(
            &mut output,
            format_args!("<th scope=\"col\">{}</th>", inline_markdown(column)),
        );
    }
    output.push_str("</tr></thead><tbody>");
    for row in &table.rows {
        output.push_str("<tr>");
        for (index, cell) in row.cells.iter().enumerate() {
            let tone = row.tones.get(index).copied().unwrap_or(Tone::Neutral);
            if index == 0 {
                fmt_write(
                    &mut output,
                    format_args!(
                        "<th scope=\"row\" class=\"tone-{}\">{}</th>",
                        tone.class(),
                        inline_markdown(cell)
                    ),
                );
            } else {
                fmt_write(
                    &mut output,
                    format_args!(
                        "<td class=\"tone-{}\">{}</td>",
                        tone.class(),
                        inline_markdown(cell)
                    ),
                );
            }
        }
        output.push_str("</tr>");
    }
    output.push_str("</tbody></table></div></figure>");
    Ok(output)
}

fn validate_items<T>(items: &[T], subject: &str) -> Result<(), String> {
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(format!("{subject} must contain 1 to {MAX_ITEMS} items"));
    }
    Ok(())
}

fn render_caption(title: &str, caption: &str, output: &mut String) {
    if title.is_empty() && caption.is_empty() {
        return;
    }
    output.push_str("<figcaption class=\"faqe-primitive-caption\">");
    if !title.is_empty() {
        fmt_write(
            output,
            format_args!("<strong>{}</strong>", inline_markdown(title)),
        );
    }
    if !caption.is_empty() {
        fmt_write(
            output,
            format_args!("<span>{}</span>", inline_markdown(caption)),
        );
    }
    output.push_str("</figcaption>");
}

fn inline_markdown(source: &str) -> String {
    const SENTINEL: &str = "FAQE_INLINE_SENTINEL";
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let wrapped = format!("{SENTINEL}{normalized}");
    let parser =
        Parser::new_ext(&wrapped, Options::ENABLE_STRIKETHROUGH).map(|event| match event {
            Event::Html(value) | Event::InlineHtml(value) => Event::Text(value),
            other => other,
        });
    let mut output = String::new();
    html::push_html(&mut output, parser);
    let inline = output
        .strip_prefix("<p>")
        .and_then(|value| value.strip_suffix("</p>\n"))
        .unwrap_or(&output);
    inline.strip_prefix(SENTINEL).unwrap_or(inline).to_owned()
}

fn block_markdown(source: &str) -> String {
    let parser = Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH).map(|event| match event {
        Event::Html(value) | Event::InlineHtml(value) => Event::Text(value),
        other => other,
    });
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn fmt_write(output: &mut String, arguments: fmt::Arguments<'_>) {
    fmt::write(output, arguments).expect("writing to String cannot fail");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_fences_are_not_primitives() {
        assert_eq!(render("rust", "fn main() {}"), Ok(None));
    }

    #[test]
    fn timeline_renders_semantic_ordered_list() {
        let source = r#"
title = "Release path"

[[items]]
title = "Build **bundle**"
meta = "make build"
body = "Compile the site."
tone = "positive"
"#;
        let html = render("faqe:timeline", source).unwrap().unwrap();
        assert!(html.contains("data-faqe-primitive=\"timeline\""));
        assert!(html.contains("<ol class=\"faqe-timeline-list\">"));
        assert!(html.contains("Build <strong>bundle</strong>"));
    }

    #[test]
    fn hero_renders_accessible_media() {
        let source = r#"
title = "Memory with receipts"
image = "memory.jpg"
image_alt = "Glowing fragments suspended in a dark room"
image_caption = "Johnny Guitar · Unsplash"
"#;
        let html = render("faqe:hero", source).unwrap().unwrap();
        assert!(html.contains("class=\"faqe-feature-hero-media\""));
        assert!(html.contains("src=\"memory.jpg\""));
        assert!(html.contains("alt=\"Glowing fragments suspended in a dark room\""));
        assert!(html.contains("<figcaption>Johnny Guitar · Unsplash</figcaption>"));
    }

    #[test]
    fn hero_image_requires_alt_text() {
        let source = r#"
title = "Memory with receipts"
image = "memory.jpg"
"#;
        let error = render("faqe:hero", source).unwrap_err();
        assert!(error.message.contains("image_alt"));
    }

    #[test]
    fn graph_rejects_unknown_edge_nodes() {
        let source = r#"
columns = 2
rows = 1

[[nodes]]
id = "one"
title = "One"
column = 1
row = 1

[[edges]]
from = "one"
to = "two"
"#;
        let error = render("faqe:graph", source).unwrap_err();
        assert!(error.message.contains("unknown target node"));
    }

    #[test]
    fn graph_rejects_oversized_groups_without_overflowing() {
        let source = r#"
columns = 2
rows = 1

[[groups]]
label = "Too wide"
column = 1
row = 1
width = 255

[[nodes]]
id = "one"
title = "One"
column = 1
row = 1
"#;
        let error = render("faqe:graph", source).unwrap_err();
        assert!(error.message.contains("exceeds the graph bounds"));
    }

    #[test]
    fn progress_rejects_values_above_maximum() {
        let source = r#"
max = 1.0

[[items]]
label = "invalid"
value = 1.1
"#;
        let error = render("faqe:progress", source).unwrap_err();
        assert!(error.message.contains("between 0 and 1"));
    }

    #[test]
    fn table_requires_rectangular_rows() {
        let source = r#"
columns = ["A", "B"]

[[rows]]
cells = ["only one"]
"#;
        let error = render("faqe:table", source).unwrap_err();
        assert!(error.message.contains("1 cells"));
    }
}
