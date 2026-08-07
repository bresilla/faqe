# Legacy compatibility contract

This file records the compatibility behavior retained after the first `faqe`
milestone. The canonical source is `content/`; renderer assets are embedded in
the generator.

## Content and routes

- Canonical Markdown files: **28**, including the typed home descriptor.
- Canonical content routes: **28**, including `/`.
- Total public routes after taxonomy expansion: **93**.
- Old `page/1/` pagination shells and feed XML files are generated support
  artifacts, not independently owned content routes. They are intentionally
  not treated as canonical pages.

## Legacy construct inventory

All shortcodes currently present in `content/` are recognized:

| Shortcode | Uses |
|---|---:|
| image | 4 |

The parser rejects unknown shortcodes, unsafe URL schemes, escaping content
assets, route collisions, excessive nesting, invalid slide-attribute names,
and active raw HTML. Supported shortcode output is built as an explicit typed
element/attribute/text tree before serialization; authored text and attribute
values are escaped at that boundary rather than interpolated into HTML strings.
`posts/software/chroot/lxd_lxc.md` contains the historical slug `lxd-lxc:`. It
is the one explicit invalid-slug compatibility override and continues to own
the published `/post/software/chroot/lxd-lxc/` route. Invalid slugs elsewhere
are rejected rather than silently rewritten.

## Embedded assets

The build stages and hashes the WASM loader/module, Rust-owned generated styles,
and license material before compiling `faqe`. The native binary verifies every
staged byte against its compiled SHA-256 manifest before running any command.
Generated content-local assets are separately content-addressed and owned by
their source path in `build-manifest.json`; no removed legacy framework or
static directory is a runtime input.

Licensing status is recorded in `THIRD_PARTY.md`. The user has superseded the
Daytona/Novecento exact-font requirement by selecting GohuFont uni14 Nerd Font
Mono as the final theme direction. The commercial legacy bytes remain excluded,
but their redistribution is no longer a release decision. The binary embeds the
exact upstream Gohu uni14 base-glyph byte, fingerprints it, and emits it without
requiring a host-installed font. Package, browser, and visual validation status
remains recorded in `PLAN.md`.

Because the complete Nerd Font's combined glyph provenance is not yet suitable
for this package, the safe implementation candidate is the exact unpatched
upstream `gohufont-uni-14.ttf` base-glyph byte with the requested CSS alias,
`GohuFont uni14 Nerd Font Mono`, declared normal/400. The alias preserves the selected Gohu
shape inside this renderer; it does not claim the OS-installed family, Nerd
Fonts patching, or Nerd/icon glyph coverage. FAQE's reviewed icons remain the
separate local SVG subset described in `THIRD_PARTY.md`.
