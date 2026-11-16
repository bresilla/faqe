# Content directory contract

`faqe` accepts a structured content directory, not a theme or prebuilt website.
Every immediate child directory is a site surface and must contain an
`_index.md` descriptor. Hidden entries and every symlink are skipped, and a
symlink escaping the content root is an error. Builds only read the content tree,
so read-only inputs work.

The canonical shape is:

```text
content/
  home/_index.md       # the site name and home surface
  about/_index.md      # visible singleton tab
  cv/_index.md         # visible CV tab
  posts/_index.md      # visible dirstruct of post items
  posts/.../*.md       # type = "post"
  talks/_index.md      # visible dirstruct of presentations
  talks/.../*.md       # Presenterm-compatible Markdown
  key/_index.md        # indirect public-key surface
  quotes/_index.md     # indirect footer destination
```

Top-level folder descriptors require `type`, `visibility`, `title`, and `slug`.
Visible folders also require `weight`. `visibility = "tab"` adds the folder to
primary navigation; `visibility = "indirect"` keeps it out of navigation while
allowing another page or component to link to it. Exactly one folder has
`type = "home"`; its title is the website name and its route is `/`. Physical
folder names and public routes are deliberately independent: for example,
`posts/_index.md` uses `slug = "post"` to preserve `/post/`.

```toml
# home/_index.md
type = "home"
visibility = "tab"
weight = 0
title = "Example Site"
slug = "home"

# posts/_index.md
type = "dirstruct"
item_type = "post"
visibility = "tab"
weight = 40
title = "Posts"
slug = "post"

# quotes/_index.md
type = "quotes"
visibility = "indirect"
title = "Quotes"
slug = "quotes"
```

`type = "dirstruct"` declares a recursive collection and requires
`item_type = "post"` or `item_type = "presentation"`. Every nested `_index.md`
repeats the same dirstruct type and item type. Post items have an explicit type
matching the containing dirstruct. Presentation items inherit their type from
the folder and use Presenterm YAML instead. Singleton folders contain only
`_index.md`. Current singleton types are `home`, `about`, `cv`, `key`, and
`quotes`.

The application resolves indirect destinations by type rather than hard-coded
paths: the home logo targets `key`, and footer quotes target `quotes`. Changing
one of their descriptor slugs therefore changes its public route without
requiring a renderer edit.

Folder descriptors, singleton pages, and posts start with TOML between `+++`
lines. Presentations use the YAML front matter defined by Presenterm. Ordinary
items derive their parent route from the top-level descriptor's `slug` and
their final segment from their own `slug` or filename. Filenames may contain
spaces or Unicode when an explicit lowercase ASCII slug is provided. Duplicate
and case-folded routes are errors.

## `site.toml`

An optional root `site.toml` configures shared metadata. Without it, embedded
defaults apply for description, palette, author, keywords, media, footer, and
social links. The executable contains no Bresilla-specific metadata or
navigation defaults;
the example site's personal values are explicitly owned by `content/`.
Unknown top-level or nested fields are errors rather than ignored typos. Its
complete schema is:

| Field | Type | Meaning |
| --- | --- | --- |
| `site_url` | string | Absolute production origin, without a path. |
| `theme` | string | Lowercase identifier of the named theme compiled into the generator. Defaults to `bresilla`. |
| `author` | string | Site author. |
| `description` | string | Default description. |
| `keywords` | array of strings | Default search/metadata keywords. |
| `info` | string | Homepage role or summary text. |
| `avatar` | string | Content-root-relative avatar asset. |
| `avatar_hover` | string | Content-root-relative hover avatar asset. |
| `favicon` | string | Content-root-relative favicon asset. |
| `default_card_thumbnail` | string | Content-root-relative fallback required when a post or presentation card has no page thumbnail. |
| `default_foot` | string | Footer text inherited by pages. |
| `disclaimer_title` | string | Optional title for the post disclosure shown when a post has a punchline. |
| `disclaimer_paragraphs` | array of strings | Content-owned disclosure paragraphs; empty suppresses the disclosure. |
| `references_copyright` | string | Optional first line below post reference links. |
| `references_notice` | string | Optional content-owned notice below post reference links. |
| `socials` | array of tables | Social entries described below. |
| `default_style` | table | Page-style defaults described below. |

`title` and `menu` are rejected in `site.toml`: the home descriptor owns the
title and tab folders own navigation. Each `[[socials]]` table requires exactly
`name`, `glyph`, and `url` strings plus an integer `weight`. Socials sort by
weight, then name, then URL.

`[default_style]` contains `accent`, `background`, and `foreground` CSS hex
colors; a two-color `chromatic` array used by glitch effects; `theme`
(`"dark"` or `"light"`), which is the page color scheme rather than the
top-level named theme; and optional content-root-relative `video`.
Page styles may override the same fields. When a page overrides `accent`
without declaring `chromatic`, FAQE pairs that accent with a deterministic
channel-rotated partner. Supplying the default-style table requires its
non-optional fields. Deployment subpaths are not part of `site.toml`; pass them
through the CLI `--base-url`.

Referenced media is resolved relative to the Markdown file (or `site.toml`),
must remain inside the content root, and is fingerprinted into generated output.
Post and presentation pages must either declare their own sibling-relative
`thumbnail` or inherit a non-empty `default_card_thumbnail`. The fallback remains
content-owned and fingerprinted; the executable does not contain a hidden logo
or generic card image. Card images are decorative because the linked title
already names the destination, but the image box remains present for stable
geometry.

### Image alternatives and captions

Image alternative text remains content-owned. An authored non-empty `alt` is
preserved, while an explicitly authored `alt=""` always means decorative and is
never replaced. FAQE derives an omitted `alt` only when all of these conditions
hold: the image is the sole image in a `<figure>`; the figure has exactly one
direct `<figcaption>`; the caption contains plain text only; and its normalized
text is between 1 and 280 characters. Whitespace is collapsed before use.
Links, emphasis, code, nested elements, multiple images/captions, empty text,
and longer captions are never flattened or guessed. If no explicit alternative
can be derived, FAQE normalizes the image to `alt=""`; authors must supply a
non-empty `alt` whenever such an image is informative.

For the legacy `image` shortcode, omitting `alt` opts into this narrow caption
derivation policy. Writing `alt=""` records deliberate decorative intent. This
distinction survives the typed JSON document and is applied equally to the
generated semantic shell and WASM view.

Supported legacy shortcodes in non-presentation content are parsed into an
explicit node tree and rendered through typed elements, attributes, text, and
raw nested-Markdown fragments. Text and attribute values are escaped only by
the structural serializer.

## Rich Markdown primitives

FAQE extends fenced code blocks with typed, declarative visual primitives. A
primitive uses `faqe:<name>` as its info string and TOML as its body. Ordinary
renderers keep showing a readable code block, while FAQE validates the TOML and
renders semantic HTML in both the generated shell and the WASM view.

Posts and presentations use the same primitive pipeline. Standard fenced code
blocks are the built-in `code-block` primitive; `faqe:*` fences select a typed
declarative primitive instead. Presenterm slide boundaries, pauses, columns,
and fragments wrap the resulting document nodes without replacing them.

Primitive text fields accept inline Markdown. Unknown fields, invalid enum
values, missing references, non-rectangular tables, and values outside declared
ranges fail the content build instead of producing a partly rendered widget.
The current registry contains `hero`, `prose`, `graph`, `timeline`, `progress`,
`grid`, and `table`.

### Feature hero

Use `hero` when a long-form post needs an editorial title block, visual, and
compact fact strip. A hero requires `title`, accepts `eyebrow`, `standfirst`,
and optional image fields, and may contain up to eight stats. `image` and
`image_alt` must be provided together; `image_caption` accepts inline Markdown.

````markdown
```faqe:hero
eyebrow = "Research dossier · 12 source files"
title = "The Balthasar Evidence Chain"
standfirst = "How a statement becomes durable memory."
image = "balthasar-hero.jpg"
image_alt = "Glowing fragments suspended in a dark room"
image_caption = "Photo by [Johnny Guitar](https://unsplash.com/@johnnyguitarks)"

[[stats]]
label = "Witnesses"
value = "8"

[[stats]]
label = "Promotion threshold"
value = "0.50"
```
````

The hero activates the theme's wide feature layout for that post while keeping
the site's configured pixel-font stack. It does not accept custom HTML, CSS, or
scripts.

### Prose columns

`prose` lays out two to four Markdown blocks as editorial columns. Each entry
accepts block Markdown and must be non-empty.

````markdown
```faqe:prose
columns = [
  "A transcript records **what happened**.",
  "A project database records **what remains true**."
]
```
````

### Graph

Graphs use an explicit grid so a complex architecture diagram stays
deterministic without a browser layout engine. Coordinates are one-based.
Nodes require unique ASCII IDs; edges must name existing nodes. Groups are
optional visual regions. `tone` accepts `accent`, `positive`, `warning`,
`negative`, `muted`, or `neutral`.

````markdown
```faqe:graph
title = "Everything, connected"
caption = "Solid lines happen live; dashed lines happen between sessions."
description = "A harness writes turns which are promoted into durable memory."
columns = 3
rows = 2

[[groups]]
label = "LIVE"
column = 1
row = 1
width = 2
height = 2

[[nodes]]
id = "harness"
title = "agent / harness"
subtitle = "anything that can observe"
column = 1
row = 1
tone = "accent"

[[nodes]]
id = "transcript"
title = "transcript.db"
subtitle = "turns, verbatim"
column = 2
row = 1
tone = "neutral"

[[nodes]]
id = "project"
title = "project.db"
subtitle = "durable facts"
column = 3
row = 2
tone = "positive"

[[edges]]
from = "harness"
to = "transcript"
label = "observe"

[[edges]]
from = "transcript"
to = "project"
label = "promote"
dashed = true
tone = "warning"
```
````

A graph may have up to 12 columns, 12 rows, 64 nodes, and 128 edges. Only one
node may occupy a cell. Authors control topology and grouping; FAQE controls
geometry, responsive overflow, colors, arrow rendering, and the accessible
text equivalent.

### Timeline

````markdown
```faqe:timeline
title = "What happens to one turn"
caption = "Most sentences stop at the third station."

[[items]]
title = "The harness calls `observe`"
meta = "transcript.db · turn"
body = "The durable transcript write happens first."
tone = "positive"

[[items]]
title = "The claim waits for evidence"
meta = "hold 0.30"
body = "Waiting and refusal remain different states."
tone = "warning"

[[items]]
title = "The claim becomes assertable"
meta = "inject 0.35"
tone = "accent"
```
````

Timeline items require `title`; `meta`, `body`, and `tone` are optional. The
result is an ordered list, so its order and meaning survive CSS, printing, and
non-visual browsing.

### Progress and threshold charts

````markdown
```faqe:progress
title = "Witness weights"
caption = "Some witnesses cross alone; others wait."
max = 1.0

[[items]]
label = "SAID"
value = 1.0
display = "1.00"
text = "the person asked to remember it"
tone = "positive"

[[items]]
label = "TIDE"
value = 0.30
display = "0.30"
text = "it left the context window"
tone = "warning"

[[markers]]
label = "hold 0.30"
value = 0.30
tone = "warning"

[[markers]]
label = "promote 0.50"
value = 0.50
tone = "positive"
```
````

`max` defaults to `100`. Every item and marker value must be finite and within
`0..max`. Each bar carries native progressbar roles and numeric ARIA values;
`display` changes only the visible number.

### Card and metric grids

````markdown
```faqe:grid
title = "Every way in"
columns = 3
variant = "cards"

[[items]]
eyebrow = "remember"
title = "Somebody typed it"
body = "Direct intent is the strongest evidence."
badge = "crosses alone"
tone = "positive"

[[items]]
eyebrow = "consolidate"
title = "Two runs agreed"
body = "Independent repetition becomes evidence."
badge = "waits"
tone = "warning"
```
````

Grid variants are `cards`, `metrics`, `compact`, and `strip`; `columns` is
between one and six and collapses to one column on narrow screens. An item may
also contain `bullets = ["..."]` and structured `[[items.lines]]` entries with
`label`, `value`, and optional `meta` and `tone` fields. Lines render as a
definition list for measurement and benchmark cards.

### Typed tables

Standard Markdown tables remain the shortest choice for ordinary data. The
typed table primitive adds presentation variants and per-cell status tones.
Variants are `plain`, `striped`, `comparison`, `matrix`, and `metrics`.

````markdown
```faqe:table
title = "balthasar and mem0"
caption = "The first cell in every row is emitted as its row header."
variant = "comparison"
columns = ["Axis", "mem0", "balthasar"]

[[rows]]
cells = ["History", "Current state", "Validity intervals"]
tones = ["neutral", "negative", "positive"]

[[rows]]
cells = ["Recall", "Vector search", "Memory plus verbatim spans"]
tones = ["neutral", "neutral", "positive"]
```
````

Rows must match the declared column count. `tones` may be omitted or must have
one entry per cell. The first cell is a semantic row header, while every
declared column becomes a column header.

### Adding the next primitive

Primitive implementations live together under
`crates/faqe-content/src/primitives/`:

- `mod.rs` owns the shared fenced-block entry point.
- `code_block.rs` owns ordinary fenced code and syntax highlighting.
- `declarative.rs` owns the strict `faqe:*` registry and data renderers.
- `kind.rs` classifies standard Markdown, shortcode, and declarative output as
  typed document elements shared by posts and presentations.

Rich primitives are a registry rather than more shortcode branches. A new
declarative primitive has four explicit integration points:

1. Add its strict TOML data structs and renderer to
   `crates/faqe-content/src/primitives/declarative.rs`, then register one
   `PrimitiveSpec`.
2. Emit one semantic root carrying `data-faqe-primitive="<name>"`; do not emit
   scripts, event handlers, or arbitrary author CSS.
3. Classify the root with a matching `ElementKind` in `faqe-model`, bump the
   site schema, and add theme-owned responsive and print CSS.
4. Document the contract and add validation plus semantic-output tests.

This boundary keeps authoring data-only, keeps rendering deterministic and
offline, and lets future primitives share the existing sanitizer, typed JSON
document, semantic fallback, WASM renderer, and theme system.

## Presentations

Presentation items follow Presenterm's source contract so the same `.md` file
can be opened by FAQE in a browser or passed directly to `presenterm` in a
terminal. They require YAML front matter between `---` lines and do not require
FAQE's `type = "presentation"` field. Front matter such as `title`,
`sub_title`, `author` or `authors`, `date`, `theme`, and `options` is left in
Presenterm's native shape. FAQE creates the same automatic introduction slide
when title, subtitle, or author metadata is present.

Slides end with `<!-- end_slide -->`. Setext headings are slide titles. FAQE
maps Presenterm comment commands for pauses, incremental lists, alignment, font
size, vertical centering, explicit new lines, footer suppression, skipped
slides, speaker notes, includes, and column layouts. Unknown Presenterm
commands remain harmless comments, keeping the source forward-compatible.

```markdown
---
title: My presentation
sub_title: One source for terminal and web
author: Example Author
theme:
  name: dark
---

First slide
===========

Visible immediately.

<!-- pause -->

Visible after advancing.

<!-- end_slide -->

Second slide
============
```

Presenterm image attributes remain valid, including
`![image:width:50%](image.png)`. Local images use the normal relative asset
resolution and fingerprinting path. A standard Markdown link to a local
`.mp4`, `.webm`, or `.ogg` file remains a link in Presenterm and is upgraded to
an embedded player by FAQE's browser renderer. This keeps the source valid for
both renderers without adding a FAQE-only shortcode.

FAQE reads Presenterm theme overrides for `default.colors.foreground`,
`default.colors.background`, `palette.colors.accent`,
`palette.colors.chromatic`, and `slide_title.colors.foreground`. The browser
theme uses those values for its page palette and computes accessible text and
interactive colors for the selected background.

FAQE-only card and taxonomy metadata stays out of the shared Markdown. An
optional sibling named `<deck>.faqe.toml` may contain only `slug`, `thumbnail`,
`foot`, `link`, `part`, `credits`, `tags`, `categories`, `series`, `template`,
`title_layout`, and `title_image`. New decks should normally derive their route
from the filename and omit the sidecar. This keeps every presentation file valid
under Presenterm's default strict front matter parser.

### Talk templates

`template` selects a per-deck talk template provided by the site's theme; an
unknown template fails `faqe check` and `faqe build`. A template owns the slide
canvas size and its own scoped styling and artwork. `title_layout` is `wide`,
`left`, or `right` and becomes a `title-<layout>` class on the introduction
slide. `title_image` is a sibling-relative image used as the introduction
slide's background. The introduction slide carries `title-subtitle`,
`title-date`, and `title-author` paragraphs so templates can place each line.
Templated talks get a dark scheme toggle: the `D` button under TOOLS, or the
`D` key, sets `data-faqe-talk-scheme="dark"` on the talk and remembers the
choice in the browser. Templates style it under that attribute.

```toml
# lecture.faqe.toml
template = "wur"
title_layout = "left"
title_image = "lecture/title.jpg"
```

The `bresilla` theme provides `wur`, the Wageningen University & Research
layout on a 960 × 540 canvas. It maps ordinary Presenterm structure onto the
WUR designs without FAQE-only commands:

| Presenterm source | WUR rendering |
| --- | --- |
| front matter `title`, `sub_title`, `date`, `author` | title slide (`title_layout` picks the wide band or the left/right panel) |
| `jump_to_middle` | section slide on the left green panel |
| `jump_to_middle` + `alignment: right` / `center` | right blue panel / wide lime band |
| image inside a `jump_to_middle` slide | full-bleed section background |
| a slide holding only one image | full-slide image without footer |
| `column_layout` | WUR text/image split columns |
| `no_footer` | hides the footer bar and slide number |

Use the WUR palette in the Presenterm theme override so the terminal rendering
matches: background `ffffff`, foreground `002b47`, accent `004d00`, chromatic
`97c600`, and slide title `004d00` with `theme.name: light`.

### Decorative background video

`style.video` selects a content-local decorative background asset; it does not
grant content control over playback. A page-level `style.autoplay` key is an
error rather than a silently ignored promise. FAQE owns playback policy so the
generated client can suppress loading and autoplay for reduced-motion and
Save-Data users. Authors should provide meaningful page content that does not
depend on the video. The generated runtime makes one explicit `play()` attempt,
does not race a native autoplay attribute, and removes the video on synchronous
failure, Promise rejection, or media error while keeping the decorative
poster/background surface. It does not retry until the component is remounted.

## Lists, taxonomies, and pagination

Section front matter may set `folders = true` and `number = 1..100`. The legacy
folder template defaults to six cards, so folder-enabled sections apply that
limit after deterministic date/route ordering. Ordinary sections and taxonomy
term pages render all current members rather than hiding content behind newly
invented pagination routes.

Taxonomy roots (`/tags/`, `/categories/`, `/series/`, and `/type/`) deliberately
retain the audited legacy empty-grid presentation with a visible singular
prefix and plural title. They do not masquerade as folder lists. Term pages
retain the card grid inside the legacy `.thelist` wrapper. Historical page-1
paths remain redirects from `aliases.toml`; FAQE does not generate new page-2
or taxonomy/section pagination shells, and redirect paths stay outside the
sitemap.

## Stable public files

An optional root `public-files.toml` publishes content-owned files at stable,
unfingerprinted paths. It contains only a `[files]` table whose keys are output
paths and whose values are source paths, both relative to the content root:

```toml
[files]
"public.asc" = "public.asc"
"public.txt" = "public.asc"
"CNAME" = "CNAME"
```

Paths must be normalized, non-empty, relative paths without `..` or backslash.
Sources must canonicalize to regular files inside the content directory.
Targets are checked case-insensitively for duplicates, path-ancestry overlap,
and collisions with routes, generated files, fingerprinted content assets,
`assets/`, and `licenses/` before output is written. Declarations are parsed by
the content loader so an absolute Markdown link such as `/public.asc` is
validated against the same target later emitted by the CLI.

Use this manifest only for compatibility endpoints that require stable names.
Ordinary page media remains sibling-relative and fingerprinted. A source may be
both fingerprinted and copied to a stable target when both contracts are
explicitly required.

## Historical aliases

An optional root `aliases.toml` owns redirects from evidenced historical HTML
paths to current canonical routes:

```toml
[aliases]
"/posts/linux/lxd-lxc/" = "/post/software/chroot/lxd-lxc/"
"/tags/linux/page/1/" = "/tags/linux/"
```

Both source and target must be normalized absolute directory routes with
leading and trailing `/`, without queries, fragments, or backslashes. Targets
must be current content, taxonomy, or home routes. Sources may not equal or
collide case-insensitively with canonical/generated routes or other aliases.
The generated redirect is a small readable HTML document with a base-aware
link, `noindex`, an immediate refresh, and an absolute canonical target. It does
not load the site bundle or WASM. Aliases are recorded separately from canonical
routes in `build-manifest.json` and are deliberately excluded from the sitemap.

Only declare an alias when historical output proves the old path and its target
are equivalent. Feed compatibility is a separate contract; do not map an XML
feed path to an HTML page.

## Feed routes

An optional root `feeds.toml` owns the RSS compatibility paths generated by a
site. It contains one sorted `routes` array:

```toml
routes = [
  "/index.xml",
  "/post/index.xml",
  "/tags/linux/index.xml",
]
```

Routes must be unique canonical absolute paths ending in `index.xml`, without
queries, fragments, or backslashes. The root `/index.xml` feed is mandatory;
when the manifest is absent it is the only feed generated. Feed files cannot
collide with other generated or content-owned output.

The root feed contains published posts. Taxonomy-root and taxonomy-term feeds
derive members from the published taxonomy model. Section feeds contain direct
published children and canonical targets selected by historical aliases. An
explicit compatibility feed with no current published members is emitted as a
valid empty feed instead of reviving retired content. Items sort by descending
validated date and then canonical route. Channel, Atom self, item, and GUID URLs
are absolute and respect `--base-url`; XML text is escaped by the generator.

## Limits

The loader rejects input above these byte limits before generation:

| Input | Limit |
| --- | ---: |
| One Markdown file, including front matter | 8 MiB |
| `site.toml` | 64 KiB |
| `public-files.toml` | 64 KiB |
| `aliases.toml` | 64 KiB |
| `feeds.toml` | 64 KiB |
| All discovered Markdown source | 64 MiB |
| One referenced content asset | 32 MiB |
| All referenced content assets | 128 MiB |
| One stable public file | 16 MiB |
| All stable public files | 32 MiB |

Only referenced assets count toward the asset totals. Draft-only assets are not
resolved or emitted in a release load.

## Determinism

Discovery, route, alias, and feed ordering, menu/social weight ordering, asset
naming, and output serialization are deterministic for identical input bytes.

## Validation without publication

`faqe check CONTENT_DIR` runs the same content loading, publication filtering,
route/alias ownership, public-file loading, route-shell generation, embedded asset
emission, bundle/generated-reference checks, feed/sitemap generation, and
manifest construction as `faqe build`. It renders into a process-unique scratch
directory and removes that directory on both success and ordinary error paths,
so checking does not replace or create a user-selected output tree.
