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
