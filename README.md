# faqe

One-binary Markdown-to-WASM website generator.

```sh
faqe check ./content
faqe build ./content --output ./dist
faqe serve ./content
faqe assets
```

The released `faqe` executable embeds the Yew WebAssembly application and emits
the HTML shell and Rust-owned theme CSS. Icons and presentations are rendered by
Rust/Yew. It does not ship an asset/theme directory, Reveal.js, Bootstrap, Font
Awesome, jQuery, highlight.js, or a separately managed webfont bundle. The
small maintained Iosevka and GohuFont uni14 font files are embedded and emitted
by the executable. CSS exposes the upstream Gohu base glyphs under the requested
`GohuFont uni14 Nerd Font Mono` family; Nerd icon glyphs are deliberately not
bundled because the site uses reviewed local SVG icons. End users do not need
Hugo, Node, Sass, Trunk, Rust, or a separate theme installation.

Personal media belongs in the content directory. An optional `site.toml` beside
the Markdown can select the avatar, hover logo, and favicon with relative paths;
the generator fingerprints and emits those files. With no `site.toml`, safe
built-in metadata defaults are used.

Content is organized by top-level surface folders. `home`, `about`, `cv`,
`posts`, and `talks` are visible tabs in the canonical site; `identity`, `key`,
`quotes`, `skills`, and `lists` are indirect surfaces reached from other UI.
Each folder has a typed `_index.md`, while dirstruct items declare their own
`post` or `presentation` type. The home descriptor owns the website name and
folder visibility/weight owns navigation; neither is duplicated in `site.toml`.

See [the content directory contract](docs/content-contract.md) for deterministic
discovery, front matter, sections, routes, symlinks, and media rules.

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
