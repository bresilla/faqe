use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SITE_SCHEMA_VERSION: u32 = 5;
pub const DEFAULT_THEME_ID: &str = "bresilla";

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SiteBundle {
    pub schema_version: u32,
    pub site: SiteMetadata,
    pub pages: Vec<Page>,
    pub taxonomies: Taxonomies,
}

impl SiteBundle {
    pub fn new(site: SiteMetadata, mut pages: Vec<Page>) -> Self {
        pages.sort_by(|left, right| left.route.cmp(&right.route));
        let taxonomies = Taxonomies::from_pages(&pages);
        Self {
            schema_version: SITE_SCHEMA_VERSION,
            site,
            pages,
            taxonomies,
        }
    }

    pub fn page(&self, route: &str) -> Option<&Page> {
        let route = canonical_route(route);
        self.pages.iter().find(|page| page.route == route)
    }

    pub fn latest_post(&self) -> Option<&Page> {
        self.pages
            .iter()
            .filter(|page| page.kind == PageKind::Post && page.is_published())
            .max_by(|left, right| {
                left.date
                    .cmp(&right.date)
                    .then_with(|| right.route.cmp(&left.route))
            })
    }

    pub fn page_of_type(&self, content_type: &str) -> Option<&Page> {
        self.pages
            .iter()
            .find(|page| page.content_type == content_type && page.is_published())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SiteMetadata {
    /// Absolute public origin used for canonical metadata (for example,
    /// `https://example.com`). Deployment subpaths remain a CLI concern.
    pub site_url: String,
    pub theme: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub info: String,
    pub avatar: String,
    pub avatar_hover: String,
    pub favicon: String,
    /// Content-owned fallback used only when a card page has no thumbnail.
    pub default_card_thumbnail: String,
    pub default_foot: String,
    /// Optional content-owned disclosure shown on posts with a punchline.
    pub disclaimer_title: String,
    pub disclaimer_paragraphs: Vec<String>,
    /// Optional content-owned notice rendered after a post's reference links.
    pub references_copyright: String,
    pub references_notice: String,
    pub menu: Vec<MenuItem>,
    pub socials: Vec<SocialLink>,
    pub default_style: PageStyle,
}

impl Default for SiteMetadata {
    fn default() -> Self {
        Self {
            site_url: String::new(),
            theme: DEFAULT_THEME_ID.into(),
            title: "Site".into(),
            author: String::new(),
            description: "Generated with FAQE".into(),
            keywords: Vec::new(),
            info: String::new(),
            avatar: String::new(),
            avatar_hover: String::new(),
            favicon: String::new(),
            default_card_thumbnail: String::new(),
            default_foot: String::new(),
            disclaimer_title: String::new(),
            disclaimer_paragraphs: Vec::new(),
            references_copyright: String::new(),
            references_notice: String::new(),
            menu: Vec::new(),
            socials: Vec::new(),
            default_style: PageStyle::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub name: String,
    pub url: String,
    pub weight: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SocialLink {
    pub name: String,
    pub glyph: String,
    pub url: String,
    pub weight: i32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Page {
    pub source_path: String,
    pub route: String,
    /// Author-declared content type from the Markdown front matter.
    pub content_type: String,
    pub kind: PageKind,
    pub title: String,
    #[serde(default)]
    pub has_explicit_title: bool,
    pub slug: String,
    pub status: String,
    pub date: Option<String>,
    pub foot: Option<String>,
    pub description: Option<String>,
    pub punchline: Option<String>,
    pub tldr: Option<String>,
    pub thumbnail: Option<String>,
    pub external_link: Option<String>,
    pub part: Option<String>,
    pub credits: Vec<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub series: Vec<String>,
    pub style: PageStyle,
    pub folders: bool,
    pub page_size: usize,
    pub reading_minutes: usize,
    pub table_of_contents: Vec<TocItem>,
    pub document: Document,
    pub resume: Option<ResumeData>,
    pub talk: Option<TalkDeck>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Document {
    pub nodes: Vec<DocumentNode>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "node", rename_all = "snake_case")]
pub enum DocumentNode {
    Text { value: String },
    Element(ElementNode),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ElementNode {
    pub kind: ElementKind,
    pub tag: String,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<DocumentNode>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Heading,
    Paragraph,
    List,
    Quote,
    CodeBlock,
    Table,
    Image,
    Button,
    Progress,
    Command,
    Callout,
    Disclosure,
    SideNote,
    SideImage,
    ReadingBreak,
    Slide,
    #[default]
    AllowedHtml,
}

impl Page {
    pub fn is_published(&self) -> bool {
        self.status.is_empty() || self.status == "published"
    }

    pub fn display_foot<'a>(&'a self, site: &'a SiteMetadata) -> &'a str {
        self.foot.as_deref().unwrap_or(&site.default_foot)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PageKind {
    Front,
    Post,
    Resume,
    Talk,
    #[default]
    Section,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PageStyle {
    pub accent: String,
    #[serde(default = "default_chromatic")]
    pub chromatic: [String; 2],
    pub theme: Theme,
    pub background: String,
    pub foreground: String,
    pub video: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessiblePalette {
    /// Accent-derived foreground that meets WCAG AA for normal text on the
    /// configured page background.
    pub interactive: String,
    /// Black or white text selected for content drawn on the decorative accent.
    pub accent_text: String,
}

pub fn accessible_palette(style: &PageStyle) -> Option<AccessiblePalette> {
    let background = opaque_rgb(&style.background, Rgb::WHITE)?;
    let accent = opaque_rgb(&style.accent, background)?;
    // Derive with a small margin so 8-bit hex quantization cannot round the
    // emitted color back below the 4.5:1 boundary.
    let interactive = accessible_foreground(accent, background, 4.55);
    let black_ratio = contrast_rgb(Rgb::BLACK, accent);
    let white_ratio = contrast_rgb(Rgb::WHITE, accent);
    Some(AccessiblePalette {
        interactive: interactive.to_hex(),
        accent_text: if black_ratio >= white_ratio {
            "#000000".into()
        } else {
            "#ffffff".into()
        },
    })
}

pub fn accessible_color(preferred: &str, background: &str) -> Option<String> {
    let background = opaque_rgb(background, Rgb::WHITE)?;
    let preferred = opaque_rgb(preferred, background)?;
    Some(accessible_foreground(preferred, background, 4.55).to_hex())
}

pub fn contrasting_text(background: &str) -> Option<String> {
    let background = opaque_rgb(background, Rgb::WHITE)?;
    let black_ratio = contrast_rgb(Rgb::BLACK, background);
    let white_ratio = contrast_rgb(Rgb::WHITE, background);
    Some(if black_ratio >= white_ratio {
        "#000000".into()
    } else {
        "#ffffff".into()
    })
}

pub fn contrast_ratio(foreground: &str, background: &str) -> Option<f64> {
    let background = opaque_rgb(background, Rgb::WHITE)?;
    let foreground = opaque_rgb(foreground, background)?;
    Some(contrast_rgb(foreground, background))
}

pub fn chromatic_partner(accent: &str) -> Option<String> {
    let color = opaque_rgb(accent, Rgb::WHITE)?;
    Some(
        Rgb {
            red: color.blue,
            green: color.red,
            blue: color.green,
        }
        .to_hex(),
    )
}

fn default_chromatic() -> [String; 2] {
    ["#00e5ff".into(), "#ff2bd6".into()]
}

#[derive(Clone, Copy)]
struct Rgb {
    red: f64,
    green: f64,
    blue: f64,
}

impl Rgb {
    const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    const WHITE: Self = Self {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
    };

    fn mix(self, other: Self, amount: f64) -> Self {
        Self {
            red: self.red + (other.red - self.red) * amount,
            green: self.green + (other.green - self.green) * amount,
            blue: self.blue + (other.blue - self.blue) * amount,
        }
    }

    fn to_hex(self) -> String {
        let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!(
            "#{:02x}{:02x}{:02x}",
            channel(self.red),
            channel(self.green),
            channel(self.blue)
        )
    }
}

fn opaque_rgb(value: &str, backdrop: Rgb) -> Option<Rgb> {
    let value = value.strip_prefix('#')?;
    let expanded;
    let value = match value.len() {
        3 | 4 => {
            expanded = value
                .chars()
                .flat_map(|character| [character, character])
                .collect::<String>();
            expanded.as_str()
        }
        6 | 8 => value,
        _ => return None,
    };
    let channel = |offset| u8::from_str_radix(&value[offset..offset + 2], 16).ok();
    let rgb = Rgb {
        red: f64::from(channel(0)?) / 255.0,
        green: f64::from(channel(2)?) / 255.0,
        blue: f64::from(channel(4)?) / 255.0,
    };
    let alpha = if value.len() == 8 {
        f64::from(channel(6)?) / 255.0
    } else {
        1.0
    };
    Some(backdrop.mix(rgb, alpha))
}

fn accessible_foreground(preferred: Rgb, background: Rgb, minimum: f64) -> Rgb {
    if contrast_rgb(preferred, background) >= minimum {
        return preferred;
    }
    [Rgb::BLACK, Rgb::WHITE]
        .into_iter()
        .filter(|target| contrast_rgb(*target, background) >= minimum)
        .map(|target| {
            let mut low = 0.0;
            let mut high = 1.0;
            for _ in 0..24 {
                let middle = (low + high) / 2.0;
                if contrast_rgb(preferred.mix(target, middle), background) >= minimum {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            (high, preferred.mix(target, high))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map_or(preferred, |(_, color)| color)
}

fn contrast_rgb(first: Rgb, second: Rgb) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn relative_luminance(color: Rgb) -> f64 {
    let channel = |value: f64| {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
}

impl Default for PageStyle {
    fn default() -> Self {
        Self {
            accent: "#00e5ff".into(),
            chromatic: default_chromatic(),
            theme: Theme::Dark,
            background: "#18191c".into(),
            foreground: "#f3f3f3".into(),
            video: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TocItem {
    pub level: u8,
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeData {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub contact: ResumeContact,
    #[serde(default)]
    pub education: ResumeEducation,
    #[serde(default)]
    pub language: ResumeLanguages,
    #[serde(default)]
    pub interests: ResumeInterests,
    #[serde(default)]
    pub summary: ResumeSummary,
    #[serde(default)]
    pub experiences: ResumeSection,
    #[serde(default)]
    pub jobs: ResumeJobs,
    #[serde(default)]
    pub projects: ResumeProjects,
    #[serde(default)]
    pub skills: ResumeSkills,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeSection {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeContact {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub list: Vec<ResumeContactItem>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeContactItem {
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeEducation {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub list: Vec<ResumeEducationItem>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeEducationItem {
    #[serde(default)]
    pub degree: String,
    #[serde(default)]
    pub college: String,
    #[serde(default)]
    pub dates: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeLanguages {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub list: Vec<ResumeLanguageItem>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeLanguageItem {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub level: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeInterests {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub list: Vec<ResumeInterestItem>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeInterestItem {
    #[serde(default)]
    pub interest: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeSummary {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub summary_document: Document,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeJobs {
    #[serde(default)]
    pub list: Vec<ResumeJob>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeJob {
    #[serde(default)]
    pub position: String,
    #[serde(default)]
    pub dates: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub details: String,
    #[serde(default)]
    pub details_document: Document,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeProjects {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub intro_document: Document,
    #[serde(default)]
    pub list: Vec<ResumeProject>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeProject {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub tagline_document: Document,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeSkills {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub list: Vec<ResumeSkill>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ResumeSkill {
    #[serde(default)]
    pub skill: String,
    #[serde(default)]
    pub level: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TalkDeck {
    pub slides: Vec<TalkSlide>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TalkSlide {
    pub document: Document,
    pub attributes: BTreeMap<String, String>,
    pub vertical_group: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Taxonomies {
    pub tags: BTreeMap<String, Vec<String>>,
    pub categories: BTreeMap<String, Vec<String>>,
    pub series: BTreeMap<String, Vec<String>>,
    pub kinds: BTreeMap<String, Vec<String>>,
}

impl Taxonomies {
    fn from_pages(pages: &[Page]) -> Self {
        let mut result = Self::default();
        for page in pages.iter().filter(|page| page.is_published()) {
            push_terms(&mut result.tags, &page.tags, &page.route);
            push_terms(&mut result.categories, &page.categories, &page.route);
            push_terms(&mut result.series, &page.series, &page.route);
            // Hugo's configured `type` taxonomy indexes explicit content
            // types, not synthetic section `_index.md` pages.
            if page.kind != PageKind::Section {
                result
                    .kinds
                    .entry(format!("{:?}", page.kind).to_lowercase())
                    .or_default()
                    .push(page.route.clone());
            }
        }
        result
    }
}

fn push_terms(index: &mut BTreeMap<String, Vec<String>>, terms: &[String], route: &str) {
    for term in terms {
        index
            .entry(slugify(term))
            .or_default()
            .push(route.to_owned());
    }
}

pub fn canonical_route(route: &str) -> String {
    let path = route.split(['?', '#']).next().unwrap_or(route);
    if path.is_empty() || path == "/" {
        return "/".into();
    }
    format!("/{}/", path.trim_matches('/'))
}

pub fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            separator = false;
            result.push(character);
        } else {
            separator = true;
        }
    }
    result.trim_matches('-').to_owned()
}
