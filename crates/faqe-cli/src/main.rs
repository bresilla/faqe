use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use chrono::NaiveDate;
use faqe_content::{
    load_site, normalized_collision_key, AliasSpec, ContentAsset, ContentError, FeedSpec,
    LoadReport, PublicFileSpec,
};
use faqe_model::{accessible_palette, Document, DocumentNode, Page, PageKind, SiteBundle, Theme};
use include_dir::{include_dir, Dir};
use regex::Regex;
use sha2::{Digest, Sha256};

mod theme;

const WEB_JS: &str = include_str!(concat!(env!("FAQE_EMBED_DIR"), "/faqe_web.js"));
const WEB_WASM: &[u8] = include_bytes!(concat!(env!("FAQE_EMBED_DIR"), "/faqe_web_bg.wasm"));
const PROJECT_LICENSE: &str = include_str!(concat!(env!("FAQE_EMBED_DIR"), "/LICENSE"));
const THIRD_PARTY: &str = include_str!(concat!(env!("FAQE_EMBED_DIR"), "/THIRD_PARTY.md"));
static THIRD_PARTY_LICENSES: Dir<'_> = include_dir!("$FAQE_EMBED_DIR/licenses");

include!(concat!(env!("OUT_DIR"), "/embedded_manifest.rs"));

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("faqe: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

#[derive(Debug)]
enum Error {
    Usage(String),
    Content(ContentError),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialization(serde_json::Error),
    InvalidEmbeddedRuntime(String),
    Validation(String),
}

impl Error {
    fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Content(_) => 3,
            Self::Io { .. } | Self::Serialization(_) => 4,
            Self::InvalidEmbeddedRuntime(_) => 5,
            Self::Validation(_) => 3,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}\n\n{}", usage()),
            Self::Content(error) => error.fmt(formatter),
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Serialization(error) => write!(formatter, "could not serialize site: {error}"),
            Self::InvalidEmbeddedRuntime(message) => {
                write!(formatter, "invalid embedded web runtime: {message}")
            }
            Self::Validation(message) => write!(formatter, "invalid site: {message}"),
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), Error> {
    verify_embedded_runtime()?;
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(Error::Usage("missing command".into()));
    };
    match command {
        "build" => {
            let options = BuildOptions::parse(&arguments[1..])?;
            let report = build_site(&options)?;
            print_report(&report, &options.output);
            Ok(())
        }
        "check" => {
            let content = positional_content(&arguments[1..])?;
            let report = check_site(&content)?;
            print_check_report(&report);
            Ok(())
        }
        "serve" => {
            let options = ServeOptions::parse(&arguments[1..])?;
            serve(options)
        }
        "assets" => {
            println!("schema\t{EMBEDDED_SCHEMA_VERSION}");
            println!("build mode\t{EMBEDDED_BUILD_MODE}");
            for (path, length, digest) in EMBEDDED_ENTRIES {
                println!("{path}\t{length} bytes\tsha256:{digest}");
            }
            Ok(())
        }
        "licenses" => {
            println!("{THIRD_PARTY}");
            Ok(())
        }
        "themes" => {
            for id in theme::available() {
                let default = if id == faqe_model::DEFAULT_THEME_ID {
                    "\tdefault"
                } else {
                    ""
                };
                println!("{id}{default}");
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        other => Err(Error::Usage(format!("unknown command {other:?}"))),
    }
}

fn usage() -> &'static str {
    "Usage:\n  faqe build [CONTENT_DIR] [--output DIR] [--base-url PATH]\n  faqe check [CONTENT_DIR]\n  faqe serve [CONTENT_DIR] [--bind IP:PORT] [--no-watch]\n  faqe assets\n  faqe licenses\n  faqe themes"
}

#[derive(Clone, Debug)]
struct BuildOptions {
    content: PathBuf,
    output: PathBuf,
    base_url: String,
}

impl BuildOptions {
    fn parse(arguments: &[String]) -> Result<Self, Error> {
        let mut content = None;
        let mut output = PathBuf::from("dist");
        let mut base_url = "/".to_owned();
        let mut index = 0;
        while index < arguments.len() {
            match arguments[index].as_str() {
                "--output" | "-o" => {
                    index += 1;
                    output = arguments
                        .get(index)
                        .map(PathBuf::from)
                        .ok_or_else(|| Error::Usage("--output requires a directory".into()))?;
                }
                "--base-url" => {
                    index += 1;
                    base_url = arguments
                        .get(index)
                        .cloned()
                        .ok_or_else(|| Error::Usage("--base-url requires a path".into()))?;
                }
                value if value.starts_with('-') => {
                    return Err(Error::Usage(format!("unknown build option {value:?}")));
                }
                value if content.is_none() => content = Some(PathBuf::from(value)),
                value => return Err(Error::Usage(format!("unexpected argument {value:?}"))),
            }
            index += 1;
        }
        Ok(Self {
            content: content.unwrap_or_else(|| PathBuf::from("content")),
            output,
            base_url: normalize_base_url(&base_url)?,
        })
    }
}

#[derive(Debug)]
struct ServeOptions {
    content: PathBuf,
    bind: SocketAddr,
    watch: bool,
}

impl ServeOptions {
    fn parse(arguments: &[String]) -> Result<Self, Error> {
        let mut content = None;
        let mut bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3000);
        let mut watch = true;
        let mut index = 0;
        while index < arguments.len() {
            match arguments[index].as_str() {
                "--bind" => {
                    index += 1;
                    bind = arguments
                        .get(index)
                        .ok_or_else(|| Error::Usage("--bind requires IP:PORT".into()))?
                        .parse()
                        .map_err(|_| Error::Usage("--bind must be an IP:PORT address".into()))?;
                }
                "--no-watch" => watch = false,
                value if value.starts_with('-') => {
                    return Err(Error::Usage(format!("unknown serve option {value:?}")));
                }
                value if content.is_none() => content = Some(PathBuf::from(value)),
                value => return Err(Error::Usage(format!("unexpected argument {value:?}"))),
            }
            index += 1;
        }
        Ok(Self {
            content: content.unwrap_or_else(|| PathBuf::from("content")),
            bind,
            watch,
        })
    }
}

fn positional_content(arguments: &[String]) -> Result<PathBuf, Error> {
    match arguments {
        [] => Ok(PathBuf::from("content")),
        [content] if !content.starts_with('-') => Ok(PathBuf::from(content)),
        _ => Err(Error::Usage(
            "check accepts only one content directory".into(),
        )),
    }
}

fn normalize_base_url(value: &str) -> Result<String, Error> {
    if value.contains(['?', '#'])
        || value.contains("..")
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '/' | '-' | '_' | '.' | '~'))
        })
    {
        return Err(Error::Usage(
            "--base-url must be a root-relative URL path using only URL-safe path characters"
                .into(),
        ));
    }
    if value == "/" || value.is_empty() {
        Ok("/".into())
    } else {
        Ok(format!("/{}/", value.trim_matches('/')))
    }
}

#[derive(Debug)]
struct BuildReport {
    pages: usize,
    routes: usize,
    assets: usize,
    source_bytes: u64,
    output_bytes: u64,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct ValidatedSite {
    loaded: LoadReport,
    public_files: Vec<PublicFile>,
}

fn build_site(options: &BuildOptions) -> Result<BuildReport, Error> {
    validate_build_paths(&options.content, &options.output)?;
    let _lock = OutputLock::acquire(&options.output)?;
    validate_build_paths(&options.content, &options.output)?;
    recover_output_state(&options.output)?;
    let ValidatedSite {
        loaded,
        public_files,
    } = validate_site(&options.content)?;
    let temporary = temporary_output(&options.output);
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(|source| io_error(&temporary, source))?;
    }
    fs::create_dir_all(&temporary).map_err(|source| io_error(&temporary, source))?;

    let generated_assets = match write_site(&temporary, &loaded, &options.base_url, &public_files) {
        Ok(count) => count,
        Err(error) => {
            let _ = fs::remove_dir_all(&temporary);
            return Err(error);
        }
    };

    // The output path may have changed while the site was being rendered. Do
    // not let a newly-created symlink ancestor redirect the final rename.
    if let Err(error) = validate_build_paths(&options.content, &options.output) {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }
    replace_output(&temporary, &options.output)?;
    let output_bytes = directory_size(&options.output)?;
    Ok(BuildReport {
        pages: loaded.markdown_files,
        routes: route_count(&loaded.bundle),
        assets: generated_assets + loaded.assets.len() + public_files.len(),
        source_bytes: loaded.source_bytes,
        output_bytes,
        warnings: loaded.warnings,
    })
}

fn validate_build_paths(content: &Path, output: &Path) -> Result<(), Error> {
    let content = fs::canonicalize(content).map_err(|source| io_error(content, source))?;
    let output_absolute = absolute_without_existing(output)?;
    let output_resolved = resolve_from_existing_ancestor(&output_absolute)?;
    if output_absolute.starts_with(&content) || output_resolved.starts_with(&content) {
        return Err(Error::Usage(
            "output directory must not be inside the content directory".into(),
        ));
    }
    if content.starts_with(&output_absolute) || content.starts_with(&output_resolved) {
        return Err(Error::Usage(
            "output directory must not contain the content directory".into(),
        ));
    }
    let executable =
        env::current_exe().map_err(|source| io_error(Path::new("<executable>"), source))?;
    let executable = fs::canonicalize(&executable).unwrap_or(executable);
    if executable == output_absolute
        || executable == output_resolved
        || executable.starts_with(&output_absolute)
        || executable.starts_with(&output_resolved)
    {
        return Err(Error::Usage(
            "output directory must not overwrite or contain the running faqe executable".into(),
        ));
    }
    Ok(())
}

/// Resolve all symlinks in the existing part of a path while retaining its
/// not-yet-created suffix. `canonicalize(output)` alone cannot protect a path
/// such as `link-to-content/missing-leaf`.
fn resolve_from_existing_ancestor(path: &Path) -> Result<PathBuf, Error> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = ancestor.file_name() else {
                    return Err(io_error(path, source));
                };
                suffix.push(name.to_owned());
                ancestor = ancestor.parent().ok_or_else(|| io_error(path, source))?;
            }
            Err(source) => return Err(io_error(ancestor, source)),
        }
    }
    let mut resolved = fs::canonicalize(ancestor).map_err(|source| io_error(ancestor, source))?;
    for component in suffix.iter().rev() {
        resolved.push(component);
    }
    Ok(normalize_path(&resolved))
}

fn validate_site(content: &Path) -> Result<ValidatedSite, Error> {
    let mut loaded = load_site(content).map_err(Error::Content)?;
    for page in &loaded.bundle.pages {
        if !matches!(page.status.as_str(), "published" | "draft") {
            return Err(Error::Validation(format!(
                "{} has unsupported status {:?}; expected \"published\" or \"draft\"",
                page.source_path, page.status
            )));
        }
    }

    loaded.bundle.pages.retain(Page::is_published);
    loaded.bundle = SiteBundle::new(loaded.bundle.site, loaded.bundle.pages);

    // Asset URLs are rewritten to their fingerprinted output paths during
    // loading. Keeping only paths referenced by the release bundle prevents a
    // draft-only secret image or download from leaking into the output.
    let published_json = serde_json::to_string(&loaded.bundle).map_err(Error::Serialization)?;
    loaded
        .assets
        .retain(|asset| published_json.contains(&asset.output_path));
    validate_output_ownership(&loaded.bundle, &loaded.aliases, &loaded.feeds)?;
    let public_files = load_public_files(
        content,
        &loaded.bundle,
        &loaded.assets,
        &loaded.public_files,
        &loaded.aliases,
        &loaded.feeds,
    )?;
    Ok(ValidatedSite {
        loaded,
        public_files,
    })
}

/// Validate the complete publication model without committing an output tree.
/// Rendering into an isolated scratch directory intentionally exercises the
/// same route shells, embedded assets, bundle references, generated references,
/// public files, sitemap, RSS, and manifest construction as `build`.
fn check_site(content: &Path) -> Result<LoadReport, Error> {
    let ValidatedSite {
        loaded,
        public_files,
    } = validate_site(content)?;
    let workspace = CheckWorkspace::create()?;
    write_site(workspace.path(), &loaded, "/", &public_files)?;
    Ok(loaded)
}

static CHECK_WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct CheckWorkspace {
    path: PathBuf,
}

impl CheckWorkspace {
    fn create() -> Result<Self, Error> {
        for _ in 0..100 {
            let sequence = CHECK_WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path =
                env::temp_dir().join(format!("faqe-check-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io_error(&path, source)),
            }
        }
        Err(Error::Io {
            path: env::temp_dir(),
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not allocate an isolated check workspace",
            ),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CheckWorkspace {
    fn drop(&mut self) {
        if let Err(source) = fs::remove_dir_all(&self.path) {
            if source.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "warning: could not remove check workspace {}: {source}",
                    self.path.display()
                );
            }
        }
    }
}

fn validate_output_ownership(
    bundle: &SiteBundle,
    aliases: &[AliasSpec],
    feeds: &[FeedSpec],
) -> Result<(), Error> {
    let mut owners = BTreeMap::<String, String>::new();
    let mut emitted_owners = BTreeMap::<String, String>::new();
    let mut claim = |route: String, owner: String| -> Result<(), Error> {
        let folded = normalized_collision_key(&route);
        if let Some(first) = owners.insert(folded, owner.clone()) {
            return Err(Error::Validation(format!(
                "output route {route:?} is claimed by both {first} and {owner}"
            )));
        }
        Ok(())
    };
    let mut claim_emitted = |path: String, owner: String| -> Result<(), Error> {
        let folded = normalized_collision_key(path.trim_matches('/'));
        if let Some((claimed, first)) = emitted_owners.iter().find(|(claimed, _)| {
            *claimed == &folded
                || folded.starts_with(&format!("{claimed}/"))
                || claimed.starts_with(&format!("{folded}/"))
        }) {
            return Err(Error::Validation(format!(
                "output path {path:?} owned by {owner} conflicts with {claimed:?} owned by {first}"
            )));
        }
        emitted_owners.insert(folded, owner);
        Ok(())
    };

    if bundle.page("/").is_none() {
        claim("/".into(), "generator:home".into())?;
        claim_emitted("index.html".into(), "generator:home".into())?;
    }
    claim("/404.html".into(), "generator:not-found".into())?;
    claim_emitted("404.html".into(), "generator:not-found".into())?;
    for fixed in ["/assets/", "/index.xml", "/sitemap.xml", "/licenses/"] {
        claim(fixed.into(), format!("generator:{fixed}"))?;
    }
    for fixed in [
        "assets",
        "index.xml",
        "sitemap.xml",
        "licenses",
        "build-manifest.json",
        "LICENSE.txt",
        "THIRD_PARTY.txt",
    ] {
        claim_emitted(fixed.into(), format!("generator:{fixed}"))?;
    }
    for (taxonomy, terms) in taxonomy_routes(bundle) {
        claim(
            format!("/{taxonomy}/"),
            format!("generator:taxonomy:{taxonomy}"),
        )?;
        claim_emitted(
            format!("{taxonomy}/index.html"),
            format!("generator:taxonomy:{taxonomy}"),
        )?;
        for term in terms {
            claim(
                format!("/{taxonomy}/{term}/"),
                format!("generator:taxonomy:{taxonomy}:{term}"),
            )?;
            claim_emitted(
                format!("{taxonomy}/{term}/index.html"),
                format!("generator:taxonomy:{taxonomy}:{term}"),
            )?;
        }
    }
    for page in &bundle.pages {
        claim(page.route.clone(), page.source_path.clone())?;
        for namespace in ["/assets/", "/licenses/"] {
            if page.route.starts_with(namespace) {
                return Err(Error::Validation(format!(
                    "{} uses reserved output namespace {namespace}",
                    page.source_path
                )));
            }
        }
        for reserved_file in [
            "/404.html/",
            "/index.xml/",
            "/sitemap.xml/",
            "/build-manifest.json/",
            "/LICENSE.txt/",
            "/THIRD_PARTY.txt/",
        ] {
            if page.route.eq_ignore_ascii_case(reserved_file) {
                return Err(Error::Validation(format!(
                    "{} conflicts with generated file {}",
                    page.source_path,
                    reserved_file.trim_end_matches('/')
                )));
            }
        }
        claim_emitted(
            format!("{}/index.html", page.route.trim_matches('/')),
            page.source_path.clone(),
        )?;
    }
    for alias in aliases {
        for namespace in ["/assets/", "/licenses/"] {
            if alias.from.starts_with(namespace) {
                return Err(Error::Validation(format!(
                    "alias source {:?} uses reserved output namespace {namespace}",
                    alias.from
                )));
            }
        }
        for reserved in [
            "/404.html/",
            "/index.xml/",
            "/sitemap.xml/",
            "/build-manifest.json/",
            "/LICENSE.txt/",
            "/THIRD_PARTY.txt/",
        ] {
            if alias.from.eq_ignore_ascii_case(reserved) {
                return Err(Error::Validation(format!(
                    "alias source {:?} conflicts with generated file {}",
                    alias.from,
                    reserved.trim_end_matches('/')
                )));
            }
        }
        claim(alias.from.clone(), format!("content-alias:{}", alias.to))?;
        claim_emitted(
            format!("{}/index.html", alias.from.trim_matches('/')),
            format!("content-alias:{}", alias.to),
        )?;
    }
    for feed in feeds.iter().filter(|feed| feed.route != "/index.xml") {
        claim(feed.route.clone(), "content-feed".into())?;
        claim_emitted(
            feed.route.trim_start_matches('/').to_owned(),
            "content-feed".into(),
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct PublicFile {
    output_path: String,
    source_path: String,
    bytes: Vec<u8>,
}

fn load_public_files(
    content: &Path,
    bundle: &SiteBundle,
    content_assets: &[ContentAsset],
    specs: &[PublicFileSpec],
    aliases: &[AliasSpec],
    feeds: &[FeedSpec],
) -> Result<Vec<PublicFile>, Error> {
    const MAX_PUBLIC_FILE_BYTES: u64 = 16 * 1024 * 1024;
    const MAX_PUBLIC_BYTES: u64 = 32 * 1024 * 1024;

    let canonical_content =
        fs::canonicalize(content).map_err(|source| io_error(content, source))?;
    let mut generated = generated_output_paths(bundle, aliases, feeds);
    generated.extend(
        content_assets
            .iter()
            .map(|asset| normalized_collision_key(&asset.output_path)),
    );
    let mut claimed = BTreeSet::new();
    let mut total = 0_u64;
    let mut output = Vec::with_capacity(specs.len());
    for spec in specs {
        let target = &spec.output_path;
        let source = &spec.source_path;
        let folded = normalized_collision_key(target);
        if claimed.contains(&folded)
            || claimed.iter().any(|path: &String| {
                folded.starts_with(&format!("{path}/")) || path.starts_with(&format!("{folded}/"))
            })
        {
            return Err(Error::Validation(format!(
                "public output {target:?} collides with another public output"
            )));
        }
        claimed.insert(folded.clone());
        if generated.contains(&folded)
            || generated.iter().any(|path| {
                folded.starts_with(&format!("{path}/")) || path.starts_with(&format!("{folded}/"))
            })
        {
            return Err(Error::Validation(format!(
                "public output {target:?} collides with generator-owned output"
            )));
        }
        let source_path = content.join(source);
        let canonical_source =
            fs::canonicalize(&source_path).map_err(|error| io_error(&source_path, error))?;
        if !canonical_source.starts_with(&canonical_content) || !canonical_source.is_file() {
            return Err(Error::Validation(format!(
                "public source {source:?} must be a regular file confined to the content directory"
            )));
        }
        let size = fs::metadata(&canonical_source)
            .map_err(|error| io_error(&canonical_source, error))?
            .len();
        if size > MAX_PUBLIC_FILE_BYTES || total.saturating_add(size) > MAX_PUBLIC_BYTES {
            return Err(Error::Validation(format!(
                "public source {source:?} exceeds public-file size limits"
            )));
        }
        total += size;
        output.push(PublicFile {
            output_path: target.clone(),
            source_path: source.clone(),
            bytes: fs::read(&canonical_source)
                .map_err(|error| io_error(&canonical_source, error))?,
        });
    }
    Ok(output)
}

fn generated_output_paths(
    bundle: &SiteBundle,
    aliases: &[AliasSpec],
    feeds: &[FeedSpec],
) -> BTreeSet<String> {
    let mut paths = BTreeSet::from([
        "index.html".into(),
        "404.html".into(),
        "index.xml".into(),
        "sitemap.xml".into(),
        "build-manifest.json".into(),
        "license.txt".into(),
        "third_party.txt".into(),
        "assets".into(),
        "licenses".into(),
    ]);
    for page in &bundle.pages {
        paths.insert(normalized_collision_key(&format!(
            "{}/index.html",
            page.route.trim_matches('/')
        )));
    }
    for (taxonomy, terms) in taxonomy_routes(bundle) {
        paths.insert(format!("{taxonomy}/index.html"));
        for term in terms {
            paths.insert(normalized_collision_key(&format!(
                "{taxonomy}/{term}/index.html"
            )));
        }
    }
    for alias in aliases {
        paths.insert(normalized_collision_key(&format!(
            "{}/index.html",
            alias.from.trim_matches('/')
        )));
    }
    paths.extend(
        feeds
            .iter()
            .map(|feed| normalized_collision_key(feed.route.trim_start_matches('/'))),
    );
    paths
}

fn absolute_without_existing(path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Ok(normalize_path(path));
    }
    let current = env::current_dir().map_err(|source| io_error(Path::new("."), source))?;
    Ok(normalize_path(&current.join(path)))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn temporary_output(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dist");
    output.with_file_name(format!(".{name}.faqe-tmp-{}", std::process::id()))
}

fn replace_output(temporary: &Path, output: &Path) -> Result<(), Error> {
    let backup = backup_output(output);
    if backup.exists() {
        remove_path(&backup)?;
    }
    if output.exists() {
        fs::rename(output, &backup).map_err(|source| io_error(output, source))?;
    }
    if let Err(source) = fs::rename(temporary, output) {
        if backup.exists() {
            let _ = fs::rename(&backup, output);
        }
        return Err(io_error(output, source));
    }
    sync_parent(output)?;
    if backup.exists() {
        remove_path(&backup)?;
    }
    sync_parent(output)?;
    Ok(())
}

fn transaction_sibling(output: &Path, suffix: &str) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dist");
    output.with_file_name(format!(".{name}.{suffix}"))
}

fn backup_output(output: &Path) -> PathBuf {
    transaction_sibling(output, &format!("faqe-backup-{}", std::process::id()))
}

fn lock_output(output: &Path) -> PathBuf {
    transaction_sibling(output, "faqe-lock")
}

struct OutputLock {
    path: PathBuf,
    token: String,
}

impl OutputLock {
    fn acquire(output: &Path) -> Result<Self, Error> {
        let path = lock_output(output);
        let parent = output_parent(&path);
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        let token = format!(
            "{}:{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    if let Err(source) = file
                        .write_all(token.as_bytes())
                        .and_then(|()| file.sync_all())
                    {
                        let _ = fs::remove_file(&path);
                        return Err(io_error(&path, source));
                    }
                    return Ok(Self { path, token });
                }
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                    let owner = fs::read_to_string(&path).unwrap_or_default();
                    let pid = owner.split(':').next().and_then(|pid| pid.parse().ok());
                    let malformed_is_fresh = pid.is_none()
                        && fs::metadata(&path)
                            .and_then(|metadata| metadata.modified())
                            .ok()
                            .and_then(|modified| modified.elapsed().ok())
                            .is_none_or(|age| age < Duration::from_secs(30));
                    if pid.is_some_and(process_is_alive) || malformed_is_fresh {
                        return Err(Error::Validation(format!(
                            "another faqe process holds the output lock {}",
                            path.display()
                        )));
                    }
                    fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
                }
                Err(source) => return Err(io_error(&path, source)),
            }
        }
        Err(Error::Validation(format!(
            "could not acquire output lock {}",
            path.display()
        )))
    }
}

impl Drop for OutputLock {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).ok().as_deref() == Some(self.token.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(all(unix, not(target_os = "linux")))]
fn process_is_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // Signal zero performs a liveness/permission check without delivering a
    // signal. EPERM still means the process exists and its lock is live.
    let status = unsafe { libc::kill(pid, 0) };
    status == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

#[cfg(not(unix))]
fn process_is_alive(pid: u32) -> bool {
    pid == std::process::id()
}

fn recover_output_state(output: &Path) -> Result<(), Error> {
    let parent = output_parent(output);
    let Some(name) = output.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    let temp_prefix = format!(".{name}.faqe-tmp-");
    let backup_prefix = format!(".{name}.faqe-backup-");
    let legacy_backup_prefix = output
        .with_extension("")
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.faqe-backup-"));
    let mut temporary = Vec::new();
    let mut backups = Vec::new();
    for entry in fs::read_dir(parent).map_err(|source| io_error(parent, source))? {
        let entry = entry.map_err(|source| io_error(parent, source))?;
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if entry_name.starts_with(&temp_prefix) {
            temporary.push((modified, entry.path()));
        } else if entry_name.starts_with(&backup_prefix)
            || legacy_backup_prefix
                .as_ref()
                .is_some_and(|prefix| entry_name.starts_with(prefix))
        {
            backups.push((modified, entry.path()));
        }
    }
    temporary.sort();
    backups.sort();

    if output.exists() {
        for (_, path) in temporary.into_iter().chain(backups) {
            remove_path(&path)?;
        }
        return Ok(());
    }
    if let Some((_, backup)) = backups.pop() {
        fs::rename(&backup, output).map_err(|source| io_error(output, source))?;
        for (_, path) in backups.into_iter().chain(temporary) {
            remove_path(&path)?;
        }
        sync_parent(output)?;
        return Ok(());
    }
    if let Some(index) = temporary.iter().rposition(|(_, path)| {
        fs::read(path.join("build-manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some()
    }) {
        let (_, completed) = temporary.remove(index);
        fs::rename(&completed, output).map_err(|source| io_error(output, source))?;
        sync_parent(output)?;
    }
    for (_, path) in temporary {
        remove_path(&path)?;
    }
    Ok(())
}

fn output_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn remove_path(path: &Path) -> Result<(), Error> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|source| io_error(path, source))
    } else {
        fs::remove_file(path).map_err(|source| io_error(path, source))
    }
}

fn sync_parent(path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let parent = output_parent(path);
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io_error(parent, source))?;
    }
    Ok(())
}

fn write_site(
    output: &Path,
    loaded: &LoadReport,
    base_url: &str,
    public_files: &[PublicFile],
) -> Result<usize, Error> {
    let site_json = serde_json::to_vec(&loaded.bundle).map_err(Error::Serialization)?;
    let selected_theme = theme::resolve(&loaded.bundle.site.theme).ok_or_else(|| {
        Error::Validation(format!(
            "theme {:?} is not compiled into this faqe binary; available themes: {}",
            loaded.bundle.site.theme,
            theme::available().collect::<Vec<_>>().join(", ")
        ))
    })?;
    let assets = AssetNames::new(&site_json, selected_theme)?;
    write_file(output.join(&assets.js), WEB_JS.as_bytes())?;
    write_file(output.join(&assets.wasm), WEB_WASM)?;
    write_file(
        output.join(&assets.bootstrap),
        assets.bootstrap_source.as_bytes(),
    )?;
    write_file(output.join(&assets.css), assets.css_source.as_bytes())?;
    for asset in &assets.theme_assets {
        write_file(output.join(&asset.output_path), asset.source.bytes)?;
    }
    write_file(
        output.join(&assets.resume_css),
        assets.resume_css_source.as_bytes(),
    )?;
    write_file(
        output.join(&assets.slide_css),
        assets.slide_css_source.as_bytes(),
    )?;
    write_file(output.join(&assets.site_json), &site_json)?;
    write_file(output.join("LICENSE.txt"), PROJECT_LICENSE.as_bytes())?;
    write_file(output.join("THIRD_PARTY.txt"), THIRD_PARTY.as_bytes())?;
    let licenses_output = output.join("licenses");
    fs::create_dir_all(&licenses_output).map_err(|source| io_error(&licenses_output, source))?;
    THIRD_PARTY_LICENSES
        .extract(&licenses_output)
        .map_err(|source| io_error(output, source))?;
    for asset in &loaded.assets {
        write_file(output.join(&asset.output_path), &asset.bytes)?;
    }
    for public in public_files {
        write_file(output.join(&public.output_path), &public.bytes)?;
    }

    write_route_shell(
        output,
        "/",
        loaded.bundle.page("/"),
        &loaded.bundle,
        base_url,
        &assets,
    )?;
    for page in loaded.bundle.pages.iter().filter(|page| page.route != "/") {
        write_route_shell(
            output,
            &page.route,
            Some(page),
            &loaded.bundle,
            base_url,
            &assets,
        )?;
    }
    for (taxonomy, terms) in taxonomy_routes(&loaded.bundle) {
        for term in terms {
            let route = format!("/{taxonomy}/{term}/");
            write_route_shell(output, &route, None, &loaded.bundle, base_url, &assets)?;
        }
        write_route_shell(
            output,
            &format!("/{taxonomy}/"),
            None,
            &loaded.bundle,
            base_url,
            &assets,
        )?;
    }
    for alias in &loaded.aliases {
        write_alias_redirect(output, alias, &loaded.bundle, base_url)?;
    }
    let not_found = shell_html("/404.html", None, &loaded.bundle, base_url, &assets);
    write_file(output.join("404.html"), not_found.as_bytes())?;
    write_file(
        output.join("sitemap.xml"),
        sitemap(&loaded.bundle, base_url).as_bytes(),
    )?;
    for feed in &loaded.feeds {
        write_file(
            output.join(feed.route.trim_start_matches('/')),
            rss(&loaded.bundle, &loaded.aliases, &feed.route, base_url).as_bytes(),
        )?;
    }
    validate_bundle_references(output, &loaded.bundle)?;
    validate_generated_references(output, base_url)?;
    let manifest = build_manifest(output, loaded, &site_json, &assets, public_files)?;
    write_file(output.join("build-manifest.json"), manifest.as_bytes())?;
    Ok(assets.generated_asset_count())
}

fn write_alias_redirect(
    output: &Path,
    alias: &AliasSpec,
    bundle: &SiteBundle,
    base_url: &str,
) -> Result<(), Error> {
    let root = base_url.trim_end_matches('/');
    let href = format!("{root}{}", alias.to);
    let canonical =
        public_url(&bundle.site.site_url, base_url, &alias.to).unwrap_or_else(|| href.clone());
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"robots\" content=\"noindex\"><meta http-equiv=\"refresh\" content=\"0; url={}\"><link rel=\"canonical\" href=\"{}\"><title>Redirecting · {}</title></head><body><main><h1>Page moved</h1><p><a href=\"{}\">Continue to the canonical page</a>.</p></main></body></html>",
        escape_attribute(&href),
        escape_attribute(&canonical),
        escape_html(&bundle.site.title),
        escape_attribute(&href),
    );
    let relative = alias.from.trim_matches('/');
    write_file(output.join(relative).join("index.html"), html.as_bytes())
}

#[derive(Debug)]
struct AssetNames {
    js: String,
    wasm: String,
    bootstrap: String,
    bootstrap_source: String,
    css: String,
    css_source: String,
    theme_id: &'static str,
    theme_assets: Vec<ThemeAssetName>,
    resume_css: String,
    resume_css_source: String,
    slide_css: String,
    slide_css_source: String,
    site_json: String,
}

#[derive(Debug)]
struct ThemeAssetName {
    output_path: String,
    source: theme::Asset,
}

impl AssetNames {
    fn new(site_json: &[u8], selected_theme: &'static theme::Definition) -> Result<Self, Error> {
        let js = asset_name("faqe", "js", WEB_JS.as_bytes());
        let wasm = asset_name("faqe-bg", "wasm", WEB_WASM);
        let js_file = Path::new(&js).file_name().unwrap().to_string_lossy();
        let wasm_file = Path::new(&wasm).file_name().unwrap().to_string_lossy();
        let bootstrap_source = format!(
            r#"const status=document.getElementById('faqe-bootstrap-status');let attempt=0;
function loading(){{if(!status)return;status.hidden=false;status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent='Loading interactive site…';}}
function failed(error){{console.error('faqe startup failed',error);if(!status)return;status.hidden=false;status.setAttribute('role','alert');status.setAttribute('aria-live','assertive');status.replaceChildren();const message=document.createElement('p');message.textContent='The interactive site failed to start. The readable page remains available.';const retry=document.createElement('button');retry.type='button';retry.textContent='Retry';retry.addEventListener('click',start);status.append(message,retry);retry.focus();}}
async function start(){{loading();const retry=attempt++===0?'':`?faqe-retry=${{attempt}}`;const moduleUrl=new URL('./{js_file}',import.meta.url);const wasmUrl=new URL('./{wasm_file}',import.meta.url);moduleUrl.search=retry;wasmUrl.search=retry;try{{const module=await import(moduleUrl.href);await module.default(wasmUrl);status?.remove();}}catch(error){{failed(error);}}}}
void start();"#
        );
        let mut asset_paths = BTreeMap::new();
        let mut theme_assets = Vec::with_capacity(selected_theme.assets.len());
        for source in selected_theme.assets {
            let output_path = asset_name(source.stem, source.extension, source.bytes);
            let file_name = Path::new(&output_path)
                .file_name()
                .expect("generated asset path has a file name")
                .to_string_lossy()
                .into_owned();
            if asset_paths.insert(source.id.into(), file_name).is_some() {
                return Err(Error::Validation(format!(
                    "theme {:?} defines asset {:?} more than once",
                    selected_theme.id, source.id
                )));
            }
            theme_assets.push(ThemeAssetName {
                output_path,
                source: *source,
            });
        }
        let mut font_faces = String::new();
        for font in selected_theme.fonts {
            let source = asset_paths.get(font.asset_id).ok_or_else(|| {
                Error::Validation(format!(
                    "theme {:?} font {:?} references unknown asset {:?}",
                    selected_theme.id, font.family, font.asset_id
                ))
            })?;
            let unicode_range = font
                .unicode_range
                .map(|range| format!("unicode-range:{range};"))
                .unwrap_or_default();
            let _ = write!(
                font_faces,
                "@font-face{{font-family:'{}';src:url('./{}');font-style:{};font-weight:{};font-display:{};{unicode_range}}}",
                font.family, source, font.style, font.weight, font.display
            );
        }
        let render = |source: &str| {
            theme::render_stylesheet(source, &asset_paths).map_err(|message| {
                Error::Validation(format!("theme {:?}: {message}", selected_theme.id))
            })
        };
        let css_source = format!(
            "{font_faces}{}{}",
            render(selected_theme.styles.base)?,
            render(&(selected_theme.styles.motion)())?
        );
        let resume_css_source = render(selected_theme.styles.resume)?;
        let slide_css_source = render(selected_theme.styles.talk)?;
        Ok(Self {
            js,
            wasm,
            bootstrap: asset_name("bootstrap", "js", bootstrap_source.as_bytes()),
            bootstrap_source,
            css: asset_name(
                &format!("theme-{}", selected_theme.id),
                "css",
                css_source.as_bytes(),
            ),
            css_source,
            theme_id: selected_theme.id,
            theme_assets,
            resume_css: asset_name(
                &format!("theme-{}-resume", selected_theme.id),
                "css",
                resume_css_source.as_bytes(),
            ),
            resume_css_source,
            slide_css: asset_name(
                &format!("theme-{}-talk", selected_theme.id),
                "css",
                slide_css_source.as_bytes(),
            ),
            slide_css_source,
            site_json: asset_name("site", "json", site_json),
        })
    }

    fn generated_asset_count(&self) -> usize {
        7 + self.theme_assets.len()
    }
}

fn asset_name(stem: &str, extension: &str, bytes: &[u8]) -> String {
    format!("assets/{stem}-{}.{}", &sha256_hex(bytes)[..16], extension)
}

fn write_route_shell(
    output: &Path,
    route: &str,
    page: Option<&Page>,
    bundle: &SiteBundle,
    base_url: &str,
    assets: &AssetNames,
) -> Result<(), Error> {
    let relative = route.trim_matches('/');
    let path = if relative.is_empty() {
        output.join("index.html")
    } else {
        output.join(relative).join("index.html")
    };
    write_file(
        path,
        shell_html(route, page, bundle, base_url, assets).as_bytes(),
    )
}

fn shell_html(
    route: &str,
    page: Option<&Page>,
    bundle: &SiteBundle,
    base_url: &str,
    assets: &AssetNames,
) -> String {
    let title = page.map_or_else(
        || static_route_title(route, &bundle.site.title),
        |page| format!("{} · {}", page.title, bundle.site.title),
    );
    let description = page
        .and_then(|page| page.description.as_deref())
        .filter(|description| !description.trim().is_empty())
        .unwrap_or(&bundle.site.description);
    let style = page
        .map(|page| &page.style)
        .unwrap_or(&bundle.site.default_style);
    let palette = accessible_palette(style).expect("validated page styles have accessible colors");
    let scheme = if style.theme == Theme::Light {
        "light"
    } else {
        "dark"
    };
    let root = base_url.trim_end_matches('/');
    let social_image = page
        .and_then(|page| page.thumbnail.as_deref())
        .unwrap_or(&bundle.site.avatar);
    let social_image_alt = page.map_or(bundle.site.title.as_str(), |page| page.title.as_str());
    let og_type = if page.is_some_and(|page| page.kind == PageKind::Post) {
        "article"
    } else {
        "website"
    };
    let article_metadata =
        page.filter(|page| page.kind == PageKind::Post)
            .map_or_else(String::new, |page| {
                let published_time = page.date.as_ref().map_or_else(String::new, |date| {
                    format!(
                        "<meta property=\"article:published_time\" content=\"{}T00:00:00Z\">",
                        escape_attribute(date)
                    )
                });
                let tags = page
                    .tags
                    .iter()
                    .map(|tag| {
                        format!(
                            "<meta property=\"article:tag\" content=\"{}\">",
                            escape_attribute(tag)
                        )
                    })
                    .collect::<String>();
                format!("{published_time}{tags}")
            });
    let canonical = public_url(&bundle.site.site_url, base_url, route);
    let social_image = public_url(&bundle.site.site_url, base_url, social_image);
    let public_metadata = canonical.as_ref().map_or_else(String::new, |canonical| {
        let image = social_image.as_ref().map_or_else(String::new, |image| {
            format!("<meta property=\"og:image\" content=\"{}\"><meta property=\"og:image:alt\" content=\"{}\"><meta name=\"twitter:image\" content=\"{}\"><meta name=\"twitter:image:alt\" content=\"{}\">", escape_attribute(image), escape_attribute(social_image_alt), escape_attribute(image), escape_attribute(social_image_alt))
        });
        format!("<meta property=\"og:url\" content=\"{}\"><link rel=\"canonical\" href=\"{}\">{image}", escape_attribute(canonical), escape_attribute(canonical))
    });
    let keywords = bundle.site.keywords.join(", ");
    let mut extra_css = String::new();
    if page.is_some_and(|page| page.kind == PageKind::Resume) {
        extra_css.push_str(&format!(
            "<link data-faqe-mode=\"resume\" rel=\"stylesheet\" href=\"{root}/{}\">",
            assets.resume_css
        ));
    }
    if page.is_some_and(|page| page.kind == PageKind::Talk) {
        extra_css.push_str(&format!(
            "<link data-faqe-mode=\"talk\" rel=\"stylesheet\" href=\"{root}/{}\">",
            assets.slide_css
        ));
    }
    let fallback_title = page.map_or(bundle.site.title.as_str(), |page| page.title.as_str());
    let fallback = fallback_html(page, bundle, base_url, fallback_title, description);
    format!(
        "<!doctype html><html lang=\"en\" data-faqe-theme=\"{}\" data-faqe-scheme=\"{scheme}\" style=\"--accent-color:{};--chromatic-a:{};--chromatic-b:{};--bg-color:{};--fg-color:{};--glitch-color:{};--interactive-color:{};--accent-text-color:{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; media-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'\"><meta name=\"faqe-base\" content=\"{}\"><meta name=\"faqe-site-url\" content=\"{}\"><meta name=\"faqe-bundle\" content=\"{}\"><meta name=\"faqe-theme\" content=\"{}\"><meta name=\"author\" content=\"{}\"><meta name=\"description\" content=\"{}\"><meta name=\"keywords\" content=\"{}\"><meta name=\"theme-color\" content=\"{}\"><meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"{og_type}\">{article_metadata}<meta name=\"twitter:card\" content=\"summary_large_image\"><meta name=\"twitter:title\" content=\"{}\"><meta name=\"twitter:description\" content=\"{}\">{public_metadata}<title>{}</title><link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{root}/index.xml\"><link rel=\"icon\" href=\"{root}{}\"><link rel=\"stylesheet\" href=\"{root}/{}\">{extra_css}</head><body><div id=\"faqe-bootstrap-status\" class=\"faqe-bootstrap-status\" hidden role=\"status\" aria-live=\"polite\"></div><div id=\"faqe\">{fallback}</div><script type=\"module\" src=\"{root}/{}\"></script></body></html>",
        escape_attribute(&bundle.site.theme),
        escape_attribute(&style.accent),
        escape_attribute(&style.chromatic[0]),
        escape_attribute(&style.chromatic[1]),
        escape_attribute(&style.background),
        escape_attribute(&style.foreground),
        escape_attribute(&style.foreground),
        escape_attribute(&palette.interactive),
        escape_attribute(&palette.accent_text),
        escape_attribute(base_url),
        escape_attribute(&bundle.site.site_url),
        escape_attribute(&assets.site_json),
        escape_attribute(&bundle.site.theme),
        escape_attribute(&bundle.site.author),
        escape_attribute(description),
        escape_attribute(&keywords),
        escape_attribute(&style.background),
        escape_attribute(&title),
        escape_attribute(description),
        escape_attribute(&title),
        escape_attribute(description),
        escape_html(&title),
        escape_attribute(&bundle.site.title),
        escape_attribute(&bundle.site.favicon),
        escape_attribute(&assets.css),
        escape_attribute(&assets.bootstrap),
    )
    .replacen(
        "content=\"width=device-width,initial-scale=1\"",
        "content=\"width=device-width,initial-scale=1,viewport-fit=cover\"",
        1,
    )
}

fn fallback_html(
    page: Option<&Page>,
    bundle: &SiteBundle,
    base_url: &str,
    title: &str,
    description: &str,
) -> String {
    let root = base_url.trim_end_matches('/');
    let mut navigation = String::new();
    for item in &bundle.site.menu {
        if item.url.starts_with('/') && item.url != "/" && bundle.page(&item.url).is_none() {
            continue;
        }
        let href = if item.url.starts_with('/') {
            format!("{root}{}", item.url)
        } else {
            item.url.clone()
        };
        let _ = write!(
            navigation,
            "<a href=\"{}\">{}</a>",
            escape_attribute(&href),
            escape_html(&item.name)
        );
    }
    let mut document = String::new();
    let mut toc = String::new();
    if let Some(page) = page {
        toc = fallback_toc(page);
        render_fallback_nodes(&page.document.nodes, root, &mut document);
    } else {
        let _ = write!(document, "<p>{}</p>", escape_html(description));
    }
    let title_class = page
        .filter(|page| !page.has_explicit_title || page.title.trim().is_empty())
        .map_or("", |_| " class=\"faqe-visually-hidden\"");
    format!(
        "<a href=\"#faqe-main\">Skip to content</a><header><nav aria-label=\"Primary navigation\">{navigation}</nav></header><main id=\"faqe-main\"><h1{title_class}>{}</h1>{toc}{document}</main>",
        escape_html(title)
    )
}

fn fallback_toc(page: &Page) -> String {
    let Some(first) = page.table_of_contents.first() else {
        return String::new();
    };
    let mut output = String::from("<nav id=\"TableOfContents\" aria-label=\"Table of contents\">");
    let mut index = 0;
    render_fallback_toc_level(
        &page.table_of_contents,
        &mut index,
        first.level,
        &mut output,
    );
    output.push_str("</nav>");
    output
}

fn render_fallback_toc_level(
    items: &[faqe_model::TocItem],
    index: &mut usize,
    level: u8,
    output: &mut String,
) {
    output.push_str("<ul>");
    while let Some(item) = items.get(*index).filter(|item| item.level == level) {
        let _ = write!(
            output,
            "<li><a href=\"#{}\">{}</a>",
            escape_attribute(&item.id),
            escape_html(&item.title)
        );
        *index += 1;
        if let Some(next) = items.get(*index).filter(|next| next.level > level) {
            render_fallback_toc_level(items, index, next.level, output);
        }
        output.push_str("</li>");
        if items.get(*index).is_some_and(|next| next.level < level) {
            break;
        }
    }
    output.push_str("</ul>");
}

fn render_fallback_nodes(nodes: &[DocumentNode], root: &str, output: &mut String) {
    for node in nodes {
        match node {
            DocumentNode::Text { value } => output.push_str(&escape_html(value)),
            DocumentNode::Element(element) => {
                let source_heading_level = if element.kind == faqe_model::ElementKind::Heading {
                    element
                        .tag
                        .strip_prefix('h')
                        .and_then(|level| level.parse::<u8>().ok())
                        .filter(|level| (1..=6).contains(level))
                } else {
                    None
                };
                let rendered_tag = source_heading_level
                    .map(|level| format!("h{}", (level + 1).min(6)))
                    .unwrap_or_else(|| element.tag.clone());
                let _ = write!(output, "<{rendered_tag}");
                if let Some(level) = source_heading_level {
                    let _ = write!(
                        output,
                        " class=\"faqe-heading-level-{level}{}\" data-faqe-source-heading-level=\"{level}\"",
                        element
                            .attributes
                            .get("class")
                            .map_or(String::new(), |class| format!(" {}", escape_attribute(class)))
                    );
                }
                for (name, value) in &element.attributes {
                    if source_heading_level.is_some() && name == "class" {
                        continue;
                    }
                    let value = if matches!(name.as_str(), "href" | "src") && value.starts_with('/')
                    {
                        format!("{root}{value}")
                    } else {
                        value.clone()
                    };
                    let _ = write!(output, " {}=\"{}\"", name, escape_attribute(&value));
                }
                output.push('>');
                render_fallback_nodes(&element.children, root, output);
                if !matches!(
                    rendered_tag.as_str(),
                    "br" | "hr" | "img" | "input" | "source" | "wbr"
                ) {
                    let _ = write!(output, "</{rendered_tag}>");
                }
            }
        }
    }
}

fn public_url(site_url: &str, base_url: &str, path: &str) -> Option<String> {
    if site_url.is_empty() {
        return None;
    }
    let base = base_url.trim_matches('/');
    let route = path.trim_start_matches('/');
    let joined = if base.is_empty() || route == base || route.starts_with(&format!("{base}/")) {
        route.to_owned()
    } else if route.is_empty() {
        base.to_owned()
    } else {
        format!("{base}/{route}")
    };
    let trailing_slash = path.ends_with('/') || path.is_empty();
    let mut url = format!("{}/{}", site_url.trim_end_matches('/'), joined);
    if trailing_slash && !url.ends_with('/') {
        url.push('/');
    }
    Some(url)
}

fn static_route_title(route: &str, site_title: &str) -> String {
    if route == "/" {
        return site_title.to_owned();
    }
    if route == "/404.html" {
        return format!("404 · {site_title}");
    }
    let parts = route.trim_matches('/').split('/').collect::<Vec<_>>();
    let label = match parts.as_slice() {
        ["folder"] => "Folder".to_owned(),
        [taxonomy] => humanize_route_part(taxonomy),
        [_, term] => humanize_route_part(term),
        _ => "404".to_owned(),
    };
    format!("{label} · {site_title}")
}

fn humanize_route_part(value: &str) -> String {
    let mut value = value.replace('-', " ");
    if let Some(first) = value.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    value
}

fn taxonomy_routes(bundle: &SiteBundle) -> BTreeMap<&'static str, Vec<String>> {
    BTreeMap::from([
        ("tags", bundle.taxonomies.tags.keys().cloned().collect()),
        (
            "categories",
            bundle.taxonomies.categories.keys().cloned().collect(),
        ),
        ("series", bundle.taxonomies.series.keys().cloned().collect()),
        ("type", bundle.taxonomies.kinds.keys().cloned().collect()),
        // Hugo emitted the configured, but currently empty, `folder` taxonomy
        // root. Keep that public compatibility route without inventing terms.
        ("folder", Vec::new()),
    ])
}

fn sitemap(bundle: &SiteBundle, base_url: &str) -> String {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">");
    let mut routes = bundle
        .pages
        .iter()
        .filter(|page| page.is_published())
        .map(|page| page.route.clone())
        .chain(std::iter::once("/".to_owned()))
        .collect::<BTreeSet<_>>();
    for (taxonomy, terms) in taxonomy_routes(bundle) {
        routes.insert(format!("/{taxonomy}/"));
        routes.extend(terms.into_iter().map(|term| format!("/{taxonomy}/{term}/")));
    }
    let route_dates = sitemap_dates(bundle);
    for route in routes {
        let location = public_url(&bundle.site.site_url, base_url, &route)
            .unwrap_or_else(|| format!("{}{route}", base_url.trim_end_matches('/')));
        let _ = write!(output, "<url><loc>{}</loc>", escape_xml(&location));
        if let Some(date) = route_dates.get(&route) {
            let _ = write!(output, "<lastmod>{}</lastmod>", rss_iso_date(date));
        }
        output.push_str("</url>");
    }
    output.push_str("</urlset>");
    output
}

fn sitemap_dates(bundle: &SiteBundle) -> BTreeMap<String, String> {
    let published = bundle
        .pages
        .iter()
        .filter(|page| page.is_published())
        .collect::<Vec<_>>();
    let page_dates = published
        .iter()
        .filter_map(|page| {
            page.date
                .as_ref()
                .map(|date| (page.route.clone(), date.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut dates = page_dates.clone();
    if let Some(date) = published.iter().filter_map(|page| page.date.as_ref()).max() {
        dates.insert("/".into(), date.clone());
    }
    for section in published
        .iter()
        .filter(|page| page.kind == PageKind::Section)
    {
        if let Some(date) = published
            .iter()
            .filter(|page| page.route != section.route && page.route.starts_with(&section.route))
            .filter_map(|page| page.date.as_ref())
            .max()
        {
            dates.insert(section.route.clone(), date.clone());
        }
    }

    let taxonomies = [
        ("tags", &bundle.taxonomies.tags),
        ("categories", &bundle.taxonomies.categories),
        ("series", &bundle.taxonomies.series),
        ("type", &bundle.taxonomies.kinds),
    ];
    for (name, terms) in taxonomies {
        let mut root_date: Option<&String> = None;
        for (term, routes) in terms {
            if let Some(date) = routes
                .iter()
                .filter_map(|route| page_dates.get(route))
                .max()
            {
                dates.insert(format!("/{name}/{term}/"), date.clone());
                root_date = Some(root_date.map_or(date, |current| current.max(date)));
            }
        }
        if let Some(date) = root_date {
            dates.insert(format!("/{name}/"), date.clone());
        }
    }
    dates
}

fn rss(bundle: &SiteBundle, aliases: &[AliasSpec], feed_route: &str, base_url: &str) -> String {
    let channel_route = feed_route
        .strip_suffix("index.xml")
        .expect("validated feed route");
    let channel_link = public_url(&bundle.site.site_url, base_url, channel_route)
        .unwrap_or_else(|| format!("{}{}", base_url.trim_end_matches('/'), channel_route));
    let feed_url = public_url(&bundle.site.site_url, base_url, feed_route)
        .unwrap_or_else(|| format!("{}{}", base_url.trim_end_matches('/'), feed_route));
    let mut pages = rss_pages(bundle, aliases, feed_route);
    pages.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| left.route.cmp(&right.route))
    });
    let last_build_date = pages
        .iter()
        .filter_map(|page| page.date.as_deref())
        .max()
        .map(rss_rfc2822_date);
    let channel_title = if feed_route == "/index.xml" {
        bundle.site.title.clone()
    } else {
        static_route_title(channel_route, &bundle.site.title)
    };
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\"><channel><title>{}</title><link>{}</link><description>{}</description><language>en</language>",
        escape_xml(&channel_title),
        escape_xml(&channel_link),
        escape_xml(&bundle.site.description)
    );
    if let Some(date) = last_build_date {
        let _ = write!(output, "<lastBuildDate>{}</lastBuildDate>", date);
    }
    let _ = write!(
        output,
        "<atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\"/>",
        escape_attribute(&feed_url)
    );
    for page in pages {
        let link = public_url(&bundle.site.site_url, base_url, &page.route)
            .unwrap_or_else(|| format!("{}{}", base_url.trim_end_matches('/'), page.route));
        let _ = write!(
            output,
            "<item><title>{}</title><link>{}</link><guid isPermaLink=\"true\">{}</guid>",
            escape_xml(&page.title),
            escape_xml(&link),
            escape_xml(&link)
        );
        if let Some(date) = page.date.as_deref() {
            let _ = write!(output, "<pubDate>{}</pubDate>", rss_rfc2822_date(date));
        }
        let _ = write!(
            output,
            "<description>{}</description></item>",
            escape_xml(&rss_summary(page, &bundle.site.description))
        );
    }
    output.push_str("</channel></rss>");
    output
}

fn rss_pages<'a>(bundle: &'a SiteBundle, aliases: &[AliasSpec], feed_route: &str) -> Vec<&'a Page> {
    if feed_route == "/index.xml" {
        return bundle
            .pages
            .iter()
            .filter(|page| page.kind == PageKind::Post && page.is_published())
            .collect();
    }
    let channel_route = feed_route
        .strip_suffix("index.xml")
        .expect("validated feed route");
    let parts = channel_route
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut selected = BTreeSet::<String>::new();
    let taxonomy = match parts.first().copied() {
        Some("tags") => Some(&bundle.taxonomies.tags),
        Some("categories") => Some(&bundle.taxonomies.categories),
        Some("series") => Some(&bundle.taxonomies.series),
        Some("type") => Some(&bundle.taxonomies.kinds),
        _ => None,
    };
    if let Some(taxonomy) = taxonomy {
        if let Some(term) = parts.get(1) {
            selected.extend(taxonomy.get(*term).into_iter().flatten().cloned());
        } else {
            selected.extend(taxonomy.values().flatten().cloned());
        }
    } else {
        selected.extend(
            bundle
                .pages
                .iter()
                .filter(|page| {
                    page.is_published()
                        && page.route.trim_end_matches('/').rsplit_once('/').map_or(
                            "/",
                            |(parent, _)| if parent.is_empty() { "/" } else { parent },
                        ) == channel_route.trim_end_matches('/')
                })
                .map(|page| page.route.clone()),
        );
        selected.extend(aliases.iter().filter_map(|alias| {
            let parent =
                alias
                    .from
                    .trim_end_matches('/')
                    .rsplit_once('/')
                    .map_or(
                        "/",
                        |(parent, _)| if parent.is_empty() { "/" } else { parent },
                    );
            (parent == channel_route.trim_end_matches('/')).then(|| alias.to.clone())
        }));
    }
    selected
        .iter()
        .filter_map(|route| bundle.page(route))
        .filter(|page| page.is_published())
        .collect()
}

fn rss_rfc2822_date(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .expect("content dates are validated before RSS generation")
        .format("%a, %d %b %Y 00:00:00 +0000")
        .to_string()
}

fn rss_iso_date(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .expect("content dates are validated before sitemap generation")
        .format("%Y-%m-%dT00:00:00+00:00")
        .to_string()
}

fn rss_summary(page: &Page, site_description: &str) -> String {
    for candidate in [
        page.description.as_deref(),
        page.tldr.as_deref(),
        page.punchline.as_deref(),
    ] {
        if let Some(candidate) = candidate.filter(|value| !value.trim().is_empty()) {
            return candidate.split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }
    let mut text = String::new();
    collect_document_text(&page.document.nodes, &mut text);
    let words = text.split_whitespace().take(70).collect::<Vec<_>>();
    if words.is_empty() {
        site_description.to_owned()
    } else {
        words.join(" ")
    }
}

fn collect_document_text(nodes: &[DocumentNode], output: &mut String) {
    for node in nodes {
        match node {
            DocumentNode::Text { value } => {
                output.push(' ');
                output.push_str(value);
            }
            DocumentNode::Element(element) => collect_document_text(&element.children, output),
        }
    }
}

fn build_manifest(
    output: &Path,
    loaded: &LoadReport,
    site_json: &[u8],
    names: &AssetNames,
    public_files: &[PublicFile],
) -> Result<String, Error> {
    let mut routes = loaded
        .bundle
        .pages
        .iter()
        .map(|page| (page.route.clone(), page.source_path.clone()))
        .collect::<BTreeMap<_, _>>();
    routes
        .entry("/".to_owned())
        .or_insert_with(|| "generator:home".to_owned());
    for (taxonomy, terms) in taxonomy_routes(&loaded.bundle) {
        routes.insert(
            format!("/{taxonomy}/"),
            format!("generator:taxonomy:{taxonomy}"),
        );
        for term in terms {
            routes.insert(
                format!("/{taxonomy}/{term}/"),
                format!("generator:taxonomy:{taxonomy}:{term}"),
            );
        }
    }
    let mut files = BTreeMap::new();
    collect_output_hashes(output, output, &mut files)?;
    let mut asset_owners = BTreeMap::from([
        (names.js.clone(), "embedded:faqe_web.js".to_owned()),
        (names.wasm.clone(), "embedded:faqe_web_bg.wasm".to_owned()),
        (names.bootstrap.clone(), "generator:bootstrap".to_owned()),
        (names.css.clone(), format!("theme:{}:base", names.theme_id)),
        (
            names.resume_css.clone(),
            format!("theme:{}:resume", names.theme_id),
        ),
        (
            names.slide_css.clone(),
            format!("theme:{}:talk", names.theme_id),
        ),
        (names.site_json.clone(), "generator:site-bundle".to_owned()),
        ("LICENSE.txt".to_owned(), "embedded:LICENSE".to_owned()),
        (
            "THIRD_PARTY.txt".to_owned(),
            "embedded:THIRD_PARTY.md".to_owned(),
        ),
    ]);
    for asset in &names.theme_assets {
        asset_owners.insert(
            asset.output_path.clone(),
            format!("theme:{}:asset:{}", names.theme_id, asset.source.id),
        );
    }
    for (path, _, _) in EMBEDDED_ENTRIES {
        if path.starts_with("licenses/") {
            asset_owners.insert((*path).to_owned(), format!("embedded:{path}"));
        }
    }
    for asset in &loaded.assets {
        asset_owners.insert(asset.output_path.clone(), asset.source_path.clone());
    }
    for public in public_files {
        asset_owners.insert(
            public.output_path.clone(),
            format!("content-public:{}", public.source_path),
        );
    }
    let aliases = loaded
        .aliases
        .iter()
        .map(|alias| (alias.from.clone(), alias.to.clone()))
        .collect::<BTreeMap<_, _>>();
    let feeds = loaded
        .feeds
        .iter()
        .map(|feed| feed.route.clone())
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "generator": env!("CARGO_PKG_VERSION"),
        "schema": loaded.bundle.schema_version,
        "embedded_build_mode": EMBEDDED_BUILD_MODE,
        "input_sha256": sha256_hex(site_json),
        "routes": routes,
        "aliases": aliases,
        "feeds": feeds,
        "asset_owners": asset_owners,
        "files": files,
    }))
    .expect("manifest JSON values are serializable"))
}

fn collect_output_hashes(
    root: &Path,
    directory: &Path,
    output: &mut BTreeMap<String, String>,
) -> Result<(), Error> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| io_error(directory, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| io_error(directory, source))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = entry.metadata().map_err(|source| io_error(&path, source))?;
        if metadata.is_dir() {
            collect_output_hashes(root, &path, output)?;
        } else {
            let bytes = fs::read(&path).map_err(|source| io_error(&path, source))?;
            let relative = path
                .strip_prefix(root)
                .expect("output file remains under output root");
            output.insert(slash_path(relative), sha256_hex(&bytes));
        }
    }
    Ok(())
}

fn validate_generated_references(root: &Path, base_url: &str) -> Result<(), Error> {
    let html_reference = Regex::new(r#"(?:href|src)=\"([^\"]+)\""#).expect("HTML reference regex");
    let css_reference = Regex::new(r#"url\(\s*['\"]?([^'\")]+)"#).expect("CSS reference regex");
    let js_import =
        Regex::new(r#"(?:from\s+|import\(\s*)['\"]([^'\"]+)['\"]"#).expect("JS import regex");
    let js_url = Regex::new(r#"new URL\(\s*['\"]([^'\"]+)['\"]"#).expect("JS URL regex");
    let mut files = Vec::new();
    collect_paths(root, &mut files)?;
    for path in files {
        let extension = path.extension().and_then(|value| value.to_str());
        let generated_asset = path
            .strip_prefix(root)
            .ok()
            .is_some_and(|relative| relative.starts_with("assets"));
        let bytes = match extension {
            Some("html") => fs::read(&path).map_err(|source| io_error(&path, source))?,
            Some("css" | "js") if generated_asset => {
                fs::read(&path).map_err(|source| io_error(&path, source))?
            }
            _ => continue,
        };
        let text = String::from_utf8_lossy(&bytes);
        if extension == Some("html") && !text.contains("name=\"faqe-bundle\"") {
            continue;
        }
        let references = match extension {
            Some("html") => html_reference
                .captures_iter(&text)
                .filter_map(|capture| capture.get(1).map(|value| value.as_str()))
                .collect::<Vec<_>>(),
            Some("css") => css_reference
                .captures_iter(&text)
                .filter_map(|capture| capture.get(1).map(|value| value.as_str()))
                .collect::<Vec<_>>(),
            Some("js") => {
                let mut references = js_import
                    .captures_iter(&text)
                    .filter_map(|capture| capture.get(1).map(|value| value.as_str()))
                    .collect::<Vec<_>>();
                if path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("bootstrap-"))
                {
                    references.extend(
                        js_url
                            .captures_iter(&text)
                            .filter_map(|capture| capture.get(1).map(|value| value.as_str())),
                    );
                }
                references
            }
            _ => Vec::new(),
        };
        for reference in references {
            validate_reference(root, &path, base_url, reference)?;
        }
    }
    Ok(())
}

fn validate_bundle_references(root: &Path, bundle: &SiteBundle) -> Result<(), Error> {
    // The compatibility theme's default menu may point to pages an intentionally
    // tiny parser fixture does not define. Content-owned references below are
    // strict; default theme navigation remains a future configurable-content
    // concern as documented in PLAN.md.
    for reference in [
        bundle.site.favicon.as_str(),
        bundle.site.avatar.as_str(),
        bundle.site.avatar_hover.as_str(),
    ]
    .into_iter()
    .filter(|reference| !reference.is_empty())
    {
        validate_bundle_reference(root, root, "<site metadata>", reference)?;
    }
    for social in &bundle.site.socials {
        validate_bundle_reference(root, root, "<site social link>", &social.url)?;
    }

    for page in &bundle.pages {
        let route_root = root.join(page.route.trim_matches('/'));
        let source = page.source_path.as_str();
        validate_document_references(root, &route_root, source, &page.document)?;
        for reference in page
            .thumbnail
            .iter()
            .chain(page.external_link.iter())
            .chain(page.style.video.iter())
            .chain(page.credits.iter())
        {
            validate_bundle_reference(root, &route_root, source, reference)?;
        }
        if let Some(resume) = &page.resume {
            validate_document_references(
                root,
                &route_root,
                source,
                &resume.summary.summary_document,
            )?;
            validate_document_references(
                root,
                &route_root,
                source,
                &resume.projects.intro_document,
            )?;
            for job in &resume.jobs.list {
                validate_document_references(root, &route_root, source, &job.details_document)?;
            }
            for project in &resume.projects.list {
                validate_bundle_reference(root, &route_root, source, &project.url)?;
                validate_document_references(root, &route_root, source, &project.tagline_document)?;
            }
            for contact in &resume.contact.list {
                validate_bundle_reference(root, &route_root, source, &contact.url)?;
            }
        }
        if let Some(talk) = &page.talk {
            for slide in &talk.slides {
                validate_document_references(root, &route_root, source, &slide.document)?;
            }
        }
    }
    Ok(())
}

fn validate_document_references(
    root: &Path,
    route_root: &Path,
    source: &str,
    document: &Document,
) -> Result<(), Error> {
    fn walk(
        root: &Path,
        route_root: &Path,
        source: &str,
        nodes: &[DocumentNode],
    ) -> Result<(), Error> {
        for node in nodes {
            let DocumentNode::Element(element) = node else {
                continue;
            };
            for attribute in ["href", "src"] {
                if let Some(reference) = element.attributes.get(attribute) {
                    validate_bundle_reference(root, route_root, source, reference)?;
                }
            }
            walk(root, route_root, source, &element.children)?;
        }
        Ok(())
    }
    walk(root, route_root, source, &document.nodes)
}

fn validate_bundle_reference(
    root: &Path,
    route_root: &Path,
    source: &str,
    reference: &str,
) -> Result<(), Error> {
    let target = reference
        .split(['?', '#'])
        .next()
        .unwrap_or(reference)
        .trim();
    if target.is_empty()
        || target.starts_with("//")
        || target
            .split_once(':')
            .is_some_and(|(scheme, _)| scheme.bytes().all(|byte| byte.is_ascii_alphabetic()))
    {
        return Ok(());
    }
    let target = if target.starts_with('/') {
        root.join(target.trim_start_matches('/'))
    } else {
        route_root.join(target)
    };
    let target = normalize_generated_path(root, &target).ok_or_else(|| {
        Error::Content(ContentError::InvalidPath {
            path: PathBuf::from(source),
            message: format!("reference {reference:?} escapes the generated site"),
        })
    })?;
    if target.is_file() || target.join("index.html").is_file() {
        Ok(())
    } else {
        Err(Error::Content(ContentError::InvalidPath {
            path: PathBuf::from(source),
            message: format!(
                "reference {reference:?} does not resolve to an emitted route or asset"
            ),
        }))
    }
}

fn collect_paths(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), Error> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| io_error(directory, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| io_error(directory, source))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, output)?;
        } else {
            output.push(path);
        }
    }
    Ok(())
}

fn validate_reference(
    root: &Path,
    source: &Path,
    base_url: &str,
    reference: &str,
) -> Result<(), Error> {
    let reference = reference
        .split(['?', '#'])
        .next()
        .unwrap_or(reference)
        .trim();
    if reference.is_empty()
        || reference.starts_with('#')
        || reference.starts_with("//")
        || reference
            .split_once(':')
            .is_some_and(|(scheme, _)| scheme.bytes().all(|byte| byte.is_ascii_alphabetic()))
    {
        return Ok(());
    }
    let target = if reference.starts_with('/') {
        let base = base_url.trim_end_matches('/');
        if base.is_empty() {
            root.join(reference.trim_start_matches('/'))
        } else if let Some(relative) = reference.strip_prefix(base) {
            root.join(relative.trim_start_matches('/'))
        } else {
            return Err(Error::InvalidEmbeddedRuntime(format!(
                "generated file {} contains root URL {reference:?} outside base URL {base_url:?}",
                source.display()
            )));
        }
    } else {
        source.parent().unwrap_or(root).join(reference)
    };
    let target = normalize_generated_path(root, &target).ok_or_else(|| {
        Error::InvalidEmbeddedRuntime(format!(
            "generated file {} contains escaping reference {reference:?}",
            source.display()
        ))
    })?;
    if target.is_file() || target.join("index.html").is_file() {
        Ok(())
    } else {
        Err(Error::InvalidEmbeddedRuntime(format!(
            "generated file {} contains unresolved reference {reference:?}",
            source.display()
        )))
    }
}

fn normalize_generated_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(root).ok()?;
    let mut normalized = root.to_owned();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value),
            Component::ParentDir => {
                if normalized == root {
                    return None;
                }
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn write_file(path: PathBuf, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    fs::write(&path, bytes).map_err(|source| io_error(&path, source))
}

fn route_count(bundle: &SiteBundle) -> usize {
    bundle.pages.len()
        + taxonomy_routes(bundle)
            .values()
            .map(|terms| 1 + terms.len())
            .sum::<usize>()
}

fn directory_size(path: &Path) -> Result<u64, Error> {
    let mut size = 0;
    for entry in fs::read_dir(path).map_err(|source| io_error(path, source))? {
        let entry = entry.map_err(|source| io_error(path, source))?;
        let metadata = entry
            .metadata()
            .map_err(|source| io_error(&entry.path(), source))?;
        if metadata.is_dir() {
            size += directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

fn print_report(report: &BuildReport, output: &Path) {
    println!("generated {} Markdown pages", report.pages);
    println!("generated {} routes", report.routes);
    println!("emitted {} embedded assets", report.assets);
    println!("source: {} bytes", report.source_bytes);
    println!(
        "output: {} bytes at {}",
        report.output_bytes,
        output.display()
    );
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
}

fn print_check_report(report: &LoadReport) {
    println!(
        "checked {} Markdown files ({} bytes): {} routes, {} warnings",
        report.markdown_files,
        report.source_bytes,
        report.bundle.pages.len(),
        report.warnings.len()
    );
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
}

fn serve(options: ServeOptions) -> Result<(), Error> {
    let output = env::temp_dir().join(format!("faqe-preview-{}", std::process::id()));
    let build_options = BuildOptions {
        content: options.content.clone(),
        output: output.clone(),
        base_url: "/".into(),
    };
    let report = build_site(&build_options)?;
    print_report(&report, &output);

    if options.watch {
        let content = options.content.clone();
        let output = output.clone();
        thread::spawn(move || watch_content(content, output));
    }

    let listener = TcpListener::bind(options.bind).map_err(|source| io_error(&output, source))?;
    let address = listener
        .local_addr()
        .map_err(|source| io_error(&output, source))?;
    println!("serving http://{address}");
    let (requests, workers) = mpsc::sync_channel::<TcpStream>(64);
    let workers = Arc::new(Mutex::new(workers));
    for _ in 0..8 {
        let workers = Arc::clone(&workers);
        let output = output.clone();
        thread::spawn(move || loop {
            let stream = {
                let receiver = workers.lock().expect("preview worker receiver poisoned");
                receiver.recv()
            };
            let Ok(stream) = stream else { break };
            if let Err(error) = serve_request(stream, &output) {
                if !matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::WouldBlock
                ) {
                    eprintln!("faqe: preview request failed: {error}");
                }
            }
        });
    }
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if requests.send(stream).is_err() {
                    break;
                }
            }
            Err(error) => eprintln!("faqe: preview connection failed: {error}"),
        }
    }
    Ok(())
}

fn watch_content(content: PathBuf, output: PathBuf) {
    let mut previous = content_snapshot(&content).ok();
    loop {
        thread::sleep(Duration::from_millis(500));
        let observed = content_snapshot(&content);
        let Ok(observed) = observed else {
            eprintln!("faqe: content watch scan failed; serving last good build");
            continue;
        };
        if Some(observed) != previous {
            thread::sleep(Duration::from_millis(150));
            let current = content_snapshot(&content).unwrap_or(observed);
            let options = BuildOptions {
                content: content.clone(),
                output: output.clone(),
                base_url: "/".into(),
            };
            match build_site(&options) {
                Ok(report) => {
                    println!("rebuilt {} pages", report.pages);
                }
                Err(error) => eprintln!("faqe: rebuild rejected; serving last good build: {error}"),
            }
            previous = Some(current);
        }
    }
}

fn content_snapshot(path: &Path) -> Result<[u8; 32], std::io::Error> {
    fn hash_path(path: &Path, root: &Path, digest: &mut Sha256) -> Result<(), std::io::Error> {
        let metadata = fs::symlink_metadata(path)?;
        let relative = path.strip_prefix(root).unwrap_or(path);
        digest.update(slash_path(relative).as_bytes());
        if metadata.is_symlink() {
            digest.update(b"symlink\0");
            digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
            if let Ok(target) = fs::canonicalize(path) {
                if target.starts_with(root) && target.is_file() {
                    digest.update(b"internal-target\0");
                    digest.update(fs::read(target)?);
                }
            }
        } else if metadata.is_dir() {
            digest.update(b"directory\0");
            let mut children = fs::read_dir(path)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?;
            children.sort();
            for child in children {
                hash_path(&child, root, digest)?;
            }
        } else if metadata.is_file() {
            digest.update(b"file\0");
            digest.update(metadata.len().to_le_bytes());
            digest.update(fs::read(path)?);
        }
        Ok(())
    }

    let mut digest = Sha256::new();
    hash_path(path, path, &mut digest)?;
    Ok(digest.finalize().into())
}

fn serve_request(mut stream: TcpStream, root: &Path) -> Result<(), std::io::Error> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = read_http_head(&mut stream)?;
    let request = String::from_utf8_lossy(&request);
    let range = request.lines().find_map(|line| {
        line.split_once(':')
            .and_then(|(name, value)| name.eq_ignore_ascii_case("range").then(|| value.trim()))
    });
    let if_none_match = request.lines().find_map(|line| {
        line.split_once(':').and_then(|(name, value)| {
            name.eq_ignore_ascii_case("if-none-match")
                .then(|| value.trim())
        })
    });
    let first = request.lines().next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let raw_path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/");
    if method != "GET" && method != "HEAD" {
        return respond(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
            method == "HEAD",
            "no-store",
            None,
        );
    }
    let Some(decoded_path) = percent_decode_path(raw_path) else {
        return respond(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad path",
            method == "HEAD",
            "no-store",
            None,
        );
    };
    let relative = decoded_path.trim_start_matches('/');
    if relative.contains('\\') || relative.split('/').any(|part| part == "..") {
        return respond(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad path",
            method == "HEAD",
            "no-store",
            None,
        );
    }
    let mut path = root.join(relative);
    if path.is_dir() {
        path.push("index.html");
    }
    let (status, bytes, mime) = match fs::read(&path) {
        Ok(bytes) => (200, bytes, mime_type(&path)),
        Err(_) => {
            let fallback = root.join("404.html");
            (404, fs::read(fallback)?, "text/html; charset=utf-8")
        }
    };
    let cache = if status == 200 && is_content_addressed(relative) {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let etag = (status == 200).then(|| entity_tag(&bytes));
    if status == 200 {
        if if_none_match.is_some_and(|value| if_none_match_matches(value, etag.as_deref().unwrap()))
        {
            return respond_not_modified(&mut stream, cache, etag.as_deref().unwrap());
        }
        if let Some((start, end)) = range.and_then(|value| parse_byte_range(value, bytes.len())) {
            return respond_range(
                &mut stream,
                mime,
                &bytes[start..=end],
                method == "HEAD",
                cache,
                start,
                end,
                bytes.len(),
                etag.as_deref(),
            );
        }
        if range.is_some() {
            return respond_unsatisfiable_range(
                &mut stream,
                mime,
                method == "HEAD",
                cache,
                bytes.len(),
                etag.as_deref(),
            );
        }
    }
    respond(
        &mut stream,
        status,
        mime,
        &bytes,
        method == "HEAD",
        cache,
        etag.as_deref(),
    )
}

fn read_http_head(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    const MAX_HEADER_BYTES: usize = 64 * 1024;
    let mut request = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    loop {
        let size = stream.read(&mut chunk)?;
        if size == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed before complete HTTP headers",
            ));
        }
        request.extend_from_slice(&chunk[..size]);
        if request.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP request headers exceed 64 KiB",
            ));
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
    }
}

fn parse_byte_range(value: &str, length: usize) -> Option<(usize, usize)> {
    let value = value.strip_prefix("bytes=")?;
    let (start, end) = value.split_once('-')?;
    if value.contains(',') || length == 0 {
        return None;
    }
    if start.is_empty() {
        let suffix = end.parse::<usize>().ok()?.min(length);
        return (suffix > 0).then_some((length - suffix, length - 1));
    }
    let start = start.parse::<usize>().ok()?;
    if start >= length {
        return None;
    }
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<usize>().ok()?.min(length - 1)
    };
    (start <= end).then_some((start, end))
}

fn entity_tag(bytes: &[u8]) -> String {
    format!("\"{}\"", sha256_hex(bytes))
}

fn if_none_match_matches(value: &str, etag: &str) -> bool {
    value.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*"
            || candidate
                .strip_prefix("W/")
                .or_else(|| candidate.strip_prefix("w/"))
                .unwrap_or(candidate)
                == etag
    })
}

fn is_content_addressed(path: &str) -> bool {
    if !path.starts_with("assets/") {
        return false;
    }
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit_once('-').map(|(_, digest)| digest))
        .is_some_and(|digest| {
            digest.len() == 16 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn percent_decode_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            let byte = high * 16 + low;
            if byte == 0 {
                return None;
            }
            output.push(byte);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    mime: &str,
    body: &[u8],
    head_only: bool,
    cache_control: &str,
    etag: Option<&str>,
) -> Result<(), std::io::Error> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let etag_header = etag.map_or_else(String::new, |etag| format!("ETag: {etag}\r\n"));
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nCache-Control: {cache_control}\r\n{etag_header}Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; media-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'\r\nReferrer-Policy: strict-origin-when-cross-origin\r\nPermissions-Policy: camera=(), microphone=(), geolocation=()\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    Ok(())
}

fn respond_not_modified(
    stream: &mut TcpStream,
    cache_control: &str,
    etag: &str,
) -> Result<(), std::io::Error> {
    write!(
        stream,
        "HTTP/1.1 304 Not Modified\r\nCache-Control: {cache_control}\r\nETag: {etag}\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n"
    )
}

#[allow(clippy::too_many_arguments)]
fn respond_range(
    stream: &mut TcpStream,
    mime: &str,
    body: &[u8],
    head_only: bool,
    cache_control: &str,
    start: usize,
    end: usize,
    total: usize,
    etag: Option<&str>,
) -> Result<(), std::io::Error> {
    let etag_header = etag.map_or_else(String::new, |etag| format!("ETag: {etag}\r\n"));
    write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{total}\r\nAccept-Ranges: bytes\r\nCache-Control: {cache_control}\r\n{etag_header}Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; media-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'\r\nReferrer-Policy: strict-origin-when-cross-origin\r\nPermissions-Policy: camera=(), microphone=(), geolocation=()\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    Ok(())
}

fn respond_unsatisfiable_range(
    stream: &mut TcpStream,
    mime: &str,
    head_only: bool,
    cache_control: &str,
    total: usize,
    etag: Option<&str>,
) -> Result<(), std::io::Error> {
    let body = b"range not satisfiable";
    let etag_header = etag.map_or_else(String::new, |etag| format!("ETag: {etag}\r\n"));
    write!(
        stream,
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nContent-Range: bytes */{total}\r\nAccept-Ranges: bytes\r\nCache-Control: {cache_control}\r\n{etag_header}X-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    Ok(())
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("ogg" | "ogv") => "video/ogg",
        Some("mp3") => "audio/mpeg",
        Some("oga") => "audio/ogg",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn verify_embedded_runtime() -> Result<(), Error> {
    if EMBEDDED_SCHEMA_VERSION != faqe_model::SITE_SCHEMA_VERSION {
        return Err(Error::InvalidEmbeddedRuntime(format!(
            "embedded schema {EMBEDDED_SCHEMA_VERSION} does not match runtime schema {}",
            faqe_model::SITE_SCHEMA_VERSION
        )));
    }
    if WEB_JS.len() < 100 {
        return Err(Error::InvalidEmbeddedRuntime(
            "JavaScript loader is missing or truncated".into(),
        ));
    }
    if WEB_WASM.len() < 8 || &WEB_WASM[..4] != b"\0asm" {
        return Err(Error::InvalidEmbeddedRuntime(
            "WASM module is missing or has an invalid magic header".into(),
        ));
    }
    for (path, expected_length, expected_digest) in EMBEDDED_ENTRIES {
        let bytes = embedded_bytes(path).ok_or_else(|| {
            Error::InvalidEmbeddedRuntime(format!("manifest entry {path:?} is not embedded"))
        })?;
        if bytes.len() != *expected_length {
            return Err(Error::InvalidEmbeddedRuntime(format!(
                "embedded entry {path:?} has {} bytes; expected {expected_length}",
                bytes.len()
            )));
        }
        let actual_digest = sha256_hex(bytes);
        if actual_digest != *expected_digest {
            return Err(Error::InvalidEmbeddedRuntime(format!(
                "embedded entry {path:?} failed its SHA-256 check"
            )));
        }
    }
    Ok(())
}

fn embedded_bytes(path: &str) -> Option<&'static [u8]> {
    match path {
        "faqe_web.js" => Some(WEB_JS.as_bytes()),
        "faqe_web_bg.wasm" => Some(WEB_WASM),
        "LICENSE" => Some(PROJECT_LICENSE.as_bytes()),
        "THIRD_PARTY.md" => Some(THIRD_PARTY.as_bytes()),
        path if path.starts_with("licenses/") => THIRD_PARTY_LICENSES
            .get_file(path.strip_prefix("licenses/")?)
            .map(|file| file.contents()),
        _ => None,
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

fn escape_xml(value: &str) -> String {
    escape_html(value)
}

fn io_error(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}
