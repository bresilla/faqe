# Theme contract

FAQE themes are complete, named visual packages compiled into the generator.
The site selects one package at build time:

```toml
# site.toml
theme = "bresilla"
```

An unknown theme identifier fails both `faqe check` and `faqe build`. Theme IDs
use lowercase ASCII letters, digits, and single hyphens. Run `faqe themes` to
list the themes compiled into a particular executable.

## Ownership boundary

A theme owns all static presentation decisions:

- base site CSS, including navigation, cards, posts, accessibility, and print;
- resume-specific CSS;
- browser-presentation CSS;
- generated motion CSS;
- font-face declarations and embedded fonts;
- any other embedded images, textures, or theme assets.

The renderer owns structure, semantics, navigation, and runtime state. Inline
styles are limited to values that only exist at runtime, such as a page palette,
card palette variables, slide transforms, progress widths, skill percentages,
and content-selected media. The document root exposes `data-faqe-theme` and
`data-faqe-scheme` so theme CSS can react to the selected package and the page's
light or dark color scheme.

Page styles remain parameters passed into the selected theme. They provide the
accent, two chromatic channels, background, foreground, and light/dark scheme.
They do not select a different theme for an individual page.

## Post titles

The Bresilla theme presents every post title as a wide editorial heading. The
last whitespace-delimited word uses the page's accessible accent color while
the preceding words retain the foreground color. Both parts use the shared
scramble renderer, but the accent word decodes for longer, glitches more often,
and displaces its chromatic channels farther.

Single-word titles render entirely in the accent treatment. Reduced-motion
visitors receive the same size and color hierarchy without animation. Authors
only set the ordinary front-matter `title`; no per-post markup is required.

## Theme module

Theme implementations live in `crates/faqe-cli/src/theme/`. A definition
contains an ID, assets, font faces, and the base, resume, talk, and motion style
layers. FAQE fingerprints every registered asset and stylesheet. Stylesheets
may reference an asset by ID with this build-time placeholder:

```css
.surface { background-image: url('{{asset:grid}}'); }
```

The placeholder resolves to the emitted fingerprinted filename. Missing and
duplicate asset IDs are build errors.

## Talk templates

A theme may register per-deck talk templates in `Definition::talk_templates`.
A `TalkTemplate` has an ID, a slide canvas width and height, its own assets, and
a talk-stylesheet layer scoped to `.faqe-talk[data-faqe-talk-template="<id>"]`.
A deck selects one with `template = "<id>"` in its `.faqe.toml` sidecar. The
build records the template's canvas on the deck, the renderer scales slides to
that canvas, and each slide exposes `data-faqe-slide-number` for template
chrome. Template asset IDs share the theme's namespace and must be unique.

Templates live beside the themes in `crates/faqe-cli/src/theme/`; `wur.rs` is
the reference template and is registered by the `bresilla` theme.

## Adding a theme

To add a `cyberpunk` theme:

1. Create `crates/faqe-cli/src/theme/cyberpunk.rs` with its complete
   `Definition`.
2. Put all of its static styling and assets in that module; do not add
   theme-specific branches to the Yew renderer or HTML generator.
3. Register its ID in `theme::resolve` and `theme::available` in
   `crates/faqe-cli/src/theme/mod.rs`.
4. Set `theme = "cyberpunk"` in the test content's `site.toml`.
5. Run `make verify-all` in the repository's development shell.

The `bresilla` module is the reference implementation.
