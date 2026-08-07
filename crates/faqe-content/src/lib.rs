mod shortcode;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use ammonia::{Builder as HtmlSanitizer, UrlRelative};
use chrono::NaiveDate;
use faqe_model::{
    accessible_palette, canonical_route, chromatic_partner, contrast_ratio, slugify, Document,
    DocumentNode, ElementKind, ElementNode, Page, PageKind, PageStyle, ResumeData, SiteBundle,
    SiteMetadata, TalkDeck, TalkSlide, Theme, TocItem,
};
use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, CharacterTokens, EndTag, StartTag, TagToken, Token, TokenSink, TokenSinkResult,
    Tokenizer, TokenizerOpts,
};
use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use regex::{Captures, Regex};
use sha2::{Digest, Sha256};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use unicode_normalization::UnicodeNormalization;

pub use shortcode::{ShortcodeError, ShortcodeParser};

/// Build the ownership key used for routes and emitted paths. NFC prevents
/// canonically equivalent Unicode spellings from claiming outputs that many
/// filesystems expose as the same name; lowercase keeps the existing
/// case-insensitive collision policy.
pub fn normalized_collision_key(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).nfc().collect()
}

const MAX_MARKDOWN_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SITE_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_PUBLIC_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_ALIAS_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_FEED_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_SITE_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONTENT_ASSET_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SITE_ASSET_BYTES: u64 = 128 * 1024 * 1024;
/// Keep automatically derived alternatives concise. Longer captions remain
/// visible but require an explicitly authored `alt` value.
const MAX_DERIVED_ALT_CHARS: usize = 280;
const LEGACY_LOGO_BODY_SHA256: &str =
    "20f2030e90fb2f85653b452b94b6393ac012443ef0217003ee60a84012f970dd";
const SAFE_HTML_TAGS: &[&str] = &[
    "a",
    "abbr",
    "aside",
    "blockquote",
    "br",
    "caption",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "g",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "img",
    "input",
    "kbd",
    "label",
    "li",
    "mark",
    "ol",
    "p",
    "path",
    "pre",
    "q",
    "s",
    "samp",
    "section",
    "small",
    "source",
    "span",
    "strong",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

#[derive(Debug)]
pub enum ContentError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    FrontMatter {
        path: PathBuf,
        message: String,
    },
    Shortcode {
        path: PathBuf,
        source: ShortcodeError,
    },
    RouteCollision {
        route: String,
        first: String,
        second: String,
    },
    InvalidPath {
        path: PathBuf,
        message: String,
    },
    UnsafeHtml {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
}

impl fmt::Display for ContentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::FrontMatter { path, message } => {
                write!(
                    formatter,
                    "{}: invalid front matter: {message}",
                    path.display()
                )
            }
            Self::Shortcode { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::RouteCollision {
                route,
                first,
                second,
            } => write!(
                formatter,
                "route collision for {route}: both {first} and {second} claim it"
            ),
            Self::InvalidPath { path, message } => {
                write!(formatter, "{}: invalid path: {message}", path.display())
            }
            Self::UnsafeHtml {
                path,
                line,
                column,
                message,
            } => write!(
                formatter,
                "{}:{line}:{column}: unsafe HTML: {message}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ContentError {}

#[derive(Clone, Debug)]
pub struct LoadReport {
    pub bundle: SiteBundle,
    pub assets: Vec<ContentAsset>,
    pub public_files: Vec<PublicFileSpec>,
    pub aliases: Vec<AliasSpec>,
    pub feeds: Vec<FeedSpec>,
    pub markdown_files: usize,
    pub source_bytes: u64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ContentAsset {
    pub source_path: String,
    pub output_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicFileSpec {
    pub output_path: String,
    pub source_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasSpec {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedSpec {
    pub route: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FolderVisibility {
    Tab,
    Indirect,
}

#[derive(Clone, Debug)]
struct FolderSpec {
    name: String,
    folder_type: String,
    visibility: FolderVisibility,
    item_type: Option<String>,
    title: String,
    weight: i32,
    route: String,
}

pub fn load_site(content_root: impl AsRef<Path>) -> Result<LoadReport, ContentError> {
    let content_root = content_root.as_ref();
    let canonical_root = fs::canonicalize(content_root).map_err(|source| ContentError::Io {
        path: content_root.to_owned(),
        source,
    })?;
    if !canonical_root.is_dir() {
        return Err(ContentError::InvalidPath {
            path: canonical_root,
            message: "content root is not a directory".into(),
        });
    }

    let folders = load_folder_specs(&canonical_root)?;
    let public_files = load_public_file_specs(&canonical_root)?;
    let aliases = load_alias_specs(&canonical_root)?;
    let feeds = load_feed_specs(&canonical_root)?;
    let mut files = Vec::new();
    discover_markdown(&canonical_root, &canonical_root, &mut files)?;
    files.sort();

    let parser = ShortcodeParser;
    let mut pages = Vec::with_capacity(files.len());
    let mut source_bytes = 0;
    let mut warnings = Vec::new();
    let mut claimed_routes = BTreeMap::<String, String>::new();
    let mut claimed_folded_routes = BTreeMap::<String, (String, String)>::new();
    let mut assets = BTreeMap::<String, ContentAsset>::new();
    let mut site = load_site_metadata(&canonical_root, &mut assets)?;
    site.socials.sort_by(|left, right| {
        left.weight
            .cmp(&right.weight)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.url.cmp(&right.url))
    });

    for relative_path in &files {
        let absolute_path = canonical_root.join(relative_path);
        let file_bytes = fs::metadata(&absolute_path)
            .map_err(|source| ContentError::Io {
                path: absolute_path.clone(),
                source,
            })?
            .len();
        if file_bytes > MAX_MARKDOWN_FILE_BYTES {
            return Err(ContentError::InvalidPath {
                path: relative_path.clone(),
                message: format!(
                    "Markdown file is {file_bytes} bytes; limit is {MAX_MARKDOWN_FILE_BYTES} bytes"
                ),
            });
        }
        if source_bytes + file_bytes > MAX_SITE_SOURCE_BYTES {
            return Err(ContentError::InvalidPath {
                path: canonical_root.clone(),
                message: format!("site Markdown exceeds the {MAX_SITE_SOURCE_BYTES} byte limit"),
            });
        }
        let source = fs::read_to_string(&absolute_path).map_err(|source| ContentError::Io {
            path: absolute_path.clone(),
            source,
        })?;
        source_bytes += file_bytes;
        if !source_is_published(relative_path, &source)? {
            continue;
        }
        let page = parse_page(
            &canonical_root,
            relative_path,
            &source,
            &parser,
            &mut assets,
            &site.default_style,
            &folders,
        )?;
        if let Some(first) = claimed_routes.insert(page.route.clone(), page.source_path.clone()) {
            return Err(ContentError::RouteCollision {
                route: page.route,
                first,
                second: page.source_path,
            });
        }
        let folded = normalized_collision_key(&page.route);
        if let Some((original, first)) =
            claimed_folded_routes.insert(folded, (page.route.clone(), page.source_path.clone()))
        {
            if original != page.route {
                return Err(ContentError::RouteCollision {
                    route: format!("{} (normalizes to {})", page.route, original),
                    first,
                    second: page.source_path,
                });
            }
        }
        pages.push(page);
    }

    let home = folders
        .values()
        .find(|folder| folder.folder_type == "home")
        .expect("folder validation guarantees one home folder");
    site.title = home.title.clone();
    site.menu = folders
        .values()
        .filter(|folder| folder.visibility == FolderVisibility::Tab && folder.folder_type != "home")
        .map(|folder| faqe_model::MenuItem {
            name: folder.name.clone(),
            url: folder.route.clone(),
            weight: folder.weight,
        })
        .collect();
    site.menu.sort_by(|left, right| {
        left.weight
            .cmp(&right.weight)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.url.cmp(&right.url))
    });

    if site.default_card_thumbnail.is_empty() {
        if let Some(page) = pages.iter().find(|page| {
            matches!(page.kind, PageKind::Post | PageKind::Talk) && page.thumbnail.is_none()
        }) {
            return Err(ContentError::FrontMatter {
                path: PathBuf::from(&page.source_path),
                message: "card pages require thumbnail or site.toml default_card_thumbnail".into(),
            });
        }
    }
    let bundle = SiteBundle::new(site, pages);
    validate_aliases(&canonical_root.join("aliases.toml"), &bundle, &aliases)?;
    validate_internal_routes(&bundle, &public_files, &aliases, &mut warnings);
    Ok(LoadReport {
        bundle,
        assets: assets.into_values().collect(),
        public_files,
        aliases,
        feeds,
        markdown_files: files.len(),
        source_bytes,
        warnings,
    })
}

fn load_folder_specs(content_root: &Path) -> Result<BTreeMap<String, FolderSpec>, ContentError> {
    let mut folders = BTreeMap::new();
    for entry in fs::read_dir(content_root).map_err(|source| ContentError::Io {
        path: content_root.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| ContentError::Io {
            path: content_root.to_owned(),
            source,
        })?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type().map_err(|source| ContentError::Io {
            path: entry.path(),
            source,
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let name = name.into_string().map_err(|_| ContentError::InvalidPath {
            path: entry.path(),
            message: "top-level content folder names must be UTF-8".into(),
        })?;
        if slugify(&name) != name {
            return Err(ContentError::InvalidPath {
                path: entry.path(),
                message: "top-level content folder names must be lowercase URL-safe slugs".into(),
            });
        }
        let index = entry.path().join("_index.md");
        if !index.is_file() {
            return Err(ContentError::InvalidPath {
                path: entry.path(),
                message: "every top-level content folder requires an _index.md descriptor".into(),
            });
        }
        let source = fs::read_to_string(&index).map_err(|source| ContentError::Io {
            path: index.clone(),
            source,
        })?;
        let (front_matter, _) =
            split_front_matter(&source).map_err(|message| ContentError::FrontMatter {
                path: index.clone(),
                message,
            })?;
        let value =
            front_matter
                .parse::<toml::Value>()
                .map_err(|error| ContentError::FrontMatter {
                    path: index.clone(),
                    message: toml_diagnostic(front_matter, &error),
                })?;
        let table = value.as_table().ok_or_else(|| ContentError::FrontMatter {
            path: index.clone(),
            message: "top-level value must be a TOML table".into(),
        })?;
        let required = |key: &str| {
            string(table, key).ok_or_else(|| ContentError::FrontMatter {
                path: index.clone(),
                message: format!("top-level folder descriptor requires string field {key:?}"),
            })
        };
        let folder_type = required("type")?.to_owned();
        if slugify(&folder_type) != folder_type {
            return Err(ContentError::FrontMatter {
                path: index.clone(),
                message: "folder type must be a lowercase URL-safe name".into(),
            });
        }
        let visibility = match required("visibility")? {
            "tab" => FolderVisibility::Tab,
            "indirect" => FolderVisibility::Indirect,
            other => {
                return Err(ContentError::FrontMatter {
                    path: index.clone(),
                    message: format!(
                        "folder visibility {other:?} is unsupported; expected tab or indirect"
                    ),
                });
            }
        };
        if !matches!(string(table, "status"), None | Some("published")) {
            return Err(ContentError::FrontMatter {
                path: index.clone(),
                message: "top-level folder descriptors must be published".into(),
            });
        }
        let item_type = string(table, "item_type").map(ToOwned::to_owned);
        if folder_type == "dirstruct" {
            if !matches!(item_type.as_deref(), Some("post" | "presentation")) {
                return Err(ContentError::FrontMatter {
                    path: index.clone(),
                    message: "dirstruct folders require item_type = \"post\" or \"presentation\""
                        .into(),
                });
            }
        } else if item_type.is_some() {
            return Err(ContentError::FrontMatter {
                path: index.clone(),
                message: "item_type is valid only for dirstruct folders".into(),
            });
        }
        let slug = required("slug")?.to_owned();
        validate_slug(&index, &slug)?;
        let title = required("title")?.to_owned();
        let weight = match integer(table, "weight") {
            Some(value) => i32::try_from(value).map_err(|_| ContentError::FrontMatter {
                path: index.clone(),
                message: "folder weight must fit a 32-bit signed integer".into(),
            })?,
            None if visibility == FolderVisibility::Indirect => 0,
            None => {
                return Err(ContentError::FrontMatter {
                    path: index.clone(),
                    message: "tab folders require an integer weight".into(),
                });
            }
        };
        let route = if folder_type == "home" {
            "/".into()
        } else {
            canonical_route(&slug)
        };
        folders.insert(
            name.clone(),
            FolderSpec {
                name,
                folder_type,
                visibility,
                item_type,
                title,
                weight,
                route,
            },
        );
    }
    let homes = folders
        .values()
        .filter(|folder| folder.folder_type == "home")
        .collect::<Vec<_>>();
    if homes.len() != 1 {
        return Err(ContentError::InvalidPath {
            path: content_root.to_owned(),
            message: format!(
                "content requires exactly one top-level home folder; found {}",
                homes.len()
            ),
        });
    }
    if homes[0].visibility != FolderVisibility::Tab {
        return Err(ContentError::FrontMatter {
            path: content_root.join(&homes[0].name).join("_index.md"),
            message: "the home folder must use visibility = \"tab\"".into(),
        });
    }
    let mut singleton_types = BTreeMap::<&str, &str>::new();
    for folder in folders
        .values()
        .filter(|folder| folder.folder_type != "dirstruct")
    {
        if let Some(first) = singleton_types.insert(&folder.folder_type, &folder.name) {
            return Err(ContentError::InvalidPath {
                path: content_root.to_owned(),
                message: format!(
                    "singleton folder type {:?} is declared by both {first:?} and {:?}",
                    folder.folder_type, folder.name
                ),
            });
        }
    }
    Ok(folders)
}

fn load_feed_specs(content_root: &Path) -> Result<Vec<FeedSpec>, ContentError> {
    let path = content_root.join("feeds.toml");
    if !path.exists() {
        return Ok(vec![FeedSpec {
            route: "/index.xml".into(),
        }]);
    }
    let size = fs::metadata(&path)
        .map_err(|source| ContentError::Io {
            path: path.clone(),
            source,
        })?
        .len();
    if size > MAX_FEED_MANIFEST_BYTES {
        return Err(ContentError::InvalidPath {
            path,
            message: format!(
                "feed manifest is {size} bytes; limit is {MAX_FEED_MANIFEST_BYTES} bytes"
            ),
        });
    }
    let source = fs::read_to_string(&path).map_err(|source| ContentError::Io {
        path: path.clone(),
        source,
    })?;
    let value = source
        .parse::<toml::Value>()
        .map_err(|error| ContentError::InvalidPath {
            path: path.clone(),
            message: format!("invalid TOML: {error}"),
        })?;
    let root = value.as_table().ok_or_else(|| ContentError::InvalidPath {
        path: path.clone(),
        message: "feeds.toml must contain a top-level table".into(),
    })?;
    if root.keys().any(|key| key != "routes") {
        return Err(ContentError::InvalidPath {
            path,
            message: "feeds.toml supports only the routes array".into(),
        });
    }
    let routes = root
        .get("routes")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| ContentError::InvalidPath {
            path: path.clone(),
            message: "feeds.toml requires a routes array".into(),
        })?;
    let mut specs = Vec::with_capacity(routes.len());
    let mut claimed = BTreeMap::<String, String>::new();
    for route in routes {
        let route = route.as_str().ok_or_else(|| ContentError::InvalidPath {
            path: path.clone(),
            message: "feed routes must be strings".into(),
        })?;
        let directory =
            route
                .strip_suffix("index.xml")
                .ok_or_else(|| ContentError::InvalidPath {
                    path: path.clone(),
                    message: format!("feed route {route:?} must end with index.xml"),
                })?;
        if route.contains(['?', '#', '\\'])
            || !route.starts_with('/')
            || (!directory.is_empty() && canonical_route(directory) != directory)
        {
            return Err(ContentError::InvalidPath {
                path: path.clone(),
                message: format!("feed route {route:?} is not a canonical absolute feed path"),
            });
        }
        let folded = normalized_collision_key(route);
        if let Some(first) = claimed.insert(folded, route.to_owned()) {
            return Err(ContentError::InvalidPath {
                path: path.clone(),
                message: format!("feed routes {first:?} and {route:?} collide"),
            });
        }
        specs.push(FeedSpec {
            route: route.to_owned(),
        });
    }
    specs.sort_by(|left, right| left.route.cmp(&right.route));
    if !specs.iter().any(|feed| feed.route == "/index.xml") {
        return Err(ContentError::InvalidPath {
            path,
            message: "feeds.toml must retain the root /index.xml feed".into(),
        });
    }
    Ok(specs)
}

fn load_alias_specs(content_root: &Path) -> Result<Vec<AliasSpec>, ContentError> {
    let path = content_root.join("aliases.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let size = fs::metadata(&path)
        .map_err(|source| ContentError::Io {
            path: path.clone(),
            source,
        })?
        .len();
    if size > MAX_ALIAS_MANIFEST_BYTES {
        return Err(ContentError::InvalidPath {
            path,
            message: format!(
                "alias manifest is {size} bytes; limit is {MAX_ALIAS_MANIFEST_BYTES} bytes"
            ),
        });
    }
    let source = fs::read_to_string(&path).map_err(|source| ContentError::Io {
        path: path.clone(),
        source,
    })?;
    let value = source
        .parse::<toml::Value>()
        .map_err(|error| ContentError::InvalidPath {
            path: path.clone(),
            message: format!("invalid TOML: {error}"),
        })?;
    let root = value.as_table().ok_or_else(|| ContentError::InvalidPath {
        path: path.clone(),
        message: "aliases.toml must contain a top-level table".into(),
    })?;
    if root.keys().any(|key| key != "aliases") {
        return Err(ContentError::InvalidPath {
            path,
            message: "aliases.toml supports only the [aliases] table".into(),
        });
    }
    let aliases = root
        .get("aliases")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContentError::InvalidPath {
            path: path.clone(),
            message: "aliases.toml requires an [aliases] table".into(),
        })?;
    let mut specs = Vec::with_capacity(aliases.len());
    for (from, to) in aliases {
        let to = to.as_str().ok_or_else(|| ContentError::InvalidPath {
            path: path.clone(),
            message: format!("alias {from:?} target must be a string"),
        })?;
        specs.push(AliasSpec {
            from: from.clone(),
            to: to.to_owned(),
        });
    }
    specs.sort_by(|left, right| left.from.cmp(&right.from));
    Ok(specs)
}

fn validate_aliases(
    manifest: &Path,
    bundle: &SiteBundle,
    aliases: &[AliasSpec],
) -> Result<(), ContentError> {
    let content_routes = bundle
        .pages
        .iter()
        .map(|page| page.route.clone())
        .collect::<BTreeSet<_>>();
    let canonical_routes = bundle
        .pages
        .iter()
        .filter(|page| page.is_published())
        .map(|page| page.route.clone())
        .chain(["/".to_owned()])
        .chain(is_builtin_routes(bundle))
        .collect::<BTreeSet<_>>();
    let mut claimed = BTreeMap::<String, String>::new();
    for alias in aliases {
        for (role, route) in [("source", &alias.from), ("target", &alias.to)] {
            if route.contains(['?', '#', '\\'])
                || !route.starts_with('/')
                || !route.ends_with('/')
                || canonical_route(route) != *route
            {
                return Err(ContentError::InvalidPath {
                    path: manifest.to_owned(),
                    message: format!(
                        "alias {role} {route:?} must be an absolute canonical directory route"
                    ),
                });
            }
        }
        if alias.from == alias.to {
            return Err(ContentError::InvalidPath {
                path: manifest.to_owned(),
                message: format!("alias {:?} cannot redirect to itself", alias.from),
            });
        }
        if content_routes.contains(&alias.from) || canonical_routes.contains(&alias.from) {
            return Err(ContentError::InvalidPath {
                path: manifest.to_owned(),
                message: format!(
                    "alias source {:?} collides with a canonical route",
                    alias.from
                ),
            });
        }
        if !canonical_routes.contains(&alias.to) {
            return Err(ContentError::InvalidPath {
                path: manifest.to_owned(),
                message: format!("alias target {:?} is not a canonical route", alias.to),
            });
        }
        let folded = normalized_collision_key(&alias.from);
        if let Some(first) = claimed.insert(folded, alias.from.clone()) {
            return Err(ContentError::InvalidPath {
                path: manifest.to_owned(),
                message: format!("alias sources {:?} and {:?} collide", first, alias.from),
            });
        }
    }
    Ok(())
}

fn is_builtin_routes(bundle: &SiteBundle) -> impl Iterator<Item = String> + '_ {
    let mut routes = vec![
        "/categories/".to_owned(),
        "/folder/".to_owned(),
        "/series/".to_owned(),
        "/tags/".to_owned(),
        "/type/".to_owned(),
    ];
    for (taxonomy, terms) in [
        ("categories", &bundle.taxonomies.categories),
        ("series", &bundle.taxonomies.series),
        ("tags", &bundle.taxonomies.tags),
        ("type", &bundle.taxonomies.kinds),
    ] {
        routes.extend(terms.keys().map(|term| format!("/{taxonomy}/{term}/")));
    }
    routes.into_iter()
}

fn load_public_file_specs(content_root: &Path) -> Result<Vec<PublicFileSpec>, ContentError> {
    let path = content_root.join("public-files.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let size = fs::metadata(&path)
        .map_err(|source| ContentError::Io {
            path: path.clone(),
            source,
        })?
        .len();
    if size > MAX_PUBLIC_MANIFEST_BYTES {
        return Err(ContentError::InvalidPath {
            path,
            message: format!(
                "public-file manifest is {size} bytes; limit is {MAX_PUBLIC_MANIFEST_BYTES} bytes"
            ),
        });
    }
    let source = fs::read_to_string(&path).map_err(|source| ContentError::Io {
        path: path.clone(),
        source,
    })?;
    let value = source
        .parse::<toml::Value>()
        .map_err(|error| ContentError::InvalidPath {
            path: path.clone(),
            message: format!("invalid TOML: {error}"),
        })?;
    let root = value.as_table().ok_or_else(|| ContentError::InvalidPath {
        path: path.clone(),
        message: "public-files.toml must contain a top-level table".into(),
    })?;
    if root.keys().any(|key| key != "files") {
        return Err(ContentError::InvalidPath {
            path,
            message: "public-files.toml supports only the [files] table".into(),
        });
    }
    let files = root
        .get("files")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ContentError::InvalidPath {
            path: path.clone(),
            message: "public-files.toml requires a [files] table".into(),
        })?;
    let mut specs = Vec::with_capacity(files.len());
    for (output_path, source_path) in files {
        let source_path = source_path
            .as_str()
            .ok_or_else(|| ContentError::InvalidPath {
                path: path.clone(),
                message: format!("public file {output_path:?} source must be a string"),
            })?;
        validate_public_manifest_path(&path, output_path, "output")?;
        validate_public_manifest_path(&path, source_path, "source")?;
        specs.push(PublicFileSpec {
            output_path: output_path.clone(),
            source_path: source_path.to_owned(),
        });
    }
    specs.sort_by(|left, right| left.output_path.cmp(&right.output_path));
    Ok(specs)
}

fn validate_public_manifest_path(
    manifest: &Path,
    value: &str,
    role: &str,
) -> Result<(), ContentError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ContentError::InvalidPath {
            path: manifest.to_owned(),
            message: format!("public {role} path {value:?} must be a normalized relative path"),
        });
    }
    Ok(())
}

/// Decide publication before parsing bodies or resolving their assets. This is
/// deliberately a small front-matter-only pass: a draft must not influence the
/// public bundle, route graph, diagnostics, or emitted asset set.
fn source_is_published(path: &Path, source: &str) -> Result<bool, ContentError> {
    let yaml = source
        .lines()
        .next()
        .is_some_and(|line| line.trim_end_matches('\r') == "---");
    let (front_matter, _) = if yaml {
        split_yaml_front_matter(source)
    } else {
        split_front_matter(source)
    }
    .map_err(|message| ContentError::FrontMatter {
        path: path.to_owned(),
        message,
    })?;
    if yaml {
        let value = parse_presenterm_front_matter(path, front_matter)?;
        return match yaml_string(&value, &["status"]) {
            None | Some("published") => Ok(true),
            Some("draft") | Some("unpublished") => Ok(false),
            Some(status) => Err(ContentError::FrontMatter {
                path: path.to_owned(),
                message: format!(
                    "status {status:?} is unsupported; expected published, draft, or unpublished"
                ),
            }),
        };
    }
    let value = front_matter
        .parse::<toml::Value>()
        .map_err(|error| ContentError::FrontMatter {
            path: path.to_owned(),
            message: toml_diagnostic(front_matter, &error),
        })?;
    let table = value.as_table().ok_or_else(|| ContentError::FrontMatter {
        path: path.to_owned(),
        message: "top-level value must be a TOML table".into(),
    })?;
    match string(table, "status") {
        None | Some("published") => Ok(true),
        Some("draft") | Some("unpublished") => Ok(false),
        Some(status) => Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!(
                "status {status:?} is unsupported; expected published, draft, or unpublished"
            ),
        }),
    }
}

fn load_site_metadata(
    content_root: &Path,
    assets: &mut BTreeMap<String, ContentAsset>,
) -> Result<SiteMetadata, ContentError> {
    let path = content_root.join("site.toml");
    if !path.exists() {
        return Ok(SiteMetadata::default());
    }
    let size = fs::metadata(&path)
        .map_err(|source| ContentError::Io {
            path: path.clone(),
            source,
        })?
        .len();
    if size > MAX_SITE_CONFIG_BYTES {
        return Err(ContentError::InvalidPath {
            path,
            message: format!("site.toml is {size} bytes; limit is {MAX_SITE_CONFIG_BYTES} bytes"),
        });
    }
    let source = fs::read_to_string(&path).map_err(|source| ContentError::Io {
        path: path.clone(),
        source,
    })?;
    let raw = source
        .parse::<toml::Value>()
        .map_err(|error| ContentError::FrontMatter {
            path: PathBuf::from("site.toml"),
            message: toml_diagnostic(&source, &error),
        })?;
    if raw
        .as_table()
        .is_some_and(|table| table.contains_key("title") || table.contains_key("menu"))
    {
        return Err(ContentError::FrontMatter {
            path: PathBuf::from("site.toml"),
            message:
                "site title and navigation are defined by top-level content folders, not site.toml"
                    .into(),
        });
    }
    let mut site: SiteMetadata =
        toml::from_str(&source).map_err(|error| ContentError::FrontMatter {
            path: PathBuf::from("site.toml"),
            message: toml_diagnostic(&source, &error),
        })?;
    validate_theme_id(&path, &site.theme)?;
    validate_style(&path, &site.default_style)?;
    for item in &site.menu {
        validate_public_url(&path, &item.url, true)?;
    }
    for item in &site.socials {
        validate_public_url(&path, &item.url, false)?;
    }
    site.site_url =
        normalize_site_url(&site.site_url).map_err(|message| ContentError::FrontMatter {
            path: PathBuf::from("site.toml"),
            message,
        })?;
    for value in [
        &mut site.avatar,
        &mut site.avatar_hover,
        &mut site.favicon,
        &mut site.default_card_thumbnail,
    ] {
        if !value.is_empty() {
            *value = resolve_asset_url(content_root, Path::new("site.toml"), value, assets)?;
        }
    }
    Ok(site)
}

fn validate_theme_id(path: &Path, theme: &str) -> Result<(), ContentError> {
    let valid = !theme.is_empty()
        && theme.bytes().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
        })
        && theme
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && theme
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && !theme.contains("--");
    if valid {
        Ok(())
    } else {
        Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!(
                "theme {theme:?} must be a lowercase ASCII identifier containing letters, digits, and single hyphens"
            ),
        })
    }
}

#[cfg(test)]
mod theme_id_tests {
    use super::*;

    #[test]
    fn theme_ids_are_safe_registry_keys() {
        let path = Path::new("site.toml");
        for valid in ["bresilla", "cyberpunk", "cyberpunk-2077", "v2"] {
            assert!(validate_theme_id(path, valid).is_ok());
        }
        for invalid in [
            "",
            "Cyberpunk",
            "cyber punk",
            "-theme",
            "theme-",
            "theme--v2",
        ] {
            assert!(validate_theme_id(path, invalid).is_err());
        }
    }
}

fn normalize_site_url(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.trim() != value {
        return Err("site_url must not contain leading or trailing whitespace".into());
    }
    let authority = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| "site_url must use http:// or https://".to_owned())?
        .trim_end_matches('/');
    if authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
        || authority.chars().any(char::is_whitespace)
        || authority.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | ':' | '[' | ']'))
        })
    {
        return Err(
            "site_url must be an absolute origin without a path, query, credentials, or fragment"
                .into(),
        );
    }
    Ok(value.trim_end_matches('/').to_owned())
}

fn discover_markdown(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), ContentError> {
    let entries = fs::read_dir(directory).map_err(|source| ContentError::Io {
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ContentError::Io {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type().map_err(|source| ContentError::Io {
            path: path.clone(),
            source,
        })?;
        if file_type.is_symlink() {
            let target = fs::canonicalize(&path).map_err(|source| ContentError::Io {
                path: path.clone(),
                source,
            })?;
            if !target.starts_with(root) {
                return Err(ContentError::InvalidPath {
                    path,
                    message: "symlink escapes the content root".into(),
                });
            }
            continue;
        }
        if file_type.is_dir() {
            discover_markdown(root, &path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            let relative = path
                .strip_prefix(root)
                .expect("discovered path must remain under content root")
                .to_owned();
            validate_relative_path(&relative)?;
            output.push(relative);
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), ContentError> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ContentError::InvalidPath {
            path: path.to_owned(),
            message: "path is not relative to the content root".into(),
        });
    }
    if path
        .components()
        .any(|component| component.as_os_str().to_str().is_none())
    {
        return Err(ContentError::InvalidPath {
            path: path.to_owned(),
            message: "non-UTF-8 filenames are not supported".into(),
        });
    }
    Ok(())
}

fn parse_page(
    content_root: &Path,
    relative_path: &Path,
    source: &str,
    shortcode_parser: &ShortcodeParser,
    assets: &mut BTreeMap<String, ContentAsset>,
    default_style: &PageStyle,
    folders: &BTreeMap<String, FolderSpec>,
) -> Result<Page, ContentError> {
    let source_path = slash_path(relative_path);
    let is_section = relative_path
        .file_name()
        .is_some_and(|name| name == "_index.md");
    let top_level_name = relative_path
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .ok_or_else(|| ContentError::InvalidPath {
            path: relative_path.to_owned(),
            message: "Markdown must belong to a top-level content folder".into(),
        })?;
    let folder = folders
        .get(top_level_name)
        .ok_or_else(|| ContentError::InvalidPath {
            path: relative_path.to_owned(),
            message: "Markdown belongs to an undeclared top-level content folder".into(),
        })?;
    let is_presenterm = !is_section && folder.item_type.as_deref() == Some("presentation");
    let (front_matter, body) = if is_presenterm {
        split_yaml_front_matter(source)
    } else {
        split_front_matter(source)
    }
    .map_err(|message| ContentError::FrontMatter {
        path: relative_path.to_owned(),
        message,
    })?;
    let presenterm = if is_presenterm {
        Some(parse_presenterm_front_matter(relative_path, front_matter)?)
    } else {
        None
    };
    let table = if let Some(value) = presenterm.as_ref() {
        presenterm_page_table(content_root, relative_path, value)?
    } else {
        let value =
            front_matter
                .parse::<toml::Value>()
                .map_err(|error| ContentError::FrontMatter {
                    path: relative_path.to_owned(),
                    message: toml_diagnostic(front_matter, &error),
                })?;
        value
            .as_table()
            .cloned()
            .ok_or_else(|| ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: "top-level value must be a TOML table".into(),
            })?
    };
    let table = &table;
    let is_folder_root = is_section && relative_path.components().count() == 2;
    let content_type = string(table, "type").ok_or_else(|| ContentError::FrontMatter {
        path: relative_path.to_owned(),
        message: "every Markdown file requires a type".into(),
    })?;
    if is_folder_root && content_type != folder.folder_type {
        return Err(ContentError::FrontMatter {
            path: relative_path.to_owned(),
            message: format!(
                "folder descriptor type {content_type:?} does not match top-level folder type {:?}",
                folder.folder_type
            ),
        });
    }
    if !is_folder_root && is_section && content_type != "dirstruct" {
        return Err(ContentError::FrontMatter {
            path: relative_path.to_owned(),
            message: "nested _index.md files must use type = \"dirstruct\"".into(),
        });
    }
    if !is_folder_root && is_section {
        let nested_item_type = string(table, "item_type");
        if nested_item_type != folder.item_type.as_deref() {
            return Err(ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: format!(
                    "nested dirstruct item_type {:?} must match top-level folder item_type {:?}",
                    nested_item_type, folder.item_type
                ),
            });
        }
    }
    if !is_section {
        if let Some(expected) = folder.item_type.as_deref() {
            if content_type != expected {
                return Err(ContentError::FrontMatter {
                    path: relative_path.to_owned(),
                    message: format!(
                        "item type {content_type:?} does not match folder item_type {expected:?}"
                    ),
                });
            }
        } else {
            return Err(ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: "singleton content folders may contain only their _index.md page".into(),
            });
        }
    }
    let kind = match content_type {
        "dirstruct" => PageKind::Section,
        "post" => PageKind::Post,
        "cv" => PageKind::Resume,
        "presentation" => PageKind::Talk,
        "home" | "about" | "key" | "quotes" => PageKind::Front,
        other => {
            return Err(ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: format!("unsupported content type {other:?}"),
            });
        }
    };

    let fallback_slug = relative_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("page");
    let slug = string(table, "slug").unwrap_or(fallback_slug).to_owned();
    validate_slug(relative_path, &slug)?;
    let title = string(table, "title")
        .map(|title| {
            if is_presenterm {
                markdown_plain_text(title)
            } else {
                title.to_owned()
            }
        })
        .unwrap_or_else(|| humanize_slug(&slug));
    let has_explicit_title = table.contains_key("title");
    let route = route_for(relative_path, &slug, is_section, folder);
    let status = string(table, "status").unwrap_or("published").to_owned();
    let date = string(table, "date").map(ToOwned::to_owned);
    if let Some(value) = date.as_deref() {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|error| {
            ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: format!(
                    "date {value:?} must use YYYY-MM-DD and name a real calendar date: {error}"
                ),
            }
        })?;
    }
    if table
        .get("style")
        .and_then(toml::Value::as_table)
        .is_some_and(|style| style.contains_key("autoplay"))
    {
        return Err(ContentError::FrontMatter {
            path: relative_path.to_owned(),
            message: "style autoplay is not an authoring option; FAQE controls decorative background playback and honors user motion/data preferences".into(),
        });
    }
    let mut style = presenterm.as_ref().map_or_else(
        || parse_style(table.get("style"), default_style),
        |value| parse_presenterm_style(value, default_style),
    );
    validate_style(relative_path, &style)?;
    if let Some(video) = style.video.take() {
        style.video = Some(resolve_asset_url(
            content_root,
            relative_path,
            &video,
            assets,
        )?);
    }

    validate_active_html(relative_path, body)?;
    validate_supported_html(relative_path, body)?;
    let expanded = shortcode_parser
        .render(body)
        .map_err(|source| ContentError::Shortcode {
            path: relative_path.to_owned(),
            source,
        })?;
    let (body_html, table_of_contents) = markdown_to_html_resolving(&expanded, |url| {
        resolve_asset_url(content_root, relative_path, url, assets)
    })?;
    let body_html = sanitize_legacy_html(&body_html);
    let word_count = visible_word_count(&body_html);
    let thumbnail = string(table, "thumbnail")
        .map(|url| resolve_asset_url(content_root, relative_path, url, assets))
        .transpose()?;
    let reading_minutes = integer(table, "readingtime")
        .or_else(|| integer(table, "reading_time"))
        .filter(|minutes| *minutes > 0)
        .map(|minutes| minutes as usize)
        // Hugo's .ReadingTime uses a 212 words-per-minute pace and rounds up.
        // A copied site can pin the archived value because Hugo versions and
        // shortcode expansion can produce different historical word counts.
        .unwrap_or_else(|| word_count.div_ceil(212).max(1));
    let mut document = document_from_sanitized_html(&body_html);
    derive_missing_image_alternatives(&mut document.nodes);
    normalize_missing_image_alternatives(&mut document.nodes);

    let external_link = string(table, "link").map(ToOwned::to_owned);
    if let Some(url) = external_link.as_deref() {
        validate_public_url(relative_path, url, true)?;
    }
    let credits = string_array(table, "credits");
    for url in &credits {
        validate_public_url(relative_path, url, false)?;
    }
    let mut resume = if kind == PageKind::Resume {
        Some(parse_resume(table, relative_path)?)
    } else {
        None
    };
    if let Some(resume) = &mut resume {
        if !resume.profile.is_empty() {
            resume.profile =
                resolve_asset_url(content_root, relative_path, &resume.profile, assets)?;
        }
        for item in &resume.contact.list {
            validate_public_url(relative_path, &item.url, true)?;
        }
        for project in &resume.projects.list {
            validate_public_url(relative_path, &project.url, true)?;
        }
    }
    let talk = if kind == PageKind::Talk {
        let mut context = TalkParseContext {
            content_root,
            page_path: relative_path,
            assets,
        };
        Some(parse_talk(
            body,
            presenterm
                .as_ref()
                .expect("talk pages require Presenterm front matter"),
            &mut context,
        )?)
    } else {
        None
    };
    let folders = boolean(table, "folders").unwrap_or(false);
    let page_size = match integer(table, "number") {
        Some(value @ 1..=100) => value as usize,
        Some(value) => {
            return Err(ContentError::FrontMatter {
                path: relative_path.to_owned(),
                message: format!("number {value} must be between 1 and 100"),
            });
        }
        None => 6,
    };
    Ok(Page {
        source_path,
        route,
        content_type: content_type.to_owned(),
        kind,
        title,
        has_explicit_title,
        slug,
        status,
        date,
        foot: string(table, "foot").map(ToOwned::to_owned),
        description: string(table, "description").map(|description| {
            if is_presenterm {
                markdown_plain_text(description)
            } else {
                description.to_owned()
            }
        }),
        punchline: string(table, "punchline").map(ToOwned::to_owned),
        tldr: string(table, "tldr").map(ToOwned::to_owned),
        thumbnail,
        external_link,
        part: string(table, "part").map(ToOwned::to_owned),
        credits,
        tags: string_array(table, "tags"),
        categories: string_array(table, "categories"),
        series: string_array(table, "series"),
        style,
        folders,
        page_size,
        reading_minutes,
        table_of_contents,
        document,
        resume,
        talk,
    })
}

fn rewrite_html_asset_attributes<E>(
    input: &str,
    resolve: &mut impl FnMut(&str) -> Result<String, E>,
) -> Result<String, E> {
    let pattern = Regex::new(r#"((?:src|href)\s*=\s*[\"'])([^\"']+)([\"'])"#)
        .expect("valid HTML attribute URL regex");
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    for captures in pattern.captures_iter(input) {
        let url = captures
            .get(2)
            .expect("URL regex always has its URL capture");
        output.push_str(&input[cursor..url.start()]);
        output.push_str(&resolve(url.as_str())?);
        cursor = url.end();
    }
    output.push_str(&input[cursor..]);
    Ok(output)
}

fn resolve_asset_url(
    content_root: &Path,
    page_path: &Path,
    url: &str,
    assets: &mut BTreeMap<String, ContentAsset>,
) -> Result<String, ContentError> {
    let trimmed = url.trim_matches(['<', '>']);
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("file:")
    {
        return Err(ContentError::InvalidPath {
            path: page_path.to_owned(),
            message: format!("unsafe URL scheme in {url:?}"),
        });
    }
    if trimmed.is_empty() {
        return Ok(url.to_owned());
    }
    if trimmed.starts_with(['/', '#', '?'])
        || trimmed.starts_with("//")
        || lower.starts_with("https:")
        || lower.starts_with("http:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
    {
        validate_public_url(page_path, trimmed, true)?;
        return Ok(url.to_owned());
    }
    if trimmed.contains(':') {
        return Err(ContentError::InvalidPath {
            path: page_path.to_owned(),
            message: format!("unsupported URL scheme in {url:?}"),
        });
    }

    let suffix_start = trimmed.find(['?', '#']).unwrap_or(trimmed.len());
    let (reference, suffix) = trimmed.split_at(suffix_start);
    let relative = Path::new(reference);
    let extension = relative
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension.is_empty() || matches!(extension.as_str(), "md" | "html" | "htm") {
        return Ok(url.to_owned());
    }

    let parent = page_path.parent().unwrap_or_else(|| Path::new(""));
    let source = content_root.join(parent).join(relative);
    let canonical = fs::canonicalize(&source).map_err(|source_error| ContentError::Io {
        path: source.clone(),
        source: source_error,
    })?;
    if !canonical.starts_with(content_root) || !canonical.is_file() {
        return Err(ContentError::InvalidPath {
            path: source,
            message: "referenced asset escapes the content root or is not a file".into(),
        });
    }
    let bytes = fs::read(&canonical).map_err(|source| ContentError::Io {
        path: canonical.clone(),
        source,
    })?;
    if bytes.len() as u64 > MAX_CONTENT_ASSET_BYTES {
        return Err(ContentError::InvalidPath {
            path: canonical,
            message: format!(
                "content asset is {} bytes; limit is {MAX_CONTENT_ASSET_BYTES} bytes",
                bytes.len()
            ),
        });
    }
    let digest = Sha256::digest(&bytes);
    let hash = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let stem = relative
        .file_stem()
        .and_then(|value| value.to_str())
        .map(slugify)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "asset".into());
    let extension = extension
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    let output_path = format!("assets/content/{stem}-{hash}.{extension}");
    let source_path = slash_path(
        canonical
            .strip_prefix(content_root)
            .expect("validated content asset remains under content root"),
    );
    insert_content_asset(assets, source_path, output_path.clone(), bytes)?;
    let total_asset_bytes = assets
        .values()
        .map(|asset| asset.bytes.len() as u64)
        .sum::<u64>();
    if total_asset_bytes > MAX_SITE_ASSET_BYTES {
        return Err(ContentError::InvalidPath {
            path: content_root.to_owned(),
            message: format!(
                "referenced content assets total {total_asset_bytes} bytes; limit is {MAX_SITE_ASSET_BYTES} bytes"
            ),
        });
    }
    Ok(format!("/{output_path}{suffix}"))
}

fn insert_content_asset(
    assets: &mut BTreeMap<String, ContentAsset>,
    source_path: String,
    output_path: String,
    bytes: Vec<u8>,
) -> Result<(), ContentError> {
    let key = normalized_collision_key(&output_path);
    if let Some(existing) = assets
        .values()
        .find(|asset| normalized_collision_key(&asset.output_path) == key)
    {
        if existing.bytes != bytes {
            return Err(ContentError::InvalidPath {
                path: PathBuf::from(&source_path),
                message: format!(
                    "fingerprinted asset collision: {source_path:?} and {:?} map to {output_path:?} with different bytes",
                    existing.source_path
                ),
            });
        }
        return Ok(());
    }
    assets.insert(
        output_path.clone(),
        ContentAsset {
            source_path,
            output_path,
            bytes,
        },
    );
    Ok(())
}

fn parse_resume(table: &toml::value::Table, path: &Path) -> Result<ResumeData, ContentError> {
    let value = table
        .get("cv")
        .cloned()
        .ok_or_else(|| ContentError::FrontMatter {
            path: path.to_owned(),
            message: "resume pages require a [cv] table".into(),
        })?;
    let mut resume: ResumeData = value
        .try_into()
        .map_err(|error| ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!("invalid [cv] data: {error}"),
        })?;
    validate_active_html(path, &resume.summary.summary)?;
    resume.summary.summary_document = markdown_fragment(&resume.summary.summary);
    for job in &mut resume.jobs.list {
        validate_active_html(path, &job.details)?;
        job.details_document = markdown_fragment(&job.details);
    }
    validate_active_html(path, &resume.projects.intro)?;
    resume.projects.intro_document = markdown_fragment(&resume.projects.intro);
    for project in &mut resume.projects.list {
        validate_active_html(path, &project.tagline)?;
        project.tagline_document = markdown_fragment(&project.tagline);
    }
    Ok(resume)
}

fn markdown_fragment(markdown: &str) -> Document {
    let (html, _) = markdown_to_html(markdown);
    document_from_sanitized_html(&sanitize_legacy_html(&html))
}

struct TalkParseContext<'a> {
    content_root: &'a Path,
    page_path: &'a Path,
    assets: &'a mut BTreeMap<String, ContentAsset>,
}

fn parse_presenterm_front_matter(
    path: &Path,
    source: &str,
) -> Result<serde_yaml::Value, ContentError> {
    let value = serde_yaml::from_str::<serde_yaml::Value>(source).map_err(|error| {
        ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!("invalid Presenterm YAML: {error}"),
        }
    })?;
    if !value.is_mapping() {
        return Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: "Presenterm front matter must be a YAML mapping".into(),
        });
    }
    Ok(value)
}

fn yaml_value_at<'a>(value: &'a serde_yaml::Value, path: &[&str]) -> Option<&'a serde_yaml::Value> {
    let mut value = value;
    for key in path {
        value = value
            .as_mapping()?
            .get(serde_yaml::Value::String((*key).into()))?;
    }
    Some(value)
}

fn yaml_string<'a>(value: &'a serde_yaml::Value, path: &[&str]) -> Option<&'a str> {
    yaml_value_at(value, path).and_then(serde_yaml::Value::as_str)
}

fn yaml_to_toml(value: &serde_yaml::Value) -> Option<toml::Value> {
    match value {
        serde_yaml::Value::Null => None,
        serde_yaml::Value::Bool(value) => Some(toml::Value::Boolean(*value)),
        serde_yaml::Value::Number(value) => value
            .as_i64()
            .map(toml::Value::Integer)
            .or_else(|| value.as_f64().map(toml::Value::Float)),
        serde_yaml::Value::String(value) => Some(toml::Value::String(value.clone())),
        serde_yaml::Value::Sequence(values) => Some(toml::Value::Array(
            values.iter().filter_map(yaml_to_toml).collect(),
        )),
        serde_yaml::Value::Mapping(values) => {
            let mut table = toml::Table::new();
            for (key, value) in values {
                let key = key.as_str()?;
                if let Some(value) = yaml_to_toml(value) {
                    table.insert(key.to_owned(), value);
                }
            }
            Some(toml::Value::Table(table))
        }
        serde_yaml::Value::Tagged(value) => yaml_to_toml(&value.value),
    }
}

fn presenterm_page_table(
    content_root: &Path,
    path: &Path,
    value: &serde_yaml::Value,
) -> Result<toml::Table, ContentError> {
    let mut table = yaml_to_toml(value)
        .and_then(|value| value.as_table().cloned())
        .ok_or_else(|| ContentError::FrontMatter {
            path: path.to_owned(),
            message: "Presenterm front matter must use string keys".into(),
        })?;
    table.insert("type".into(), toml::Value::String("presentation".into()));
    if !table.contains_key("description") {
        if let Some(subtitle) = yaml_string(value, &["sub_title"]) {
            table.insert(
                "description".into(),
                toml::Value::String(markdown_plain_text(subtitle)),
            );
        }
    }
    let sidecar_path = path.with_extension("faqe.toml");
    let absolute_sidecar = content_root.join(&sidecar_path);
    if absolute_sidecar.exists() {
        let canonical_sidecar =
            fs::canonicalize(&absolute_sidecar).map_err(|source| ContentError::Io {
                path: sidecar_path.clone(),
                source,
            })?;
        if !canonical_sidecar.starts_with(content_root) || !canonical_sidecar.is_file() {
            return Err(ContentError::InvalidPath {
                path: sidecar_path,
                message: "presentation sidecar escapes the content root or is not a file".into(),
            });
        }
        let size = fs::metadata(&canonical_sidecar)
            .map_err(|source| ContentError::Io {
                path: canonical_sidecar.clone(),
                source,
            })?
            .len();
        if size > MAX_SITE_CONFIG_BYTES {
            return Err(ContentError::InvalidPath {
                path: sidecar_path,
                message: format!(
                    "presentation sidecar is {size} bytes; limit is {MAX_SITE_CONFIG_BYTES} bytes"
                ),
            });
        }
        let source = fs::read_to_string(&canonical_sidecar).map_err(|source| ContentError::Io {
            path: sidecar_path.clone(),
            source,
        })?;
        let sidecar = source
            .parse::<toml::Table>()
            .map_err(|error| ContentError::FrontMatter {
                path: sidecar_path.clone(),
                message: toml_diagnostic(&source, &error),
            })?;
        const SIDECAR_KEYS: &[&str] = &[
            "categories",
            "credits",
            "foot",
            "link",
            "part",
            "series",
            "slug",
            "tags",
            "thumbnail",
        ];
        if let Some(key) = sidecar
            .keys()
            .find(|key| !SIDECAR_KEYS.contains(&key.as_str()))
        {
            return Err(ContentError::FrontMatter {
                path: sidecar_path,
                message: format!("unsupported presentation sidecar field {key:?}"),
            });
        }
        table.extend(sidecar);
    }
    Ok(table)
}

fn presenterm_color(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let normalized = value.strip_prefix('#').unwrap_or(value);
    matches!(normalized.len(), 3 | 4 | 6 | 8)
        .then(|| normalized.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .filter(|valid| *valid)
        .map(|_| format!("#{normalized}"))
}

fn parse_presenterm_style(value: &serde_yaml::Value, defaults: &PageStyle) -> PageStyle {
    let mut style = defaults.clone();
    let theme_name = yaml_string(value, &["theme", "name"]);
    if theme_name.is_some_and(|name| name.contains("light") || name.ends_with("-day")) {
        style.theme = Theme::Light;
        std::mem::swap(&mut style.background, &mut style.foreground);
    } else if theme_name.is_some() {
        style.theme = Theme::Dark;
    }
    if let Some(background) = presenterm_color(yaml_string(
        value,
        &["theme", "override", "default", "colors", "background"],
    )) {
        style.background = background;
    }
    if let Some(foreground) = presenterm_color(yaml_string(
        value,
        &["theme", "override", "default", "colors", "foreground"],
    )) {
        style.foreground = foreground;
    }
    let accent = presenterm_color(
        yaml_string(value, &["theme", "override", "palette", "colors", "accent"]).or_else(|| {
            yaml_string(
                value,
                &["theme", "override", "slide_title", "colors", "foreground"],
            )
        }),
    );
    if let Some(accent) = accent {
        style.accent = accent.clone();
        style.chromatic[0] = accent.clone();
        style.chromatic[1] = presenterm_color(yaml_string(
            value,
            &["theme", "override", "palette", "colors", "chromatic"],
        ))
        .or_else(|| chromatic_partner(&accent))
        .unwrap_or_else(|| defaults.chromatic[1].clone());
    }
    style.video = None;
    style
}

fn markdown_plain_text(markdown: &str) -> String {
    let (html, _) = markdown_to_html(markdown);
    Regex::new(r"<[^>]+>")
        .expect("valid HTML tag regex")
        .replace_all(&html, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_talk(
    source: &str,
    front_matter: &serde_yaml::Value,
    context: &mut TalkParseContext<'_>,
) -> Result<TalkDeck, ContentError> {
    let mut slides = Vec::new();
    let mut markdown = String::new();
    let mut fence = None;

    if let Some(slide) = presenterm_intro_slide(front_matter, context)? {
        slides.push(slide);
    }

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some((marker, length)) = fence {
            markdown.push_str(line);
            markdown.push('\n');
            if is_closing_fence(line, marker, length) {
                fence = None;
            }
        } else if let Some(marker) = opening_fence(line) {
            fence = Some(marker);
            markdown.push_str(line);
            markdown.push('\n');
        } else if trimmed == "<!-- end_slide -->" {
            push_presenterm_slide(&mut slides, &mut markdown, context)?;
        } else {
            markdown.push_str(line);
            markdown.push('\n');
        }
    }
    push_presenterm_slide(&mut slides, &mut markdown, context)?;
    Ok(TalkDeck { slides })
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let trimmed = line.trim_start_matches(' ');
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = trimmed.chars().take_while(|value| *value == marker).count();
    (length >= 3).then_some((marker, length))
}

fn is_closing_fence(line: &str, marker: char, opening_length: usize) -> bool {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return false;
    }
    let trimmed = line.trim_start_matches(' ');
    let length = trimmed.chars().take_while(|value| *value == marker).count();
    length >= opening_length && trimmed[length..].trim().is_empty()
}

fn presenterm_intro_slide(
    front_matter: &serde_yaml::Value,
    context: &mut TalkParseContext<'_>,
) -> Result<Option<TalkSlide>, ContentError> {
    let title = yaml_string(front_matter, &["title"]);
    let subtitle = yaml_string(front_matter, &["sub_title"]);
    let author = yaml_string(front_matter, &["author"]);
    let authors = yaml_value_at(front_matter, &["authors"])
        .and_then(serde_yaml::Value::as_sequence)
        .map(|authors| {
            authors
                .iter()
                .filter_map(serde_yaml::Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if title.is_none() && subtitle.is_none() && author.is_none() && authors.is_empty() {
        return Ok(None);
    }
    let mut markdown = String::new();
    if let Some(title) = title {
        markdown.push_str("# ");
        markdown.push_str(title);
        markdown.push_str("\n\n");
    }
    if let Some(subtitle) = subtitle {
        markdown.push_str(subtitle);
        markdown.push_str("\n\n");
    }
    if let Some(author) = author {
        markdown.push_str("**");
        markdown.push_str(author);
        markdown.push_str("**\n");
    } else if !authors.is_empty() {
        markdown.push_str("**");
        markdown.push_str(&authors.join(" · "));
        markdown.push_str("**\n");
    }
    let document = presenterm_markdown_document(&markdown, context, false)?;
    Ok(Some(TalkSlide {
        document,
        attributes: BTreeMap::from([("class".into(), "title-slide".into())]),
        vertical_group: None,
    }))
}

struct PresentermColumns {
    weights: Vec<u32>,
    nodes: Vec<Vec<DocumentNode>>,
}

fn push_presenterm_slide(
    slides: &mut Vec<TalkSlide>,
    source: &mut String,
    context: &mut TalkParseContext<'_>,
) -> Result<(), ContentError> {
    if source.trim().is_empty() {
        source.clear();
        return Ok(());
    }
    let mut roots = Vec::new();
    let mut notes = Vec::new();
    let mut buffer = String::new();
    let mut attributes = BTreeMap::new();
    let mut fragment = false;
    let mut incremental_lists = false;
    let mut columns: Option<PresentermColumns> = None;
    let mut column = None;
    let mut skip = false;
    let mut fence = None;
    let (source_without_notes, multiline_notes) = extract_presenterm_notes(source);
    for note in multiline_notes {
        notes.extend(presenterm_markdown_document(&note, context, false)?.nodes);
    }

    for line in source_without_notes.lines() {
        let trimmed = line.trim();
        if let Some((marker, length)) = fence {
            buffer.push_str(line);
            buffer.push('\n');
            if is_closing_fence(line, marker, length) {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = opening_fence(line) {
            fence = Some(marker);
            buffer.push_str(line);
            buffer.push('\n');
            continue;
        }
        let Some((command, value)) = presenterm_command(trimmed) else {
            buffer.push_str(line);
            buffer.push('\n');
            continue;
        };
        flush_presenterm_segment(
            &mut buffer,
            &mut roots,
            columns.as_mut(),
            column,
            fragment,
            incremental_lists,
            context,
        )?;
        match command {
            "pause" => fragment = true,
            "jump_to_middle" => add_class(&mut attributes, "jump-middle"),
            "no_footer" => add_class(&mut attributes, "no-footer"),
            "skip_slide" => skip = true,
            "alignment" => match value {
                Some("left" | "center" | "right") => {
                    add_class(&mut attributes, &format!("align-{}", value.unwrap()))
                }
                _ => {
                    return Err(presenterm_command_error(
                        context,
                        "alignment must be left, center, or right",
                    ))
                }
            },
            "font_size" => {
                let size = value.and_then(|value| value.parse::<u8>().ok());
                if !matches!(size, Some(1..=7)) {
                    return Err(presenterm_command_error(
                        context,
                        "font_size must be between 1 and 7",
                    ));
                }
                add_class(&mut attributes, &format!("font-size-{}", size.unwrap()));
            }
            "incremental_lists" => {
                incremental_lists = match value {
                    Some("true") => true,
                    Some("false") => false,
                    _ => {
                        return Err(presenterm_command_error(
                            context,
                            "incremental_lists must be true or false",
                        ))
                    }
                };
            }
            "list_item_newlines" => {
                let count = value
                    .and_then(|value| value.parse::<u8>().ok())
                    .unwrap_or(0);
                add_class(&mut attributes, &format!("list-lines-{count}"));
            }
            "new_line" | "newline" => {
                push_presenterm_breaks(1, &mut roots, columns.as_mut(), column, fragment)
            }
            "new_lines" | "newlines" => {
                let count = value
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(1);
                push_presenterm_breaks(
                    count.min(20),
                    &mut roots,
                    columns.as_mut(),
                    column,
                    fragment,
                );
            }
            "column_layout" => {
                finish_presenterm_columns(&mut roots, &mut columns);
                let weights = parse_column_weights(value.unwrap_or_default()).ok_or_else(|| {
                    presenterm_command_error(
                        context,
                        "column_layout must contain positive integer weights",
                    )
                })?;
                let nodes = vec![Vec::new(); weights.len()];
                columns = Some(PresentermColumns { weights, nodes });
                column = None;
            }
            "column" => {
                let index = value.and_then(|value| value.parse::<usize>().ok());
                if columns
                    .as_ref()
                    .is_none_or(|columns| index.is_none_or(|index| index >= columns.nodes.len()))
                {
                    return Err(presenterm_command_error(
                        context,
                        "column must select an index from the active column_layout",
                    ));
                }
                column = index;
            }
            "reset_layout" => {
                finish_presenterm_columns(&mut roots, &mut columns);
                column = None;
            }
            "speaker_note" | "speaker_notes" => {
                if let Some(value) = value {
                    notes.extend(presenterm_markdown_document(value, context, false)?.nodes);
                }
            }
            "include" => {
                let include = value.ok_or_else(|| {
                    presenterm_command_error(context, "include requires a relative Markdown path")
                })?;
                buffer.push_str(&load_presenterm_include(include, context)?);
                buffer.push('\n');
            }
            "comment" | "//" => {}
            _ => {}
        }
    }
    flush_presenterm_segment(
        &mut buffer,
        &mut roots,
        columns.as_mut(),
        column,
        fragment,
        incremental_lists,
        context,
    )?;
    finish_presenterm_columns(&mut roots, &mut columns);
    if !notes.is_empty() {
        roots.push(DocumentNode::Element(ElementNode {
            kind: ElementKind::Quote,
            tag: "aside".into(),
            attributes: BTreeMap::from([("class".into(), "notes".into())]),
            children: notes,
        }));
    }
    source.clear();
    if !skip && !roots.is_empty() {
        slides.push(TalkSlide {
            document: Document { nodes: roots },
            attributes,
            vertical_group: None,
        });
    }
    Ok(())
}

fn extract_presenterm_notes(source: &str) -> (String, Vec<String>) {
    let comments = Regex::new(r"(?s)<!--\s*\r?\n(?P<body>.*?)\r?\n\s*-->")
        .expect("valid multiline comment regex");
    let mut notes = Vec::new();
    let source = comments
        .replace_all(source, |captures: &Captures<'_>| {
            let body = captures.name("body").map_or("", |body| body.as_str());
            let note = serde_yaml::from_str::<serde_yaml::Value>(body)
                .ok()
                .and_then(|value| {
                    yaml_string(&value, &["speaker_note"])
                        .or_else(|| yaml_string(&value, &["speaker_notes"]))
                        .map(ToOwned::to_owned)
                });
            if let Some(note) = note {
                notes.push(note);
                String::new()
            } else {
                captures[0].to_owned()
            }
        })
        .into_owned();
    (source, notes)
}

fn presenterm_command(line: &str) -> Option<(&str, Option<&str>)> {
    let inner = line.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    if inner.is_empty() {
        return None;
    }
    if let Some(comment) = inner.strip_prefix("//") {
        return Some(("//", Some(comment.trim())));
    }
    let (command, value) = inner
        .split_once(':')
        .map_or((inner, None), |(command, value)| {
            (command.trim(), Some(value.trim()))
        });
    Some((command, value))
}

fn presenterm_command_error(context: &TalkParseContext<'_>, message: &str) -> ContentError {
    ContentError::FrontMatter {
        path: context.page_path.to_owned(),
        message: format!("invalid Presenterm command: {message}"),
    }
}

fn add_class(attributes: &mut BTreeMap<String, String>, class: &str) {
    let classes = attributes.entry("class".into()).or_default();
    if !classes.is_empty() {
        classes.push(' ');
    }
    classes.push_str(class);
}

fn flush_presenterm_segment(
    markdown: &mut String,
    roots: &mut Vec<DocumentNode>,
    columns: Option<&mut PresentermColumns>,
    column: Option<usize>,
    fragment: bool,
    incremental_lists: bool,
    context: &mut TalkParseContext<'_>,
) -> Result<(), ContentError> {
    if markdown.trim().is_empty() {
        markdown.clear();
        return Ok(());
    }
    let mut document = presenterm_markdown_document(markdown, context, incremental_lists)?;
    let nodes = if fragment {
        vec![DocumentNode::Element(ElementNode {
            kind: ElementKind::AllowedHtml,
            tag: "div".into(),
            attributes: BTreeMap::from([("class".into(), "fragment".into())]),
            children: document.nodes,
        })]
    } else {
        std::mem::take(&mut document.nodes)
    };
    if let (Some(columns), Some(index)) = (columns, column) {
        columns.nodes[index].extend(nodes);
    } else {
        roots.extend(nodes);
    }
    markdown.clear();
    Ok(())
}

fn presenterm_markdown_document(
    markdown: &str,
    context: &mut TalkParseContext<'_>,
    incremental_lists: bool,
) -> Result<Document, ContentError> {
    validate_active_html(context.page_path, markdown)?;
    validate_supported_html(context.page_path, markdown)?;
    let (html, _) = markdown_to_html_resolving(markdown, |url| {
        resolve_asset_url(context.content_root, context.page_path, url, context.assets)
    })?;
    let mut document = document_from_sanitized_html(&sanitize_legacy_html(&html));
    normalize_presenterm_media(&mut document.nodes);
    if incremental_lists {
        mark_presenterm_list_items(&mut document.nodes);
    }
    Ok(document)
}

fn push_presenterm_breaks(
    count: usize,
    roots: &mut Vec<DocumentNode>,
    columns: Option<&mut PresentermColumns>,
    column: Option<usize>,
    fragment: bool,
) {
    let breaks = (0..count)
        .map(|_| {
            DocumentNode::Element(ElementNode {
                kind: ElementKind::AllowedHtml,
                tag: "br".into(),
                attributes: BTreeMap::new(),
                children: Vec::new(),
            })
        })
        .collect::<Vec<_>>();
    let nodes = if fragment {
        vec![DocumentNode::Element(ElementNode {
            kind: ElementKind::AllowedHtml,
            tag: "div".into(),
            attributes: BTreeMap::from([("class".into(), "fragment".into())]),
            children: breaks,
        })]
    } else {
        breaks
    };
    if let (Some(columns), Some(index)) = (columns, column) {
        columns.nodes[index].extend(nodes);
    } else {
        roots.extend(nodes);
    }
}

fn parse_column_weights(value: &str) -> Option<Vec<u32>> {
    let values = value.trim().strip_prefix('[')?.strip_suffix(']')?;
    let weights = values
        .split(',')
        .map(|value| value.trim().parse::<u32>().ok().filter(|value| *value > 0))
        .collect::<Option<Vec<_>>>()?;
    (!weights.is_empty()).then_some(weights)
}

fn finish_presenterm_columns(
    roots: &mut Vec<DocumentNode>,
    columns: &mut Option<PresentermColumns>,
) {
    let Some(columns) = columns.take() else {
        return;
    };
    let template = columns
        .weights
        .iter()
        .map(|weight| format!("{weight}fr"))
        .collect::<Vec<_>>()
        .join(" ");
    let children = columns
        .nodes
        .into_iter()
        .enumerate()
        .map(|(index, children)| {
            DocumentNode::Element(ElementNode {
                kind: ElementKind::AllowedHtml,
                tag: "div".into(),
                attributes: BTreeMap::from([(
                    "class".into(),
                    format!("presenterm-column presenterm-column-{index}"),
                )]),
                children,
            })
        })
        .collect();
    roots.push(DocumentNode::Element(ElementNode {
        kind: ElementKind::AllowedHtml,
        tag: "div".into(),
        attributes: BTreeMap::from([
            ("class".into(), "presenterm-columns".into()),
            ("style".into(), format!("--faqe-columns:{template}")),
        ]),
        children,
    }));
}

fn load_presenterm_include(
    include: &str,
    context: &TalkParseContext<'_>,
) -> Result<String, ContentError> {
    let relative = Path::new(include);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || relative
            .extension()
            .is_none_or(|extension| extension != "md")
    {
        return Err(presenterm_command_error(
            context,
            "include requires a normalized relative .md path",
        ));
    }
    let source = context
        .content_root
        .join(context.page_path.parent().unwrap_or_else(|| Path::new("")))
        .join(relative);
    fs::read_to_string(&source).map_err(|source_error| ContentError::Io {
        path: source,
        source: source_error,
    })
}

fn mark_presenterm_list_items(nodes: &mut [DocumentNode]) {
    for node in nodes {
        let DocumentNode::Element(element) = node else {
            continue;
        };
        if element.tag == "li" {
            let classes = element.attributes.entry("class".into()).or_default();
            if !classes.split_whitespace().any(|class| class == "fragment") {
                if !classes.is_empty() {
                    classes.push(' ');
                }
                classes.push_str("fragment");
            }
        }
        mark_presenterm_list_items(&mut element.children);
    }
}

fn normalize_presenterm_media(nodes: &mut [DocumentNode]) {
    for node in nodes.iter_mut() {
        let DocumentNode::Element(element) = node else {
            continue;
        };
        normalize_presenterm_media(&mut element.children);
        if element.tag == "img" {
            let alt = element.attributes.get("alt").cloned().unwrap_or_default();
            if let Some(value) = alt.strip_prefix("image:width:") {
                if let Some(width) = value.split_whitespace().next() {
                    if valid_presenterm_image_width(width) {
                        element
                            .attributes
                            .insert("style".into(), format!("--faqe-image-width:{width}"));
                        element.attributes.insert("alt".into(), String::new());
                    }
                }
            } else if let Some(value) = alt.strip_prefix("image:w:") {
                if let Some(width) = value.split_whitespace().next() {
                    if valid_presenterm_image_width(width) {
                        element
                            .attributes
                            .insert("style".into(), format!("--faqe-image-width:{width}"));
                        element.attributes.insert("alt".into(), String::new());
                    }
                }
            }
        }
    }
    for node in nodes.iter_mut() {
        let DocumentNode::Element(element) = node else {
            continue;
        };
        if element.tag != "a" {
            continue;
        }
        let Some(href) = element.attributes.get("href").cloned() else {
            continue;
        };
        let path = href.split(['?', '#']).next().unwrap_or(&href);
        if !matches!(
            Path::new(path).extension().and_then(|value| value.to_str()),
            Some("mp4" | "webm" | "ogg")
        ) {
            continue;
        }
        let caption = document_text(&element.children);
        let media_type = match Path::new(path).extension().and_then(|value| value.to_str()) {
            Some("webm") => "video/webm",
            Some("ogg") => "video/ogg",
            _ => "video/mp4",
        };
        *element = ElementNode {
            kind: ElementKind::AllowedHtml,
            tag: "figure".into(),
            attributes: BTreeMap::from([("class".into(), "mediaframeholder".into())]),
            children: vec![
                DocumentNode::Element(ElementNode {
                    kind: ElementKind::AllowedHtml,
                    tag: "video".into(),
                    attributes: BTreeMap::from([
                        ("aria-label".into(), caption.clone()),
                        ("class".into(), "faqe-embedded-video".into()),
                        ("controls".into(), String::new()),
                        ("playsinline".into(), String::new()),
                        ("preload".into(), "metadata".into()),
                    ]),
                    children: vec![DocumentNode::Element(ElementNode {
                        kind: ElementKind::AllowedHtml,
                        tag: "source".into(),
                        attributes: BTreeMap::from([
                            ("src".into(), href),
                            ("type".into(), media_type.into()),
                        ]),
                        children: Vec::new(),
                    })],
                }),
                DocumentNode::Element(ElementNode {
                    kind: ElementKind::Image,
                    tag: "figcaption".into(),
                    attributes: BTreeMap::from([("class".into(), "imagetextframe".into())]),
                    children: vec![DocumentNode::Text { value: caption }],
                }),
            ],
        };
    }
}

fn valid_presenterm_image_width(value: &str) -> bool {
    value
        .strip_suffix('%')
        .and_then(|value| value.parse::<u8>().ok())
        .is_some_and(|value| value > 0 && value <= 100)
}

fn document_text(nodes: &[DocumentNode]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            DocumentNode::Text { value } => value.clone(),
            DocumentNode::Element(element) => document_text(&element.children),
        })
        .collect::<Vec<_>>()
        .join("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_front_matter(source: &str) -> Result<(&str, &str), String> {
    let mut offset = 0;
    let mut lines = source.split_inclusive('\n');
    let first = lines.next().ok_or("file is empty")?;
    if first.trim_end_matches(['\r', '\n']) != "+++" {
        return Err("expected TOML front matter beginning with +++".into());
    }
    offset += first.len();
    let front_start = offset;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "+++" {
            let front_end = offset;
            offset += line.len();
            return Ok((&source[front_start..front_end], &source[offset..]));
        }
        offset += line.len();
    }
    Err("front matter is missing its closing +++ delimiter".into())
}

fn split_yaml_front_matter(source: &str) -> Result<(&str, &str), String> {
    let mut offset = 0;
    let mut lines = source.split_inclusive('\n');
    let first = lines.next().ok_or("file is empty")?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Err("Presenterm files require YAML front matter beginning with ---".into());
    }
    offset += first.len();
    let front_start = offset;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let front_end = offset;
            offset += line.len();
            return Ok((&source[front_start..front_end], &source[offset..]));
        }
        offset += line.len();
    }
    Err("Presenterm front matter is missing its closing --- delimiter".into())
}

fn toml_diagnostic(source: &str, error: &toml::de::Error) -> String {
    let Some(span) = error.span() else {
        return error.to_string();
    };
    let prefix = &source[..span.start.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count() + 1, |(_, tail)| {
            tail.chars().count() + 1
        });
    format!("line {line}, column {column}: {error}")
}

fn route_for(path: &Path, slug: &str, is_section: bool, folder: &FolderSpec) -> String {
    let relative_parent = path
        .parent()
        .and_then(|parent| parent.strip_prefix(&folder.name).ok())
        .map(slash_path)
        .unwrap_or_default();
    let root = folder.route.trim_matches('/');
    let parent = match (root.is_empty(), relative_parent.is_empty()) {
        (true, _) => relative_parent,
        (false, true) => root.to_owned(),
        (false, false) => format!("{root}/{relative_parent}"),
    };
    if is_section {
        canonical_route(&parent)
    } else if parent.is_empty() {
        canonical_route(&slugify(slug))
    } else {
        canonical_route(&format!("{parent}/{}", slugify(slug)))
    }
}

fn validate_slug(path: &Path, slug: &str) -> Result<(), ContentError> {
    // This typo was part of the published legacy URL. Keep it as an explicit,
    // narrowly scoped compatibility override instead of silently normalizing
    // arbitrary invalid slugs.
    if slash_path(path) == "posts/software/chroot/lxd_lxc.md" && slug == "lxd-lxc:" {
        return Ok(());
    }
    if slug.is_empty()
        || slug == "."
        || slug == ".."
        || slugify(slug) != slug
        || slug.chars().any(char::is_control)
    {
        return Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!(
                "slug {slug:?} must be a lowercase URL segment containing only letters, digits, and single hyphens"
            ),
        });
    }
    Ok(())
}

fn parse_style(value: Option<&toml::Value>, defaults: &PageStyle) -> PageStyle {
    let Some(style) = value.and_then(toml::Value::as_table) else {
        return defaults.clone();
    };
    let theme = match string(style, "theme") {
        Some("light") => Theme::Light,
        _ => Theme::Dark,
    };
    let mut background = string(style, "bg")
        .unwrap_or(&defaults.background)
        .to_owned();
    let mut foreground = string(style, "fg")
        .unwrap_or(&defaults.foreground)
        .to_owned();
    if theme == Theme::Light {
        background = defaults.foreground.clone();
        foreground = defaults.background.clone();
    }
    let explicit_accent = string(style, "accent");
    let accent = explicit_accent.unwrap_or(&defaults.accent).to_owned();
    let chromatic = style
        .get("chromatic")
        .and_then(toml::Value::as_array)
        .filter(|colors| colors.len() == 2)
        .and_then(|colors| {
            Some([
                colors.first()?.as_str()?.to_owned(),
                colors.get(1)?.as_str()?.to_owned(),
            ])
        })
        .or_else(|| {
            explicit_accent.map(|_| {
                [
                    accent.clone(),
                    chromatic_partner(&accent).unwrap_or_else(|| defaults.chromatic[1].clone()),
                ]
            })
        })
        .unwrap_or_else(|| defaults.chromatic.clone());
    PageStyle {
        accent,
        chromatic,
        theme,
        background,
        foreground,
        video: string(style, "video").map(ToOwned::to_owned),
    }
}

fn validate_style(path: &Path, style: &PageStyle) -> Result<(), ContentError> {
    for (name, value) in [
        ("accent", style.accent.as_str()),
        ("chromatic[0]", style.chromatic[0].as_str()),
        ("chromatic[1]", style.chromatic[1].as_str()),
        ("background", style.background.as_str()),
        ("foreground", style.foreground.as_str()),
    ] {
        if !is_css_hex_color(value) {
            return Err(ContentError::FrontMatter {
                path: path.to_owned(),
                message: format!(
                    "style {name} {value:?} must be a #RGB, #RGBA, #RRGGBB, or #RRGGBBAA color"
                ),
            });
        }
    }
    let foreground_contrast =
        contrast_ratio(&style.foreground, &style.background).ok_or_else(|| {
            ContentError::FrontMatter {
                path: path.to_owned(),
                message: "style colors could not be evaluated for contrast".into(),
            }
        })?;
    if foreground_contrast < 4.5 {
        return Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: format!(
                "style foreground {:?} has {foreground_contrast:.2}:1 contrast on background {:?}; normal text requires 4.5:1",
                style.foreground, style.background
            ),
        });
    }
    if accessible_palette(style).is_none() {
        return Err(ContentError::FrontMatter {
            path: path.to_owned(),
            message: "style accent/background combination could not produce accessible UI colors"
                .into(),
        });
    }
    Ok(())
}

fn is_css_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_public_url(path: &Path, value: &str, allow_relative: bool) -> Result<(), ContentError> {
    let invalid = || ContentError::InvalidPath {
        path: path.to_owned(),
        message: format!("unsafe or malformed URL {value:?}"),
    };
    if value.is_empty()
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
    {
        return Err(invalid());
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("mailto:") {
        let address = &value[7..];
        return (address.contains('@')
            && !address.starts_with('@')
            && !address.ends_with('@')
            && !address.chars().any(char::is_whitespace))
        .then_some(())
        .ok_or_else(invalid);
    }
    if lower.starts_with("tel:") {
        let number = &value[4..];
        return (!number.is_empty()
            && number
                .chars()
                .all(|character| character.is_ascii_digit() || "+-().".contains(character)))
        .then_some(())
        .ok_or_else(invalid);
    }
    if let Some(authority) = value.strip_prefix("//") {
        let host = authority.split(['/', '?', '#']).next().unwrap_or_default();
        return (!host.is_empty() && !host.contains('@'))
            .then_some(())
            .ok_or_else(invalid);
    }
    if value.starts_with(['/', '#', '?']) {
        return allow_relative.then_some(()).ok_or_else(invalid);
    }
    if lower.starts_with("https://") || lower.starts_with("http://") {
        let authority = value
            .split_once("://")
            .map(|(_, value)| value)
            .unwrap_or("");
        let host = authority.split(['/', '?', '#']).next().unwrap_or_default();
        return (!host.is_empty() && !host.contains('@'))
            .then_some(())
            .ok_or_else(invalid);
    }
    if value.contains(':') {
        return Err(invalid());
    }
    allow_relative.then_some(()).ok_or_else(invalid)
}

fn markdown_to_html(markdown: &str) -> (String, Vec<TocItem>) {
    markdown_to_html_resolving(markdown, |url| Ok::<_, ContentError>(url.to_owned()))
        .expect("default URL resolver cannot fail")
}

fn markdown_to_html_resolving(
    markdown: &str,
    mut resolve: impl FnMut(&str) -> Result<String, ContentError>,
) -> Result<(String, Vec<TocItem>), ContentError> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    let mut parser = Parser::new_ext(markdown, options).peekable();
    let mut events = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(language) => language.trim().to_owned(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut source = String::new();
                for event in parser.by_ref() {
                    match event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) | Event::Code(text) => source.push_str(&text),
                        Event::SoftBreak | Event::HardBreak => source.push('\n'),
                        _ => {}
                    }
                }
                events.push(Event::Html(CowStr::Boxed(
                    highlighted_code_block(&language, &source).into_boxed_str(),
                )));
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => events.push(Event::Start(Tag::Link {
                link_type,
                dest_url: CowStr::Boxed(resolve(&dest_url)?.into_boxed_str()),
                title,
                id,
            })),
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => events.push(Event::Start(Tag::Image {
                link_type,
                dest_url: CowStr::Boxed(resolve(&dest_url)?.into_boxed_str()),
                title,
                id,
            })),
            Event::Html(value) => events.push(Event::Html(CowStr::Boxed(
                rewrite_html_asset_attributes(&value, &mut resolve)?.into_boxed_str(),
            ))),
            Event::InlineHtml(value) => events.push(Event::InlineHtml(CowStr::Boxed(
                rewrite_html_asset_attributes(&value, &mut resolve)?.into_boxed_str(),
            ))),
            other => events.push(other),
        }
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());

    let heading = Regex::new(r#"(?s)<h([1-6])(?: id="([^"]+)")?>(.*?)</h[1-6]>"#)
        .expect("valid heading regex");
    let tags = Regex::new(r"<[^>]+>").expect("valid tag regex");
    let mut used = BTreeSet::new();
    let mut toc = Vec::new();
    let output = heading
        .replace_all(&output, |captures: &Captures<'_>| {
            let title = decode_basic_entities(&tags.replace_all(&captures[3], ""));
            let requested_id = captures.get(2).map(|value| value.as_str());
            let base = requested_id
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| goldmark_heading_id(&title));
            let mut id = if base.is_empty() {
                "section".into()
            } else {
                base
            };
            let original = id.clone();
            let mut suffix = 1;
            while !used.insert(id.clone()) {
                id = format!("{original}-{suffix}");
                suffix += 1;
            }
            let level = captures[1].parse::<u8>().unwrap_or(1);
            toc.push(TocItem {
                level,
                id: id.clone(),
                title,
            });
            format!("<h{level} id=\"{id}\">{}</h{level}>", &captures[3])
        })
        .into_owned();
    Ok((output, toc))
}

/// Goldmark's default heading IDs lowercase text, turn each whitespace byte
/// into a hyphen, and discard punctuation rather than treating it as a word
/// separator. This distinction preserves published IDs such as `usbip`,
/// `debianubuntu`, and `long-term-goals--skill-track`.
fn goldmark_heading_id(title: &str) -> String {
    title
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|character| {
            if character.is_whitespace() {
                Some('-')
            } else if character.is_alphanumeric() || matches!(character, '-' | '_') {
                Some(character)
            } else {
                None
            }
        })
        .collect()
}

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

fn highlighted_code_block(language: &str, source: &str) -> String {
    let syntax = highlight_language(language);
    let highlighted = syntax
        .and_then(|syntax| tree_sitter_highlight(syntax, source))
        .unwrap_or_else(|| escape_html(source));
    let language = escape_attribute(language);
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
        "<div class=\"highlight ts-highlight\" data-highlighter=\"{engine}\"><div class=\"code-toolbar\" aria-hidden=\"true\"><span class=\"code-language\">{label}</span><span class=\"code-engine\">{engine}</span></div><pre tabindex=\"0\" class=\"chroma\"><code class=\"language-{language}\" data-lang=\"{language}\">{highlighted}</code></pre></div>"
    )
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

fn visible_word_count(html: &str) -> usize {
    let tags = Regex::new(r"(?s)<[^>]*>").expect("valid tag regex");
    tags.replace_all(html, " ").split_whitespace().count()
}

fn sanitize_legacy_html(html: &str) -> String {
    // Start empty so a dependency default can never broaden the content
    // contract. These are the structural elements emitted by Markdown and the
    // supported legacy shortcodes. SVG is limited to the four elements used by
    // the built-in key mark.
    let tags = SAFE_HTML_TAGS.iter().copied().collect::<HashSet<_>>();
    let attributes = ["class", "id", "lang", "role", "style", "title"]
        .into_iter()
        .collect();
    let prefixes = ["aria-", "data-"].into_iter().collect();
    let schemes = ["http", "https", "mailto", "tel"].into_iter().collect();
    let style_properties = [
        "--faqe-align",
        "--faqe-columns",
        "--faqe-image-border",
        "--faqe-image-radius",
        "--faqe-image-width",
        "background-color",
        "border",
        "border-radius",
        "color",
        "cursor",
        "display",
        "height",
        "justify-content",
        "margin",
        "padding",
        "padding-bottom",
        "padding-left",
        "padding-right",
        "padding-top",
        "text-align",
        "width",
    ]
    .into_iter()
    .collect();

    // The only legacy `background` declarations are solid colors. Normalize
    // those to the narrower safe property before CSS parsing; URL-bearing
    // background values are consequently discarded.
    let solid_background = Regex::new(
        r#"(?i)(^|[;"'])\s*background\s*:\s*(#[0-9a-f]{3,8}|var\(--[a-z0-9_-]+\))\s*(;|["']|$)"#,
    )
    .expect("valid solid background regex");
    let html = solid_background.replace_all(html, "${1}background-color:${2}${3}");

    let mut sanitizer = HtmlSanitizer::empty();
    sanitizer
        .tags(tags)
        .generic_attributes(attributes)
        .generic_attribute_prefixes(prefixes)
        .url_schemes(schemes)
        .url_relative(UrlRelative::PassThrough)
        .filter_style_properties(style_properties)
        .add_clean_content_tags(&[
            "script", "style", "iframe", "object", "embed", "template", "noscript",
        ])
        .add_tag_attributes("a", &["download", "href", "hreflang", "target"])
        .add_tag_attributes("blockquote", &["cite"])
        .add_tag_attributes("col", &["span"])
        .add_tag_attributes("colgroup", &["span"])
        .add_tag_attributes("hr", &["align", "size", "width"])
        .add_tag_attributes(
            "img",
            &["align", "alt", "height", "loading", "src", "width"],
        )
        .add_tag_attributes("source", &["src", "type"])
        .add_tag_attributes(
            "video",
            &[
                "controls",
                "height",
                "playsinline",
                "poster",
                "preload",
                "src",
                "width",
            ],
        )
        .add_tag_attributes("input", &["checked", "disabled", "name", "type", "value"])
        .add_tag_attributes("label", &["for"])
        .add_tag_attributes("ol", &["reversed", "start", "type"])
        .add_tag_attributes("q", &["cite"])
        .add_tag_attributes(
            "svg",
            &["fill", "height", "viewBox", "viewbox", "width", "xmlns"],
        )
        .add_tag_attributes(
            "path",
            &["d", "fill", "fill-rule", "stroke", "stroke-width"],
        )
        .add_tag_attributes("td", &["align", "colspan", "headers", "rowspan"])
        .add_tag_attributes("th", &["align", "colspan", "headers", "rowspan", "scope"]);
    sanitizer.clean(&html).to_string()
}

#[derive(Default)]
struct DocumentSink {
    state: RefCell<DocumentBuilder>,
}

#[derive(Default)]
struct DocumentBuilder {
    roots: Vec<DocumentNode>,
    stack: Vec<ElementNode>,
}

impl TokenSink for DocumentSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        let mut state = self.state.borrow_mut();
        match token {
            CharacterTokens(text) => append_document_node(
                &mut state,
                DocumentNode::Text {
                    value: text.to_string(),
                },
            ),
            TagToken(tag) if tag.kind == StartTag => {
                let tag_name = tag.name.to_string();
                let attributes = tag
                    .attrs
                    .into_iter()
                    .map(|attribute| {
                        let mut name = attribute.name.local.to_string();
                        if tag_name == "svg" && name == "viewbox" {
                            name = "viewBox".to_owned();
                        }
                        (name, attribute.value.to_string())
                    })
                    .collect::<BTreeMap<_, _>>();
                let element = ElementNode {
                    kind: classify_element(&tag_name, &attributes),
                    tag: tag_name.clone(),
                    attributes,
                    children: Vec::new(),
                };
                if tag.self_closing || is_void_element(&tag_name) {
                    append_document_node(&mut state, DocumentNode::Element(element));
                } else {
                    state.stack.push(element);
                }
            }
            TagToken(tag) if tag.kind == EndTag => {
                if let Some(element) = state.stack.pop() {
                    debug_assert_eq!(element.tag, tag.name.as_ref());
                    append_document_node(&mut state, DocumentNode::Element(element));
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

fn document_from_sanitized_html(html: &str) -> Document {
    let input = BufferQueue::default();
    input.push_back(StrTendril::from(html));
    let tokenizer = Tokenizer::new(DocumentSink::default(), TokenizerOpts::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    let mut state = tokenizer.sink.state.into_inner();
    while let Some(element) = state.stack.pop() {
        append_document_node(&mut state, DocumentNode::Element(element));
    }
    Document { nodes: state.roots }
}

/// Derive an alternative only when author intent is unambiguous: a figure has
/// exactly one image, that image omitted `alt` entirely, and its sole direct
/// figcaption contains concise plain text. An explicitly empty `alt` is an
/// authored decorative decision and is never replaced. Rich captions are not
/// flattened because links, code, or other elements can carry meaning that is
/// inappropriate in an image alternative.
fn derive_missing_image_alternatives(nodes: &mut [DocumentNode]) {
    for node in nodes {
        let DocumentNode::Element(element) = node else {
            continue;
        };

        if element.tag == "figure" && image_count(&element.children) == 1 {
            let mut captions = element.children.iter().filter_map(|node| match node {
                DocumentNode::Element(caption) if caption.tag == "figcaption" => Some(caption),
                _ => None,
            });
            let caption = captions
                .next()
                .filter(|_| captions.next().is_none())
                .and_then(plain_caption_text);
            if let Some(caption) = caption {
                if let Some(image) = first_image_mut(&mut element.children) {
                    if !image.attributes.contains_key("alt") {
                        image.attributes.insert("alt".to_owned(), caption);
                    }
                }
            }
        }

        derive_missing_image_alternatives(&mut element.children);
    }
}

fn normalize_missing_image_alternatives(nodes: &mut [DocumentNode]) {
    for node in nodes {
        let DocumentNode::Element(element) = node else {
            continue;
        };
        if element.tag == "img" {
            element.attributes.entry("alt".to_owned()).or_default();
        }
        normalize_missing_image_alternatives(&mut element.children);
    }
}

fn image_count(nodes: &[DocumentNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            DocumentNode::Element(element) => {
                usize::from(element.tag == "img") + image_count(&element.children)
            }
            DocumentNode::Text { .. } => 0,
        })
        .sum()
}

fn first_image_mut(nodes: &mut [DocumentNode]) -> Option<&mut ElementNode> {
    for node in nodes {
        let DocumentNode::Element(element) = node else {
            continue;
        };
        if element.tag == "img" {
            return Some(element);
        }
        if let Some(image) = first_image_mut(&mut element.children) {
            return Some(image);
        }
    }
    None
}

fn plain_caption_text(caption: &ElementNode) -> Option<String> {
    let mut source = String::new();
    for child in &caption.children {
        let DocumentNode::Text { value } = child else {
            return None;
        };
        if value
            .chars()
            .any(|character| character.is_control() && !character.is_whitespace())
        {
            return None;
        }
        source.push_str(value);
    }
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty() && normalized.chars().count() <= MAX_DERIVED_ALT_CHARS)
        .then_some(normalized)
}

fn append_document_node(state: &mut DocumentBuilder, node: DocumentNode) {
    let nodes = state
        .stack
        .last_mut()
        .map_or(&mut state.roots, |parent| &mut parent.children);
    if let DocumentNode::Text { value } = &node {
        if let Some(DocumentNode::Text { value: previous }) = nodes.last_mut() {
            previous.push_str(value);
            return;
        }
    }
    nodes.push(node);
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "br" | "col" | "hr" | "img" | "input" | "source" | "wbr"
    )
}

fn classify_element(tag: &str, attributes: &BTreeMap<String, String>) -> ElementKind {
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

fn validate_active_html(path: &Path, source: &str) -> Result<(), ContentError> {
    // The reviewed legacy logo fragment is translated to CSS/Yew behavior and
    // then sanitized. Trust only that exact fixture, never merely a filename or
    // route supplied by an arbitrary content directory.
    if sha256_string(source.as_bytes()) == LEGACY_LOGO_BODY_SHA256 {
        return Ok(());
    }

    let active = Regex::new(
        r#"(?is)<\s*/?\s*(?:script|style|iframe|object|embed|template|noscript|meta|link|base|form)\b|\s+on[a-z][a-z0-9_-]*\s*=|(?:href|src)\s*=\s*["']?\s*(?:javascript|vbscript|data|file)\s*:"#,
    )
    .expect("valid active HTML regex");
    let Some(found) = active.find(source) else {
        return Ok(());
    };
    let prefix = &source[..found.start()];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count() + 1, |(_, tail)| {
            tail.chars().count() + 1
        });
    Err(ContentError::UnsafeHtml {
        path: path.to_owned(),
        line,
        column,
        message:
            "scripts, embedded documents, event handlers, and active URL schemes are not allowed"
                .to_owned(),
    })
}

fn validate_supported_html(path: &Path, source: &str) -> Result<(), ContentError> {
    if sha256_string(source.as_bytes()) == LEGACY_LOGO_BODY_SHA256 {
        return Ok(());
    }
    let tag = Regex::new(r"(?i)<\s*/?\s*([a-z][a-z0-9-]*)").expect("valid HTML tag regex");
    let parser = Parser::new_ext(source, Options::all()).into_offset_iter();
    for (event, range) in parser {
        let value = match event {
            Event::Html(value) | Event::InlineHtml(value) => value,
            _ => continue,
        };
        for capture in tag.captures_iter(&value) {
            let name = capture[1].to_ascii_lowercase();
            if SAFE_HTML_TAGS.contains(&name.as_str()) {
                continue;
            }
            let found = capture.get(0).expect("tag capture has a full match");
            let offset = range.start + found.start();
            let prefix = &source[..offset];
            // Hugo angle-bracket shortcodes contain `< name >` inside `{{...}}`.
            // They are parsed by ShortcodeParser, not authored HTML.
            if prefix.ends_with("{{") {
                continue;
            }
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = prefix
                .rsplit_once('\n')
                .map_or(prefix.chars().count() + 1, |(_, tail)| {
                    tail.chars().count() + 1
                });
            return Err(ContentError::UnsafeHtml {
                path: path.to_owned(),
                line,
                column,
                message: format!(
                    "unsupported HTML element <{name}>; use a supported semantic element or Markdown"
                ),
            });
        }
    }
    Ok(())
}

fn sha256_string(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_internal_routes(
    bundle: &SiteBundle,
    public_files: &[PublicFileSpec],
    aliases: &[AliasSpec],
    warnings: &mut Vec<String>,
) {
    let routes = bundle
        .pages
        .iter()
        .map(|page| page.route.as_str())
        .collect::<BTreeSet<_>>();
    let public_routes = public_files
        .iter()
        .map(|file| format!("/{}", file.output_path))
        .collect::<BTreeSet<_>>();
    let alias_routes = aliases
        .iter()
        .map(|alias| alias.from.as_str())
        .collect::<BTreeSet<_>>();
    for page in &bundle.pages {
        let mut links = Vec::new();
        collect_document_attributes(&page.document.nodes, "href", &mut links);
        for link in links.into_iter().filter(|link| link.starts_with('/')) {
            let direct_path = link.split(['?', '#']).next().unwrap_or(link);
            if public_routes.contains(direct_path) {
                continue;
            }
            let route = canonical_route(link);
            if route != "/"
                && !routes.contains(route.as_str())
                && !alias_routes.contains(route.as_str())
                && !is_builtin_route(&route)
            {
                warnings.push(format!(
                    "{}: internal link {} does not resolve to a content page",
                    page.source_path, route
                ));
            }
        }
    }
}

fn collect_document_attributes<'a>(
    nodes: &'a [DocumentNode],
    name: &str,
    output: &mut Vec<&'a str>,
) {
    for node in nodes {
        if let DocumentNode::Element(element) = node {
            if let Some(value) = element.attributes.get(name) {
                output.push(value);
            }
            collect_document_attributes(&element.children, name, output);
        }
    }
}

fn is_builtin_route(route: &str) -> bool {
    route.starts_with("/assets/content/")
}

fn string<'a>(table: &'a toml::value::Table, key: &str) -> Option<&'a str> {
    table.get(key).and_then(toml::Value::as_str)
}

fn boolean(table: &toml::value::Table, key: &str) -> Option<bool> {
    table.get(key).and_then(toml::Value::as_bool)
}

fn integer(table: &toml::value::Table, key: &str) -> Option<i64> {
    table.get(key).and_then(toml::Value::as_integer)
}

fn string_array(table: &toml::value::Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn humanize_slug(slug: &str) -> String {
    let mut characters = slug.replace(['-', '_'], " ").chars().collect::<Vec<_>>();
    if let Some(first) = characters.first_mut() {
        first.make_ascii_uppercase();
    }
    characters.into_iter().collect()
}

fn decode_basic_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
