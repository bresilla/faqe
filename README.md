# faqe

One-binary Markdown-to-WASM website generator.

```sh
faqe check ./content
faqe build ./content --output ./dist
faqe serve ./content
faqe assets
faqe themes
```

The released `faqe` executable embeds the Yew WebAssembly application and a
registry of complete named themes. A site selects one compiled theme in
`site.toml`; that theme owns its base, resume, presentation, motion, font, and
static asset layers. The current design is the `bresilla` theme. See
[the theme contract](docs/themes.md) for the boundary and the steps required to
add another theme such as `cyberpunk`.

Icons and presentations are rendered by Rust/Yew. FAQE does not require
Reveal.js, Bootstrap, Font Awesome, jQuery, highlight.js, or a separately
managed webfont bundle. Theme assets are fingerprinted and emitted by the
executable. End users do not need Hugo, Node, Sass, Trunk, Rust, or a separate
theme installation.

Personal media belongs in the content directory. An optional `site.toml` beside
the Markdown can select the named theme, avatar, hover logo, and favicon; the
generator fingerprints and emits referenced files. With no `site.toml`, safe
built-in metadata and the `bresilla` theme are used.

Content is organized by top-level surface folders. `home`, `about`, `cv`,
`posts`, and `talks` are visible tabs in the canonical site; `key` and `quotes`
are indirect surfaces reached from other UI. Each folder has a typed
`_index.md`, while dirstruct items declare their own `post` or `presentation`
type. The home descriptor owns the website name and folder visibility/weight
owns navigation; neither is duplicated in `site.toml`.

See [the content directory contract](docs/content-contract.md) for deterministic
discovery, front matter, sections, routes, symlinks, and media rules.

Presentation items use Presenterm's YAML front matter, `<!-- end_slide -->`
boundaries, and comment commands directly. Run the example in a terminal with
`make present`, or choose another deck with
`make present PRESENTATION=content/talks/agro/precisionag.md`; FAQE renders the
same file as an accent-aware browser presentation.

Use `--base-url /name/` with `faqe build` when publishing below a domain
subpath. Builds are deterministic and atomic: invalid content leaves the last
good output untouched. `faqe check` exercises the complete generator and
generated-reference validation in an automatically removed scratch directory;
it does not commit output. `faqe assets` verifies and lists the embedded runtime
manifest; `faqe licenses` prints its attribution inventory.

See [deployment and preview behavior](docs/deployment.md) for the distinction
between `site_url` and `--base-url`, static-host deployment, and the security
limits of VPN-visible preview serving.

Development and validation use the repository Makefile:

```sh
git submodule update --init --recursive
make build
make run
make content-check
make verify
make verify-all
make release-build
make package
```

The `content/` test fixture is the `bresilla/website` repository, tracked as a
submodule. `make content-check` validates it with the locally built generator,
and `make verify` includes that validation.
