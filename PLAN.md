# FAQE implementation and parity plan

> **Milestone accepted on 2026-08-03.** The owner confirmed that the generated
> site reached the desired starting point. At the owner's request, all automated
> tests, visual/semantic oracles, fixtures, smoke scripts, and test CI were then
> removed. This document is retained as implementation history; unchecked test
> work below is not an active delivery requirement.

> **Current content contract:** top-level folders now own site surfaces. The
> visible `home`, `about`, `cv`, `posts`, and `talks` folders derive primary
> navigation; `identity`, `key`, `quotes`, `skills`, and `lists` are typed
> indirect surfaces. Dirstruct folders enforce `post` or `presentation` item
> types while preserving the established public `/post/`, `/talk/`, and
> `/resume/` routes.

## 1. Goal

Build a single native `faqe` executable that accepts a content directory,
validates it, and generates or serves a polished WebAssembly website.

The released runtime model is:

```text
faqe binary + user-selected content directory -> generated website
```

The binary owns and generates the HTML shell, Rust/Yew WebAssembly runtime,
theme CSS, bootstrap JavaScript, icons, licenses, and other application assets.
Users must not maintain a separate HTML template, CSS theme, JavaScript bundle,
Node project, Hugo installation, Sass pipeline, or Rust toolchain.

Personal media and content-specific files belong inside the selected content
directory, preferably beside the Markdown that references them. The generator
fingerprints and emits those files.

`./xtra/website` was a temporary design and behavior reference only. No build,
runtime, generated file, documentation example, or released binary depends on
that directory. It is safe to delete.

## 2. Non-negotiable constraints

1. **One runtime binary.** The website application and generated scaffolding
   are compiled into `faqe`. Distribution archives may include license files,
   but the program must not require them beside the executable at runtime.
2. **Content ownership.** Personal images, video, downloadable keys, and
   article-local media come from the selected content directory.
3. **No dependency on `xtra`.** It is reference material, not an input path.
4. **Rust-first implementation.** Page rendering, interactions, parsing,
   presentation behavior, and asset generation belong in Rust/Yew unless a
   tiny generated bootstrap script is technically necessary.
5. **Generated HTML and CSS are allowed.** Users do not manage those files.
6. **Faithful current-site migration first.** General arbitrary-directory
   conventions can be finalized after the existing website is visually and
   behaviorally correct.
7. **No copied framework trees.** Do not restore complete Bootstrap,
   Font Awesome, Reveal.js, jQuery, highlight.js, or legacy theme directories.
   Implement required behavior and embed only the smallest justified assets.
8. **Preserve published URLs and anchors.** Existing routes, feeds, downloads,
   and heading IDs are public compatibility contracts unless explicitly
   retired with redirects.
9. **Safe output.** Builds must never overwrite content, the running binary, or
   an unrelated directory through lexical paths, symlinks, collisions, races,
   or failed validation.
10. **Repository-native verification.** Use the Makefile for builds and gates.

## 3. Current architecture

### 3.1 Workspace

- `crates/faqe-model` owns the serialized site schema and typed page models.
- `crates/faqe-content` discovers Markdown, parses TOML front matter and
  shortcodes, sanitizes HTML, creates document trees, fingerprints content
  assets, parses public-file declarations, and derives routes/taxonomies.
- `crates/faqe-web` is the Yew WebAssembly application and interaction layer.
- `crates/faqe-cli` embeds the WASM runtime, owns generated themes, validates
  content/output, writes route shells and metadata, and provides the preview
  server.
- `content/` is the current canonical example site.
- `tests/` owns route, browser, interaction, package, and future immutable
  compatibility fixtures.

### 3.2 Implemented foundations to retain

- One native binary embeds the WASM runtime and Rust-owned themes.
- `crates/faqe-web/assets` has been removed.
- Reveal.js, Bootstrap, jQuery, Font Awesome, highlight.js, copied CSS trees,
  and unused webfont trees are not runtime dependencies.
- Content-local media is fingerprinted under `assets/content/`.
- Builds are deterministic for the current fixtures.
- Invalid content preserves the last successfully generated output in ordinary
  error paths.
- Content discovery rejects symlinks that escape the content root.
- Content assets are canonicalized, confined to the content root, size-limited,
  hashed, and emitted only when referenced.
- Generated assets receive immutable caching in preview mode.
- Percent-decoding happens before preview traversal validation.
- Unit, interaction, visual, and package lanes exist, but their fidelity and
  coverage require the changes below.

## 4. Audit status and truth

The current website is functional, but it is not yet an exact replica.

The earlier visual-success claim was invalid because the current renderer had
overwritten its own comparison images. That defect is now contained:

- `tests/visual/baseline/` contains 68 captures made directly from legacy
  commit `8c73c4caa12400304f43479a5f7d1bd617beea38`;
- `tests/oracle/` preserves hashes, legacy routes, headings, metadata, and
  public-file evidence independently of the current renderer;
- `make visual-oracle` can only capture from a supplied legacy checkout;
- `make visual` can only compare the current renderer with the immutable
  captures; and
- `make oracle-check`, included by `make verify`, detects any fixture drift.

The latest measured RMSE values include the fifth-round content-policy,
responsive, local-icon, preview-cache, and targeted visual work. The ordinary
raster lane forces reduced motion, so exact animated glitch tracks remain
proven separately by the interaction lane. The captures still fail the strict
compatibility threshold and therefore remain work, not success:

| Page | Desktop | Small | Tablet | Phone |
|---|---:|---:|---:|---:|
| Home | 0.094759 | 0.049542 | 0.160952 | 0.213924 |
| About | 0.133336 | 0.157993 | 0.186044 | 0.233781 |
| Lists | 0.065312 | 0.091504 | 0.066516 | 0.104592 |
| Progress | 0.082432 | 0.122875 | 0.123685 | 0.162795 |
| Post | 0.121686 | 0.195714 | 0.293371 | 0.210283 |
| Dark article | 0.171363 | 0.180221 | 0.209928 | 0.238886 |
| Light article | 0.140068 | 0.182631 | 0.177077 | 0.225903 |
| Resume | 0.130157 | 0.140191 | 0.175873 | 0.105131 |
| Logo | 0.044381 | 0.056991 | 0.033222 | 0.050033 |
| PGP | 0.161235 | 0.185474 | 0.226335 | 0.243911 |
| Talk | 0.105809 | 0.161421 | 0.094381 | 0.069899 |
| Taxonomy | 0.125650 | 0.183796 | 0.187548 | 0.283813 |
| Tags root | 0.053852 | 0.070180 | 0.046488 | 0.073777 |
| Categories root | 0.061397 | 0.080357 | 0.060336 | 0.095186 |
| Series root | 0.056880 | 0.074114 | 0.053544 | 0.083989 |
| Type root | 0.049665 | 0.064620 | 0.039466 | 0.062058 |
| 404 | 0.069479 | 0.090542 | 0.075299 | 0.118293 |

Twenty-two of 68 captures pass with Gohu and 46 remain above the strict 0.075
threshold. The new face is intentionally not accepted by refreshing the oracle.
A scoped static reduced-motion accent still preserves the archived root-title
foreground, but categories small/phone, series phone, and several ordinary
pages remain red. The result is worse than the former Work Sans 25/68 snapshot,
so the next visual-polish round must tune component typography and geometry for
Gohu rather than restoring the rejected 8px browser frame. Scoped `-0.1em`
title tracking removed the fixed-cell phone-title overflow, reducing
progress-phone from `0.378351` to `0.162795`; a scoped Reveal base-size change
from 4rem to 3.6rem improved all four deck captures and made talk-phone pass.
The original 52 files were hash-verified unchanged before and after the
additive legacy-only capture.

The selected GohuFont uni14 direction is implemented and measured; the primary
visible blocker is incomplete page-wide raster parity. Dedicated
taxonomy-root raster captures are complete. Presentation theme isolation, exact
control cosmetics, and the
print/PDF lane now pass. The ordinary unzoomed 320px/tablet matrix,
767/768/769 boundary, and 200%/400% reflow lanes now pass. The fallback is
the shipped, safely licensed Gohu base face exposed under the requested
`GohuFont uni14 Nerd Font Mono` alias. Its package, browser, accessibility,
print, and full immutable visual-matrix gates have run; the visual gate remains
red rather than being accepted by refreshing the oracle. Work Sans and its OFL
file have been removed.

### 4.1 Checklist status convention

- `[x]` means implemented and covered by a relevant completed validation lane.
- `[ ]` means incomplete, undecided, or implemented but not yet verified.
- An **implemented, awaiting verification** note records work already present in
  the worktree without prematurely claiming completion.

### 4.2 Current validation snapshot

The latest validated nonvisual state was run through the pinned Nix environment.
`make fmt`, `fmt-check`, `test`, `test-web`, `clippy`, `interaction`,
`oracle-check`, and `package` pass. The most recent full Rust test gate passed
134 tests: 40 CLI, 39 content unit, 5 generic-content, 13 legacy-content, 3
model, and 34 web tests. The dedicated accessibility lane
passes 12 representative routes at desktop, tablet, phone, and 320px narrow
sizes (48 ordinary combinations), plus zoom and forced-colors assertions. The
interaction suite passes all current assertions. Both embedded fonts load in
the standalone browser smoke test.

One combined-gate run exposed a subpixel-only accessibility harness flake: the
full-bleed article title can report `-1.02px` instead of `-1px` at 320px. A
debug rerun passed every route and proved no meaningful document overflow; the
harness now uses a narrowly scoped `1.1px` geometry tolerance rather than
hiding overflow or changing the required gap-free page layout. Repeatability
remains a release-gate requirement.

The current package reports:

- native executable: 6,168,808 bytes;
- WASM: 264,709 bytes gzip;
- generated CSS: 49,475 raw bytes;
- generated site: 7,408,217 logical bytes;
- site JSON: 927,654 bytes;
- site-JSON parsing: 14ms; first browser readiness: 153ms;
- 30 Markdown files, 85 canonical routes, 82 redirect aliases, and 27 content
  assets, plus all 82 content-selected XML feed routes.

`make visual` is intentionally not included in the passing statement: it
remains red at 46 of 68 captures and is the current release blocker. The RMSE
matrix in Section 4 is the authoritative visual result.

Historical progression was 71, 76, 85, 88, 91, and 93 tests as the generic-content,
stable-public-file, presentation/media, font, and semantic-heading tranches
landed, followed by the public/media and syntax/counter tranche. Those counts
are retained only as implementation history; 131 is the current count.

The deletion-independence proof also passed: `./xtra` was moved completely
outside the repository, `make test oracle-check package` passed in the pinned
environment, and the directory was restored afterward. No build, oracle check,
package input, browser startup, or generated manifest depended on `./xtra`.

### 4.3 Worktree implementation ledger

Verified foundations already present include draft pre-filtering, strict
statuses, symlink-safe output validation, the reserved ownership graph,
Goldmark-compatible heading/TOC behavior, parser-event asset rewriting,
structural safe HTML, absolute-origin metadata, semantic fallback HTML, SPA
focus/scroll restoration, core presentation state, content-hash watching,
preview range/MIME/security handling, recoverable output transactions, and
mandatory standalone browser package smoke testing.

Implemented and verified in focused/current gates:

- unknown `site.toml` field rejection, line/column diagnostics, and independent
  empty/one-page/space/Unicode/read-only/symlink content fixtures;
- decorative card thumbnails and blank-alt document images;
- reduced-motion/Save-Data video suppression with a thumbnail-derived poster;
- visual-viewport resize handling, a seekable presentation progress control,
  explicit slide-ID hashes, 44px controls, safe areas, dynamic viewport units,
  and print CSS; and
- embedded GohuFont uni14 with exact-byte, license, manifest,
  `document.fonts.check`, computed-style, and standalone-package assertions as
  the deterministic current face; the former Work Sans fallback was removed;
- Rust `1.94.0` plus wasm-bindgen CLI/crate version pinning in Make and CI.

Implemented and verified in the second visual/interaction round, while their
larger page captures may still fail for unrelated remaining differences:

- inline mobile navigation title flow and accent-backed mobile brand;
- the then-current Work Sans fallback, resume-specific layout/heading/profile rules, exact
  inset article separator, and legible static reduced-motion glitch layers;
- vertical `side-by-side` slide layout, corrected Reveal margin scale,
  invariant deck typography/padding, progress-line placement, and unobtrusive
  presentation utilities;
- homepage intrinsic geometry, inherited author line height, latest-post
  weight, ordinary 404 heading treatment, and document-heading fallback
  shifting.
- centralized public-file specifications shared by content-route validation and
  CLI emission, a stable `/public.asc` PGP link, canonical sibling-thumbnail
  provenance tests, exact card separator borders, and semantic nested folder
  tiles with browser geometry assertions.
- six-level article counters/indentation keyed to preserved source-heading
  classes, plus the complete legacy Chroma palette mapped onto Syntect semantic
  scopes with computed browser color assertions.
- complete `faqe check` generated-reference validation through an isolated,
  automatically removed scratch publication using the exact `write_site` path
  as build; paired valid/invalid reference-graph tests prove agreement without
  committing user output.
- an awaited dynamic-import bootstrap with a host-level polite loading status,
  caught module/WASM startup failures, durable alert plus Retry control, and
  retained readable fallback. Standalone packaged-browser tests prove successful
  status removal and readable error states for missing and corrupt WASM.
- exact Rust-generated 101-frame homepage and 31/21-frame title glitch tracks,
  including reviewed durations, directions, stepped timing, homepage hover
  tracks, controlled motion-enabled frames, and a stable reduced-motion
  fallback;
- a strict content-owned 82-route feed manifest whose root, section, taxonomy,
  and term feeds are derived from current published content and parsed as XML;
- normalized emitted-path ownership checks that reject exact, ancestor,
  descendant, case-folded, generated, public, alias, feed, and fingerprinted
  asset conflicts before output writes; and
- build-time WCAG foreground/background rejection, derived interactive and
  accent-text palette variables, numeric browser contrast assertions, and a
  forced-colors focus/text lane.

Still unresolved:

- component-level typography and geometry tuning against the measured Gohu
  result. Gohu maintained-byte, provenance, package, browser, and full-matrix
  evidence are complete. Daytona/Novecento are historical oracle fonts whose
  commercial bytes remain excluded, not pending deliverables. The complete
  reviewed homepage, resume, menu, folder, clock, and globe icon subset is now
  local Rust-rendered SVG;
- full page/component raster parity remains. Browser print/PDF evidence, exact
  animated glitch frames, ordinary unzoomed 320px/tablet route matrices,
  200%/400% zoom, no-JS/static-crawler coverage, and the complete
  loader/WASM/site-data startup failure/retry matrix already pass;
- dedicated taxonomy-root browser captures are now complete: 16 additive
  legacy-only fixtures, all four root semantics browser-tested, and 14 passing
  comparisons. The missing-thumbnail and taxonomy-root presentation policies
  are implemented. All 82 reviewed feed routes, sitemap `lastmod`, article
  OpenGraph/Twitter metadata, 82 evidence-backed historical redirects, and 30
  explicitly retired stale HTML routes are implemented.

### 4.4 Relevant worktree map

- `crates/faqe-model/src/lib.rs` — serialized site/page/presentation contract.
- `crates/faqe-content/src/lib.rs` — discovery, front matter, routes, headings,
  assets, validation, and deterministic site construction.
- `crates/faqe-content/src/shortcode.rs` — shortcode parsing and typed output.
- `crates/faqe-content/tests/{legacy,generic}.rs` — current-site fidelity and
  independent arbitrary-directory contract tests.
- `crates/faqe-content/tests/fixtures/minimal/` — generic fixture that must not
  depend on this personal website.
- `crates/faqe-web/src/lib.rs` — SPA rendering, navigation, accessibility,
  presentation state, media policy, and browser interactions.
- `crates/faqe-cli/src/main.rs` — commands, generated output, ownership graph,
  transactions, preview HTTP, watching, metadata, feeds, and sitemap.
- `crates/faqe-cli/src/theme.rs` — Rust-owned generated theme; the remaining
  visual parity work belongs here rather than in an external CSS tree.
- `content/site.toml` and adjacent Markdown/media — canonical example content;
  every personal asset must remain content-owned.
- `content/public-files.toml` — strict content-owned stable-public-file manifest
  for `CNAME`, `keybase.txt`, `public.asc`, and `public.txt`.
- `content/feeds.toml` — strict content-owned selection of all 82 reviewed feed
  routes; feed entries are derived from the loaded content model.
- `tests/visual/baseline/` and `tests/oracle/` — immutable compatibility
  evidence; ordinary implementation work must never refresh them.
- `tests/visual/{run.sh,interactions.sh}` — immutable raster comparison plus
  motion-enabled, reduced-motion, responsive, and interaction gates.
- `tests/accessibility.sh` — landmark, naming, focus, numeric contrast,
  forced-colors, and 200%/400% reflow browser evidence.
- `tests/package-smoke.sh` — standalone one-binary build/startup proof.
- `Makefile`, `rust-toolchain.toml`, and `.github/workflows/` — the only
  supported validation/toolchain/CI entry points.
- `README.md`, `COMPATIBILITY.md`, `THIRD_PARTY.md`, and
  `docs/content-contract.md` — public behavior, oracle, licensing, and input
  contract documentation.

### 4.5 Second-round deep visual audit

Three independent read-only comparisons examined source structure, computed
styles, element rectangles, raster differences, content/media provenance, CLI
safety, and the presentation engine. The audit separates real regressions from
legacy-capture artifacts and from intentional accessibility/offline changes.

#### Global and navigation findings

- The legacy browser's default 8px body margin appears in the immutable
  screenshots. It is **not** the target: the user explicitly rejected the
  resulting frame/gap. Keep `body { margin: 0; }` and document this deliberate
  oracle divergence rather than reintroducing it to lower RMSE.
- A block-level navigation title forced the mobile menu onto a second 60px row.
  The title wrapper is now inline and the mobile accent background, foreground,
  and 4px brand padding are restored without shrinking the 44px menu hit
  target. Interaction and no-overflow tests pass; remaining home-phone RMSE is
  not evidence that the menu has regressed.
- Iosevka is exact and locally owned. The user superseded the Daytona/Novecento
  redistribution path by selecting GohuFont uni14 Nerd Font Mono. The
  maintained, safely licensed Gohu base-glyph face is now embedded and
  measured; Work Sans is removed. Do not fall back to Georgia or claim exact
  legacy-font parity.
- Several late CSS overrides contradicted the legacy rules: accent navigation
  titles, light card-title weight, a neutral article-separator guess, and
  resume inheritance. Prefer scoped component
  corrections over another global override layer.
- Exact 767px, 768px, and 769px browser assertions now preserve the legacy
  boundary deliberately. At 767px navigation, headings, estate, and progress
  are fully narrow; at 768px the menu/headings/estate remain narrow while the
  navigation gradient and floated progress label switch on; at 769px all four
  are wide. Navigation/footer hover media queries now begin at the archived
  `min-width: 768px`, while headings, estate, and menu retain
  `max-width: 768px`. Do not collapse these selectors onto one breakpoint.

#### Homepage findings

- Removing `.homesection { width: 100% }` restored intrinsic widths to
  660/660/544/351px versus legacy 658/658/539/336.625px at
  desktop/small/tablet/phone. Avatar heights now differ by less than one pixel
  at every viewport; desktop brand Y differs by about .22px.
- Author line height now inherits, `.latestchild` has local weight 900, and the
  real author string stays opaque under reduced motion with static glitch
  accents. Narrow vertical differences of about 17–18px are dominated by
  intentionally visible local social controls where the oracle's external
  Font Awesome failed.
- The legacy capture shows only `P` from Typed.js and no social glyphs because
  animation timing and external Font Awesome loading were unstable. Keep the
  complete role text and offline/accessibly named local icons; validate their
  geometry with component assertions instead of reproducing missing content.

#### Cards, folders, and content-media findings

- Card data order, values, and content-local image resolution are correct. The
  reviewed Font Awesome glyph outlines are now a minimal maintained
  Rust-rendered SVG subset; no framework or icon-font restoration remains.
- The Docker article's legacy first-thumbnail capture is a broken remote
  Picsum result caused by unavailable network content. The current local image
  beside the Markdown is the correct one-binary/content-owned behavior. Record
  this as an intentional oracle exception and add a semantic component test so
  visual parity never regresses back to a remote URL.
- Folder/taxonomy structure subsequently received semantic local-SVG and nested
  tile polish; the 768px geometry is aligned after restoring `.estate`
  horizontal padding to 5%. The remaining populated-term raster error is
  dominated by the oracle's broken remote first thumbnail, not by a reason to
  reintroduce that URL.
- The progress shortcode uses the legacy escaped-float label/bar mechanics,
  not a normalized flex row. Browser/visual geometry verification is complete;
  desktop RMSE is now 0.077323, narrowly above the 0.075 page threshold.

#### Resume and ordinary pages

- Resume inherited `.estate` 5% padding, compressing desktop content by 72px.
  The corrected contract is `.resume .estate { padding: 2% 0 }` with the
  archived `.cv { max-width: 96rem }` cascade, not a 150rem resume canvas.
- Global heading backgrounds produced solid bars inside the resume. Resume
  headings require transparent backgrounds and no generic heading padding.
- The audit's initial claim that the circular crop was wrong was itself wrong:
  legacy Bootstrap applied `.img-circle`. Keep the circular aspect crop. The
  current scoped rules restore the profile on mobile and retain the legacy
  290px phone canvas padding; remaining work is sidebar/main density and
  section spacing.
- About phone imagery must clear floats and become full width at the legacy
  narrow breakpoint while preserving its alt text and content ownership.
- Article separators and heading styling must be scoped independently from
  cards and resume. Do not use one generic accent/dashed rule for all three.

#### 404 findings

- The giant orange heading override is removed. Size, line height, and color
  now match the ordinary legacy 28/32 desktop and 26/30 tablet/phone treatment;
  natural width is 239px versus legacy 236px.
- The cited measurement was historical Work Sans evidence: its scoped weight
  300 differed from legacy heading 700 and paragraph 900. Gohu normal/400 is
  now shipped and measured in the current matrix; continue component tuning
  without reviving the redesign or rejected body margin.

#### Presentation findings

- The 960x700 logical canvas now applies Reveal's configured 4% margin once.
  Current scale/rectangle matches legacy essentially exactly at all four
  viewports; desktop is x=219.0447, y=15.2201, width=1001.9106,
  height=730.5598 versus legacy 219.0469/15.2188/1001.9063/730.5469.
- Deck text now uses the shipped Gohu face at 40px/52px weight 400; h3 is
  62px/74.4px weight 400, and slide padding remains `20px 0` at every viewport.
  Before the Gohu decision, the historical Work Sans experiment left desktop
  h3 about 16px too high and 10px too tall versus the Daytona oracle; that
  historical result is not evidence about the current Gohu rendering.
- `side-by-side` on the audited title slide is now a verified vertical flex
  stack rather than the accidental two-column grid.
- The progress target remains 44px tall and its independent 2px visual line is
  positioned at `bottom: 2vw`; Reveal-compatible 400/800/1200ms transition
  timing is implemented and browser-tested.
- The O/B/F/? utilities are now unobtrusive until hover/focus-within and remain
  keyboard accessible. Exact Reveal arrow geometry and enabled/faded cosmetics
  are implemented and browser-tested.
- The cyan `talk-small` background is a stale legacy Reveal state: the same
  first slide is gray at desktop, tablet, and phone. Do not reproduce that
  one-capture anomaly.

#### Ranked continuation from this audit

1. [x] Rebuild and validate the inline nav, resume-scoping, fallback-font, and
   side-by-side patches.
2. [x] Patch homepage intrinsic width, inherited author line height, reduced-motion
   foreground text, latest-post weight, and mobile brand styling.
3. [x] Remove the giant 404 treatment and restore ordinary size/color/width.
   Direct browser assertions now cover computed size, color, width, and the
   archived 700/900 requested weights; raster drift from synthesized fallback
   faces remains part of the licensed-font floor.
4. [x] Correct deck margin scaling and its pure math expectations; restore
   invariant 40px/52px typography and `20px 0` section padding.
5. [x] Separate presentation progress visuals from the 44px input target and
   de-emphasize extra utilities without removing keyboard access.
6. [x] Add explicit oracle-exception documentation/tests for the rejected body
   gap, Docker remote thumbnail, blocked legacy socials, and cyan talk-small
   capture.
7. [x] Rerun the current nonvisual gates and replace the RMSE table only with a
   new measured capture. The full `make verify-all` release gate remains open
   while visual fails. Never refresh the immutable baseline from this renderer.

### 4.6 Third-round consolidated audit

The follow-up audit re-read the current plan and implementation from three
independent angles: CLI/safety, content/publication fidelity, and raster/CSS
parity. It did not refresh any oracle. ImageMagick recomputation confirmed the
Section 4 matrix exactly and confirmed the release state remains 11 of 52
captures passing.

#### Status corrections established by code and tests

- Draft filtering, symlink-ancestor output confinement, core route ownership,
  immutable oracles, Goldmark IDs/TOC, `site_url`/`base_url`, semantic route
  fallback, internal content symlink policy, content-hash watching, and the 85
  canonical routes are implemented. Historical defect descriptions must remain
  in past tense rather than masquerading as current work.
- Output-path tests now cover symlink ancestors and leaves, spaces, relative
  parent segments, Unicode names, and executable-parent rejection. Native CI
  coverage for remaining platform-specific path forms is still open.
- `faqe check` now shares content, publication, route ownership, public files,
  route-shell generation, embedded assets, bundle/generated reference checks,
  feeds, sitemap, and manifest construction with build. It renders into a
  process-unique scratch tree whose RAII guard removes both success and error
  paths, so validation commits no user output.
- Successful WASM startup replaces the semantic fallback without duplicate main
  or h1 landmarks. Runtime site-bundle errors and host/pre-WASM module import,
  fetch, compilation, and instantiation failures render retryable alerts while
  preserving readable fallback content. Browser tests cover success, missing
  and corrupt WASM, cache-busted retry recovery, wrong-MIME fallback, and missing
  and corrupt site JSON.
- Browser harness cleanup now waits for preview and ChromeDriver child processes
  to exit after signalling them, preventing sequential Makefile gates from
  racing on a port still owned by the preceding test.
- The root RSS feed has complete channel/item fields, deterministic ordering,
  absolute links, and semantic XML coverage. The absolute 85-route sitemap has
  content-derived page/aggregate `lastmod` values and semantic location-set
  coverage. Fourth-round work restored all 81 non-root legacy feed paths; 82
  evidence-backed HTML aliases are retained and 30 stale HTML routes are
  explicitly retired.
- `/public.asc` is a stable content-selected file. A Markdown link to that
  endpoint must not create a redundant fingerprinted key asset; package output
  currently contains 27 ordinary content assets after this correction.
- All nine canonical post cards have explicit sibling-owned thumbnails and the
  Docker card is deliberately local. The configured content-owned
  `default_card_thumbnail` covers the thumbnail-less talk card without an
  application-embedded fallback.

#### Visual truth and intentional exceptions

- Highest current failures are taxonomy phone `0.283132`, dark-article phone
  `0.227715`, PGP phone `0.222729`, About phone `0.219962`, and light-article
  phone `0.215223`. These are prioritization evidence, not permission to tune toward
  broken or inaccessible legacy behavior.
- The standard visual runner emulates reduced motion and proves only the static
  glitch fallback. The separate motion-enabled interaction lane now proves
  exact keyframe cardinality, hover tracks, durations, directions, pseudo-layer
  content, and controlled homepage/title frames.
- Homepage and title glitch approximations were replaced with the reviewed
  archived sequences: 101 homepage states, 31/21 title states including both
  Sass-loop endpoints, and independent homepage hover tracks.
- `/tags/linux/` is the visual taxonomy fixture. It does not measure taxonomy
  roots. Its phone error is heavily affected by the intentional local Docker
  image replacing the oracle's broken remote Picsum region, plus fallback font
  and glitch differences.
- About phone currently places its image at approximately
  x=26/y=166/339x339 versus legacy x=7/y=185/336x336. Some difference follows
  the explicitly required gap-free body and 5% estate inset, so the desired
  component target must be recorded before changing geometry.
- Progress-page float geometry is restored. Article header/body/disclaimer/TLDR
  geometry is already close. Their remaining page-wide raster error is driven
  mainly by commercial-font metrics, reduced-motion glitch layers, and media
  exceptions rather than a reason for broad global layout overrides.
- The presentation canvas matches legacy essentially pixel-for-pixel, while its
  text boxes remain font-limited. The cyan `talk-small` background, missing
  Font Awesome socials, legacy 8px body gap, and broken Docker thumbnail are
  recorded oracle artifacts or intentional accessibility/offline divergences,
  not targets.

#### Ranked third-round continuation

1. [x] Implement the selected GohuFont uni14 direction with the exact unpatched
   upstream base glyphs. The maintained byte is fingerprinted, licensed,
   manifest-owned, package/browser-tested, and exposed at normal/400 under the
   requested `GohuFont uni14 Nerd Font Mono` CSS alias. The alias does not claim
   Nerd icon coverage; reviewed icons remain local SVG. The full matrix was run
   without refreshing its oracle and records 22 passes/46 failures.
2. [x] Replace coarse glitch approximations with exact legacy keyframe density,
   timing, and homepage hover tracks; add motion-enabled controlled-frame tests.
3. [x] Add 767/768/769 browser assertions for navigation, progress bars, headings,
   and estate geometry, preserving component-specific hybrid selectors.
4. [x] Add taxonomy-root captures for `/tags/`, `/categories/`, `/series/`, and
   `/type/`; keep `/tags/linux/` as the term-page/card target. The additive
   legacy-only tranche contributes 16 fixtures without changing the original
   52 and passes 12 of its 16 comparisons.
5. [x] Identify the actual legacy post-link component before applying archived
   `.content .post a` typography. No `.post` instance exists in the generated
   legacy oracle or the Rust renderer; the live post-index target is the
   already-asserted `.gridlist .relative` card, so the dead archived selector
   is intentionally not copied. Phone 404 heading/paragraph ordering, natural
   width, centering, and ordinary 30/34 + 26/30 geometry are now asserted;
   exact weights remain part of the separately recorded font/licensing floor.
6. [x] Decide the gap-free About mobile inset target and assert it rather than
   optimizing whole-page RMSE. At 390px the target is the zero-margin body plus
   the existing 5% `.estate` and `.6rem` article insets, not the oracle's 8px
   browser frame. The browser gate proves the image clears its float, spans the
   article, precedes the paragraph, and resolves to content-owned `profile_04`.
7. [x] Add component-level assertions for intentional local Docker media and
   the maintained local SVG social/UI icons so broken external oracle resources
   do not dominate parity decisions.
8. [x] Finish Reveal theme isolation and exact arrow cosmetics. The route-owned
   theme now resets leaked article heading padding/backgrounds, paragraph
   alignment, link decoration, and list markers only inside `.faqe-talk`.
   Controls retain 44px hit targets around the archived 5x36px arrow canvas,
   20x5px square-ended bars, 2%/90% edge anchor, 0.9 enabled opacity, 0.3 faded
   back arrows, and full hover/focus opacity. The real
   `Page.printToPDF` proof now passes alongside fragment-aware
   progress/navigation/hashes, timing, notes/presenter behavior, guarded touch,
   print visibility, and zoom/reflow focused tests.
9. [x] Complete check/build emitted-reference agreement through the shared
   scratch publication path.
10. [x] Complete host/bootstrap/browser failure coverage for missing, corrupt,
    wrong-MIME, and recovered WASM plus missing/corrupt site JSON.
11. [x] Complete all 82 reviewed legacy feed routes with content-derived root,
    section, taxonomy, and term semantics. Historical HTML classification,
    sitemap semantic XML coverage, and no-JS/crawler evidence are also complete.

### 4.7 Fourth-round implementation and evidence

This round closed four previously independent gaps without modifying the
immutable visual or semantic baselines:

1. **Motion parity.** Generated theme CSS now reproduces the reviewed homepage,
   title, and hover glitch tables exactly. Unit tests pin frame cardinality and
   browser tests pin timing, direction, pseudo-element content, representative
   paused frames, pointer hover, and reduced-motion fallback behavior.
2. **Feed compatibility.** `content/feeds.toml` owns all 82 historical XML
   paths. Loading is capped at 64 KiB and rejects noncanonical, duplicate, or
   case-folded paths. Feed selection derives solely from current published
   content, taxonomies, sections, and reviewed aliases; empty historical
   families remain valid empty feeds rather than fabricated content.
3. **Ownership hardening.** The CLI claims every emitted filesystem path in one
   normalized ownership graph. Exact, ancestor, descendant, Unicode/case-fold,
   reserved generated, public-file, page, alias, feed, and injected asset
   conflicts are rejected before publication.
4. **Accessible generated palettes.** Shared model code computes WCAG relative
   luminance, rejects page foreground/background contrast below 4.5:1, and
   derives quantization-safe interactive and accent-text colors without
   changing decorative accents. Static shells and SPA navigation use the same
   variables; Chromium verifies numeric contrast, forced colors, and keyboard
   focus across the representative desktop/phone matrix.

Combined validation passed `make fmt fmt-check test clippy interaction
accessibility oracle-check package`. The Rust total is 107 tests. Package smoke
tests prove browser startup, static crawling, JavaScript-disabled fallback,
fallback/hydrated heading and TOC equivalence, and all reviewed runtime failure
states. A subsequent comparison-only `make visual` retained the immutable
oracle and reported the Section 4 result: 11 passes and 41 failures.

### 4.8 Fifth-round implementation and evidence

This round advanced site fidelity and the generic one-binary contract in five
independently tested areas:

1. **Content-owned cards and reviewed listing policy.** `site.toml` now defines
   an optional fingerprinted `default_card_thumbnail`; published post/talk
   cards must provide either a page thumbnail or that default. The audited
   six-item limit applies only to folder-backed sections, ordinary sections and
   taxonomy terms show all members without generating nonexistent pagination,
   and taxonomy roots retain the legacy empty-grid/heading treatment.
2. **Responsive browser coverage.** The accessibility matrix now exercises all
   12 representative routes at 1440px, 768px, 390px, and 320px. It caught and
   fixed a real tablet scrollbar-width overflow in full-bleed article blocks
   using percentage geometry derived from the existing 5% gutters rather than
   hiding overflow.
3. **Conditional preview delivery.** Preview responses emit strong
   content-derived ETags and honor wildcard, list, and weak `If-None-Match`
   validators before Range processing for GET/HEAD. Matching requests return a
   bodyless 304 and changed bytes invalidate the previous validator.
4. **Homepage behavior and icons.** A deterministic state-machine test proves
   the complete Typed.js-compatible role order, typing, smart deletion,
   one-second pauses, and repeated looping. The five social glyphs are a local
   Rust-rendered SVG subset with an embedded OFL notice; the browser verifies
   their order, paths, inherited color, hidden decorative semantics, and legacy
   60%/80% layout. The offline oracle lacks its failed external icon font, so
   the restored phone/tablet icons intentionally increase those raster errors
   rather than reproducing missing deployed chrome.
5. **Targeted visual/component fidelity.** The PGP Chroma surface again exposes
   the reviewed 10% accent tint while retaining rounded local scrolling; PGP
   phone RMSE improved from `0.228538` to `0.222938`. Component assertions pin
   the gap-free About phone inset/flow, PGP surface, real post-card target, and
   ordinary 404 phone geometry without copying the oracle's 8px body frame.
6. **Generic defaults no longer impersonate the example site.** All personal
   title, author, description, roles, footer, menu, socials, palette,
   disclosure, and reference-notice values moved into `content/site.toml`.
   Schema v3 carries the latter content-owned fields to the renderer. A
   directory without `site.toml` receives a neutral `Site`/`Generated with
   FAQE` identity and empty personal fields; generic tests prove its serialized
   metadata contains no `bresilla` value.

Combined validation passed `make fmt fmt-check test test-web clippy interaction
accessibility oracle-check package`; all tests then present passed. Package smoke
reported the current Section 4.2 sizes and confirmed the Font Awesome notice is
present. Comparison-only `make visual` left all immutable baselines unchanged:
11 captures pass and 41 remain above the strict threshold.

### 4.9 Sixth-round implementation and evidence

The detailed legacy scan is now preserved as additive, hash-protected semantic
evidence rather than informal notes or a dependency on `xtra`:

1. **Page and card semantics.** Immutable fixtures record 117 non-pagination
   HTML page rows and 17 distinct card rows, including visible descriptions,
   palettes, taxonomy/series/part labels, rendered dates, and Hugo reading-time
   labels.
2. **Publication formats.** All 82 RSS channels and 176 ordered items are
   captured field-for-field, and the sitemap fixture records all 85 locations
   plus optional `lastmod`, `changefreq`, and `priority`. This comparison found
   and fixed a real generator bug: section sitemap routes now inherit the newest
   dated descendant instead of omitting the legacy aggregate `lastmod`.
3. **Presentation and safe HTML.** Reveal configuration, 13 leaf slides, one
   three-slide vertical wrapper, sorted attributes/background declarations,
   and normalized text digests are preserved. Eight reviewed shortcode output
   families have explicit safe root shapes and typed sanitized-model tests.
4. **Honest exceptions.** Two archived pages' empty foreground variables,
   Reveal's hidden ordinary undertitle/taxonomy metadata, a missing legacy deck
   background image, and feeds for retired taxonomy members remain documented
   evidence rather than broken behavior to resurrect.
5. **Explicit capture boundary.** `semantic-oracle` invokes both maintained
   capture scripts only with `FAQE_LEGACY_SITE_DIR`; neither searches for
   `xtra`, and ordinary build/test/package lanes never invoke either capture.
6. **Icon and package completion.** Menu, clock, folder, and six resume UI marks
   now use the reviewed local Font Awesome SVG subset in addition to the five
   homepage brands. The package gate enforces binary, compressed WASM, raw CSS,
   logical site, site-JSON, parse-time, and first-readiness limits. Only the
   unusually large globe outline remains a text-icon approximation.

The final Rust gate for this round passes 115 tests (35 CLI, 32 content unit, 5
generic, 13 legacy, 3 model, 27 web). `make oracle-check`, `clippy`,
`interaction`, `accessibility`, and `package` pass independently. The latest
comparison-only visual run still has 11 passes and 41 failures; the complete
measured matrix is recorded in Section 4 and no baseline was refreshed.

### 4.10 Seventh-round implementation and evidence

This round converted the latest parallel visual, presentation, and publication
audits into explicit implementation and test contracts. No runtime/build path
references `xtra`, and no immutable semantic or visual oracle was regenerated.

1. **Presentation state is fragment-aware.** `DeckState` now carries the
   fragment step. Explicit `data-fragment-index` values form ordered groups;
   unindexed fragments receive deterministic steps; next/previous and all
   directional navigation reveal or hide fragment groups before crossing a
   slide boundary. Progress, slider seeking, hashes, deep links, ARIA status,
   `visible`, `current-fragment`, and `aria-hidden` all use the same flattened
   step model.
2. **Notes and presenter behavior are owned by Rust.** Reviewed
   `<aside class="notes">` nodes are removed from audience slide content and
   retained as speaker notes. The `S` shortcut opens a live presenter surface
   with the active slide/fragment, notes, and next-slide preview while deck
   navigation remains active. Keyboard modes are mutually exclusive, repeated
   toggles are guarded, Escape consistently exits a modal state, and touch
   navigation rejects multi-touch/control gestures and handles `touchcancel`.
3. **Reveal timing and print rules are explicit and browser-proven.** Fast/default/slow
   transitions use the reviewed 400/800/1200ms semantics. The normal progress
   animation is 800ms. Print media removes controls, presenter UI, notes, and
   decoration; restores every fragment; applies each slide's background; and
   flattens the vertical wrapper into 13 ordered 960x700 leaf pages. The
   headless CDP gate checks exact DOM rectangles, hidden-note and visible-
   fragment markers, three authored background colors, and a 13-page
   720x525.12pt PDF.
4. **Resume component density is measured rather than guessed.** Contact and
   education rows use the reviewed 15px rhythm, exact local icon sizes and
   alignment, 7px inset with the SVG-specific -2px correction, reviewed
   metadata/time/language rules, main/sidebar separators, 30px experience and
   project-intro spacing, project/skill margins, and desktop skill-bar offset.
   Focused RMSE improved from
   `0.131565/0.144300/0.169892/0.0903501` to
   `0.122928/0.134769/0.159036/0.0893143` for
   desktop/small/tablet/phone. Remaining resume drift is principally exact font
   metrics and the wrapping/content-height consequences of those metrics.
5. **The last text icon is gone.** The globe now uses the exact reviewed Font
   Awesome 4.5 `f0ac` outline through the same licensed, Rust-owned SVG subset
   as the other UI marks. Rust and live-browser subset assertions prevent a
   fallback to a glyph font, Unicode approximation, external CSS, or network
   asset.
6. **Image alternatives have a narrow authoring contract.** A missing alt may
   be derived only when a figure has exactly one image and exactly one direct,
   plain-text caption whose normalized text is at most 280 characters. Rich,
   ambiguous, multi-image, and oversized captions are never flattened into an
   alternative. Explicit `alt=""` remains an intentional decorative choice,
   and the image shortcode preserves the difference between omitted and empty
   alt values.
7. **Decorative autoplay policy is explicit.** Authored `[style] autoplay` is
   rejected as an accessibility-policy error. Runtime decorative background
   video makes one explicit `play()` attempt rather than racing a native
   `autoplay` attribute, exposes attempting/playing/fallback states, removes a
   rejected or errored video, and retains the decorative background surface.
   Injected Chromium rejection and media-error tests prove the fallback and
   absence of retry churn. Reduced-motion and Save-Data suppression remain
   verified.
8. **Visual capture owns its animation clock.** Comparison capture pauses every
   animation after readiness, seeks infinite tracks to time zero, finishes
   finite entrance tracks, and waits two animation frames before taking the
   screenshot. This replaces browser scheduling luck with a deterministic
   capture contract and applies equally to an explicit legacy-oracle capture.
   A fresh comparison-only `make visual` validated this capture path; it never
   authorizes updating a baseline.

The Rust gate for that round passed 128 tests (35 CLI, 38 content unit, 5 generic, 13
legacy, 3 model, 34 web). `fmt-check`, `test`, `test-web`, `clippy`,
`interaction`, `accessibility`, `oracle-check`, and `package` all pass through
the pinned Makefile lanes. The comparison-only `make visual` run exercised the
new deterministic animation clock and remained honestly red at 11 passes and
41 failures. No baseline was refreshed. Its only material matrix changes are
the resume scores above; all 48 other values remain those recorded in this
section. Package measurements were subsequently superseded by Section 4.2.

### 4.11 Eighth-round implementation and evidence

This round closed the remaining functional presentation and publication
hardening gaps without weakening the immutable visual gate:

1. **Presentation chrome matches the reviewed Reveal geometry.** Each arrow is
   rendered from two square-ended 20x5 bars inside a 5x36 canvas while the
   enclosing button retains a 44x44 target. Controls use the archived 2% right
   and 90% bottom anchor, unavailable directions are hidden, enabled forward
   directions use `.9` opacity, enabled back directions use `.3`, and
   hover/focus restores full opacity. Scoped resets prevent article heading,
   paragraph, link, and list rules from leaking into slides, and route-exit
   assertions prove presentation rules do not leak back into ordinary pages.
2. **Print is a generated artifact, not a CSS assumption.** The dedicated
   `presentation-print` Makefile target uses Chromium CDP print emulation and
   `Page.printToPDF`, verifies the 13 consecutive 960x700 leaf rectangles,
   flattened vertical stack, visible hidden fragments, absent notes/UI,
   authored slide backgrounds, exact 13 PDF pages at 720x525.12 points, and
   extracted first/last content. Poppler is pinned in the Nix shell and the
   target is part of `verify-all`.
3. **Shortcodes serialize structurally.** Supported output now uses typed
   element, attribute, text, raw nested-Markdown, and sequence nodes. A single
   serializer owns text/attribute escaping, void elements and boolean
   attributes are explicit, invalid slide-attribute names are rejected, and
   the complete legacy semantic/shape suite proves the refactor preserved
   published output.
4. **Mobile text scaling is deterministic without disabling zoom.** Generated
   route shells declare `width=device-width,initial-scale=1,viewport-fit=cover`
   and the Rust-owned theme pins standard and WebKit text adjustment to 100%.
   Chromium mobile metrics plus iPhone and Android user-agent profiles verify
   iPhone portrait/landscape and Android portrait font sizing, orientation,
   readable text, and document containment. Existing 200%/400% user-zoom proof
   remains green.
5. **Native filesystem behavior runs on every release architecture.** All four
   required Linux amd64/arm64 and macOS amd64/arm64 package jobs now run the
   native path/publication test suite before constructing an artifact.
6. **404 weight semantics follow the archived CSS.** The 404 copy restores
   weight 900 and both headings restore weight 700, with a live computed-style
   assertion. This is structurally correct but raised the Work-Sans-fallback
   404 RMSE. Preserve that historical measurement separately from the current
   measured Gohu result rather than treating Daytona redistribution as a blocker.

At the close of this historical eighth-round snapshot, the Rust gate passed
129 tests (35 CLI, 39 content unit, 5 generic, 13 legacy, 3 model, 34 web).
`fmt-check`, `test`, `test-web`, `clippy`, `interaction`, `accessibility`,
`presentation-print`, `oracle-check`, and `package` passed. That round's package
measured 6,099,176 native bytes, 264,699 gzip-compressed WASM bytes, 48,239 CSS
bytes, 7,337,057 logical site bytes, and 926,964 site-JSON bytes; its package
Chromium run measured 33ms JSON parsing and 114ms first readiness. Its visual
result was 11 passes and 41 failures. Talk improved at every viewport; correct
archived 404 weights increased its RMSE; no baseline was refreshed. Section
4.2 and Section 16 contain the current Gohu-era results.

### 4.12 Ninth-round plan consolidation

Three read-only audits rechecked the maintained font pipeline, licensing,
visual diffs, Makefile gates, implementation, and this plan. No new broad CSS
regression was found. The current work is deliberately reduced to these
independent exit items:

1. [x] **Selected typography:** the safely licensed Gohu uni14 base-glyph byte
   is maintained, embedded, fingerprinted, notice-complete, manifest-owned,
   package/browser-loaded, and measured across the full immutable matrix under
   the requested `GohuFont uni14 Nerd Font Mono` alias. Work Sans and its OFL
   file were removed. Clean-checkout proof remains a release-level repository
   state check until these new files are committed.
2. [x] **Fallback weight experiment:** 200+300 and 200+500 were each executed
   through maintained embedding, licensing, package/browser proof, and the
   full then-current immutable matrix. Both regressed and were removed; 200 is
   retained only for that historical pre-Gohu snapshot. The combined case was correctly rejected as unjustified after
   both added faces independently worsened broad parity. This evidence remains
   historical and does not predict or accept Gohu.
3. [ ] **Font-dependent components:** after the font choice, remeasure homepage
   author/latest-post geometry and resume/article typography. Treat these as
   consequences of one font decision, not as independent opportunities for
   viewport-specific geometry hacks.
4. [x] **Taxonomy roots:** dedicated immutable-comparison captures now cover
   `/tags/`, `/categories/`, `/series/`, and `/type/` at all four viewports.
   They were captured only from the reviewed legacy output; all original 52
   hashes remained unchanged. Fourteen of 16 root comparisons pass after
   restoring the root-only static accent fallback. Root/term
   semantics and the archived bare `/type/` prefix are unit/browser-tested;
   `/tags/linux/` remains the distinct populated term/card target.
5. [x] **Structural shortcode parity:** typed block/image/tip/hide wrappers now
   retain the archived inner hooks and palettes. Browser assertions cover
   full-bleed geometry, local media/captions, NOTE/INFO/WARN frames, native
   disclosure keyboard behavior, command/button geometry, every progress
   value, and sidenote/sideimage reveal behavior without adding screenshot
   baselines from the current renderer.
6. [x] **Cross-process publication lock:** Linux retains `/proc` probing and
   macOS/other Unix uses signal-zero liveness with `EPERM` treated as alive and
   only `ESRCH` reclaimable. A real child-process test proves live-lock
   rejection, token preservation, forced-exit orphan recovery, interruption at
   all three rename boundaries, six rounds of six simultaneous contenders,
   complete-tree recovery, and orphan cleanup. These run in all four
   Linux/macOS native package jobs.
7. [ ] **Final evidence:** rerun every Section 21 Makefile lane on the resulting
   tree and collect the four native CI package results. `verify-all` is already
   wired and mandatory; it is currently red because `make visual` honestly
   fails 46 of 68 comparisons, not because a lane is skipped.

The user resolved the font-direction fork in favor of GohuFont uni14. The 43
pre-Gohu visual failures received no automatic blanket waiver, and the shipped
face has now been measured across all 68 immutable captures: 21 pass and 47
fail. Any resulting component-specific thresholds/exceptions must be
documented explicitly. Never update the immutable legacy baseline to make the
decision appear green.

### 4.13 Tenth-round non-font visual re-audit

After the structural-shortcode corrections and additive taxonomy-root oracle,
comparison captures for the complete 68-capture matrix were generated from the
then-current binary with `FAQE_VISUAL_RMSE_MAX=1 make visual` and inspected
against the unchanged legacy baseline at phone and desktop scale. That
historical pre-Gohu matrix measured 25 passes and 43 failures above the strict
`0.075` threshold. Because this audit
found no eligible scoped correction, its before/after matrix is unchanged;
the useful bookends remain home desktop `0.073054` and phone `0.206413`, logo
desktop `0.044381` and phone `0.050038`, populated taxonomy desktop `0.124525`
and phone `0.283132`, and the new taxonomy roots from `0.039582` through
`0.084398`.

The re-audit did not identify another concrete non-font regression that can be
honestly fixed from the reviewed evidence. Clean structural pages align in
their navigation, estate, footer, card/bar, and empty-root geometry; their
remaining diff regions follow text silhouettes and wrapping. The conspicuous
remaining non-text regions are already explicit exceptions: the rejected 8px
body frame, unavailable legacy Font Awesome/Typed.js state, the broken remote
Docker thumbnail reused by the populated-taxonomy oracle, and video/autoplay
capture state on the logo surface. The newly repaired block/image/tip/hide
route is not one of the page-wide raster fixtures, while the matrix's dark
article still exercises command and collapsed-disclaimer styling; focused
interaction assertions remain the truthful evidence for the full shortcode
set. No CSS, renderer, threshold, route, or baseline was changed merely to
reduce RMSE. `make oracle-check` verifies all 68 immutable PNGs and the full
oracle manifest unchanged, and `make fmt-check` remains green.

## 5. Priorities

### P0: correctness, safety, and oracle recovery

- [x] Prevent draft/unpublished content from being serialized or emitted.
- [x] Fix output overlap validation through symlinked ancestors.
- [x] Finish Unicode-normalized and synthetic different-byte fingerprint
  collision handling.
- [x] Extend adversarial case-folded/ancestry coverage around the implemented
  ownership graph.
- [x] Restore immutable visual and semantic legacy fixtures.
- [x] Repair heading loss and published anchor/TOC compatibility.
- [x] Add durable bootstrap loading and startup-failure UI while retaining the
  readable semantic fallback.
- [x] Separate deployment base paths from the absolute production site origin.

### P1: visible parity and public compatibility

- Resolve exact typography, remaining page/component geometry,
  page-specific weights, and the 43 failing visual captures in the expanded
  68-capture matrix. The complete
  reviewed local SVG icon subset, including the globe, is implemented.
  Motion-enabled glitch and homepage social-icon parity are implemented and
  separately browser-proven.
- Presentation theme isolation and exact arrow cosmetics are complete. Core scaling,
  state, backgrounds, transition timing, print/PDF output,
  fragment-aware navigation/progress/hashes, notes/presenter behavior,
  keyboard, guarded touch, zoom/reflow, and accessibility are implemented.
- All 82 reviewed legacy feed routes and 82 redirects/aliases are implemented,
  along with root-feed fields, sitemap fields, article metadata, stable public
  files, and 85 canonical routes.
- Numeric contrast, forced-colors, and 200%/400% zoom coverage are implemented;
  ordinary unzoomed 320px/tablet matrices and iPhone portrait/landscape plus
  Android portrait text-scaling profiles are implemented.

### P2: generalized generator and operational hardening

- Stabilize the implemented arbitrary Markdown directory contract; card
  fallback, alias, and feed configuration are implemented, while sitemap
  selection remains deliberately fixed to canonical routes.
- Adversarial case-fold/ancestry collision coverage and check/build
  emitted-reference agreement are implemented.
- Watcher churn/future/coarse-timestamp stress is implemented. Real
  Linux/macOS process contention, forced-exit stale-lock recovery,
  interruption at every publication rename boundary, and sustained
  multi-process stress are implemented; Windows durability semantics remain
  open. Preview compression is
  explicitly deferred and non-blocking: this binary's server is a local/VPN
  preview surface, while production static hosting may negotiate compression.
  Conditional preview requests are implemented with content-derived ETags.
- Finish optional presentation advanced features; print/PDF proof, release
  checksums, and required standalone package/browser CI are already implemented.

## 6. Phase 0: freeze trustworthy oracles

This phase was completed before the deletion-independence proof and broad visual
changes. Any future oracle expansion must still be captured from the pinned
legacy checkout, never from the current renderer.

### 6.1 Visual oracle

- [x] Capture the reconstructed legacy renderer's actual offline state without
  making legacy assets runtime inputs to `faqe`. The CDN Normalize failure and
  resulting 8px body-margin region are preserved evidence but a documented
  non-target, not proof of the deployed site's Normalize behavior.
- [x] Use the pinned Chromium build and wait for fonts, `image.complete`, and a
  legacy page root. Failed image decode can still satisfy `image.complete`, and
  the current readiness probe detects a Reveal root rather than proving Reveal
  initialization; treat both limitations as oracle artifacts.
- [x] Capture legacy pages separately from current `faqe` output.
- [x] Store immutable compatibility captures in
  `tests/visual/baseline/`, protected by the oracle hash manifest.
- [x] Keep ordinary current-render snapshots under a separate path.
- [x] Remove or rename the dangerous target that refreshes the compatibility
  oracle directly from the current implementation.
- [x] Require explicit approval metadata for intentional oracle changes.
- [x] Assert oracle hashes remain unchanged during `make verify-all`.
- [x] Add deterministic animation-clock capture. Reduced-motion emulation is
  not an animation freeze and does not control the legacy glitch timeline.

Required routes:

- `/`
- `/about/`
- `/post/`
- one long dark post
- one long light post
- `/resume/`
- `/pgp/`
- `/talk/agro/precisionag/`
- one taxonomy term
- `/404.html`

Required viewports and boundary cases:

- 1440x900 desktop
- 1024x768 small desktop
- 768x1024 tablet
- 390x844 phone
- 320px narrow phone
- 560px and 561px sidenote boundary
- 767px, 768px, and 769px layout boundaries
- phone landscape
- 200% and 400% zoom/reflow (browser-emulated and passing for representative
  standard, article, resume, and talk pages)

### 6.2 Semantic oracle

- [x] Record the 85 current canonical routes and the broader legacy route
  evidence. The oracle contains 280 route records: 82 feed paths and 198 HTML,
  alias, or pagination shells; every HTML path now has an explicit disposition.
- [x] Record every heading text, level, generated ID, and TOC nesting.
- [x] Record page titles and canonical paths for the complete legacy output.
- [x] Record exact current post-card order in a maintained golden fixture.
- [x] Record descriptions, theme values, dates, reading time, taxonomies,
  series, and part values. The additive immutable fixtures contain 117
  non-pagination HTML rows and 17 distinct rendered cards. Current post/talk
  tests consume the exact visible metadata where the legacy shell emitted it;
  the Reveal shell's omitted undertitle and two archived empty foreground
  variables remain documented evidence rather than unsafe parity targets.
- [x] Record RSS routes/items/fields and sitemap fields. The detailed oracle
  contains all 82 channels plus 176 ordered items and all 85 sitemap locations
  with optional fields. Semantic build tests preserve the posts-only root-feed
  policy while comparing its nine-item legacy prefix field-for-field; retired
  taxonomy feeds remain explicit valid empty endpoints rather than reviving
  stale content. This audit also found and fixed missing aggregate `lastmod`
  values for section routes.
- [x] Record public static endpoints and their ownership.
- [x] Record presentation slide count, horizontal/vertical structure,
  attributes, text, and active backgrounds. The fixtures preserve Reveal's
  defaults and page JSON, 13 leaf slides, the one three-slide vertical wrapper,
  sorted attributes/background declarations, and normalized text digests.
  Model tests verify the count, vertical ownership, configuration, transition,
  and three color backgrounds; the archived missing image declaration remains
  evidence and is not restored as a broken runtime URL.
- [x] Record the safe subset of rendered HTML and shortcode output. Eight
  reviewed root shapes (`block`, `button`, `command`, `hide`, image/reading
  break, `sideimage`, `sidenote`, and `tip`) are immutable evidence, and tests
  require corresponding typed sanitized document-node ownership rather than
  copying inline legacy script/style output.

### 6.3 Exit criteria

- Compatibility fixtures are independent of the current renderer.
- Normal verification cannot rewrite them.
- Deleting `./xtra` does not affect builds or tests.
- Every later parity patch can be measured against immutable evidence.

## 7. Phase 1: security and publication correctness

### 7.1 Draft filtering

The original implementation serialized route shells and bodies before draft
filtering. Publication filtering now occurs before body parsing, route and asset
derivation, serialization, sitemap, and feed generation. Preview intentionally
uses the same publication graph; no draft-inclusion switch is part of the
current contract.

- [x] Define published statuses strictly.
- [x] Reject unknown status values.
- [x] Filter unpublished pages before route derivation, asset collection,
  serialization, site JSON hashing, shell generation, search/list derivation,
  sitemap generation, and RSS generation.
- [x] Do not add `--include-drafts` unless a future product requirement changes
  the contract. Keeping preview/build/check on the same filtered graph prevents
  a preview-only path from weakening the proven publication invariant.
- [x] Ensure content assets referenced only by drafts are absent from release
  output.

Tests must insert a unique secret into a draft and prove it is absent from:

- generated route files;
- site JSON;
- emitted assets;
- list/taxonomy pages;
- sitemap and feeds;
- browser-visible routes.

### 7.2 Symlink-safe output validation

The original validation canonicalized output only when the complete path
already existed, allowing a symlink ancestor with a missing leaf to bypass the
overlap guard. Nearest-ancestor resolution and pre-replacement rechecks now
close that path.

- [x] Find and canonicalize the nearest existing output ancestor.
- [x] Append unresolved path components to the resolved ancestor.
- [x] Compare the fully resolved candidate against the canonical content root
  and executable path.
- [x] Reject content-inside-output and output-inside-content in all lexical and
  resolved combinations.
- [x] Re-check immediately before replacement to reduce TOCTOU exposure.
- [x] Test symlink ancestors, symlink leaves, and output paths containing
  spaces.
- [x] Test relative `..`, Unicode output paths, and executable-parent output.
- [x] Run the complete native path/publication test suite in every required
  Linux amd64/arm64 and macOS amd64/arm64 package job before constructing the
  artifact, so path behavior is exercised by each native filesystem/runtime.

### 7.3 Reserved route and file namespace

The original writer allowed content such as `/tags/` to be overwritten by a
generated taxonomy shell. The ownership graph now rejects the known fixed,
taxonomy, public-file, page, and case-folded claim families before writing.

- [x] Build one ownership graph before writing any files.
- [x] Reserve generator routes and namespaces:
  - `/`;
  - `/404.html`;
  - `/tags/`, `/categories/`, `/series/`, `/type/`, `/folder/`;
  - every generated taxonomy term;
  - `/assets/`;
  - `/index.xml` and all future feed routes;
  - `/sitemap.xml`;
  - license/attribution outputs;
  - explicit public/static files.
- [x] Reject page/page, page/generated, static/generated, case-folded, and
  public-file ancestry collisions covered by current fixtures.
- [x] Add Unicode-normalization collisions and deliberately injectable
  asset/hash collision fixtures.
- [x] Treat a fingerprint collision with different bytes as a hard error rather
  than silently keeping the first asset.

### 7.4 URL and CSS value validation

- [x] Centralize URL validation for menu items, socials, credits, resume
  contacts/projects, external links, media, buttons, and document attributes.
- [x] Allow only the schemes required by the model, such as `https`, `http`,
  `mailto`, and `tel`.
- [x] Reject `javascript`, `data`, `file`, unexpected custom schemes, malformed
  protocol-relative values, and missing internal menu routes.
- [x] Validate accent/background/foreground values with a strict color grammar
  before inserting them into generated style attributes.
- [x] Keep structural HTML sanitization as a second layer after URL and style
  validation; source-located active-HTML and unsafe-URL tests prove both layers.

## 8. Phase 2: parser, document, and anchor fidelity

### 8.1 Markdown/shortcode boundaries

The original shortcode expansion emitted block HTML without reliably preserving
Markdown block boundaries, which removed published headings from the document
tree and TOC. Block separation, heading preservation, and typed structural
shortcode serialization are now implemented.

- [x] Make shortcode output structurally explicit through typed element,
  attribute, text, raw nested-Markdown, and sequence nodes. One serializer owns
  text/attribute escaping; invalid authored slide-attribute names are rejected.
- [x] Guarantee block separators around block-level shortcode output.
- [x] Parse Markdown destinations using Markdown parser events rather than a
  regex that cannot correctly handle balanced parentheses, escaping, or titles.
- [x] Prevent `---` inside fenced code blocks from becoming talk separators.
- [x] Parse talk attributes with the same quoting/escaping rules as ordinary
  shortcode arguments.
- [x] Convert sanitizer removals into explicit diagnostics when the input asks
  for unsupported semantics.

### 8.2 Goldmark-compatible headings and TOC

Confirmed missing or changed anchors include:

- `standards`
- `usbip`
- `setting-up-the-usbip-server`
- `forging-keys`
- `lock-the-tomb`
- `backing-up-keys`
- `pubickey-backup`
- `subkey-backup`
- `signing-1`
- `openbsd-1`
- `debianubuntu`
- `lxdlxc`
- `copyingmoving-*`

- [x] Match the original Goldmark anchor algorithm, including punctuation,
  duplicate suffix numbering, and Unicode behavior.
- [x] Preserve explicitly requested IDs.
- [x] Ensure shortcode expansion cannot remove surrounding headings.
- [x] Produce the same nested TOC structure and labels.
- [x] Add a golden heading/ID/TOC manifest for every post.
- [x] Confirm redirect anchors are unnecessary: every published heading ID is
  restored exactly by the Goldmark-compatible algorithm or an explicit ID, and
  the all-post golden manifest has no unresolved legacy anchor.

Intentional semantic divergence: the Yew article renderer shifts body heading
levels down (`h1→h2` through `h5→h6`, with `h6→h6`) so the page title remains
the single h1. IDs, text, TOC nesting, and `.faqe-heading-level-N` visual sizes
remain compatibility targets; literal legacy DOM heading levels do not.

- [x] Add explicit no-JS-fallback versus hydrated-Yew equivalence coverage for
  heading text, IDs, visual source-level classes, and TOC order.
- [x] Record and test the deliberate source-h6-to-rendered-h6 collapse policy.

The package browser matrix now extracts normalized heading and TOC signatures
from the JavaScript-disabled semantic fallback and hydrated article, then
requires exact equality. Fallback headings carry the same source-level classes
and nested TOC order; the Rust unit lane explicitly pins source h6 to rendered
h6 while keeping the page title as the only h1.

### 8.3 Safe HTML contract

- [x] Support or explicitly reject strikethrough instead of silently stripping
  `<del>` after enabling it in Markdown.
- [x] Decide support for `caption`, `colgroup`, `col`, `tfoot`, `dl`, `dt`,
  `dd`, and `aside`.
- [x] Preserve safe tables, footnotes, figures, captions, blockquotes, and
  nested list semantics.
- [x] Continue rejecting scripts, inline handlers, unsafe URLs, escaping paths,
  and unsupported active HTML.
- [x] Add document-tree golden tests, not only substring tests.

### 8.4 Reading time and ordering

- [x] Match the legacy/Hugo word-count and rounding algorithm.
- [x] Sort menu and social entries by `weight`, with a deterministic tie-breaker.
- [x] Preserve page/date/card ordering exactly.
- [x] Apply configured site `default_style` as the page inheritance source
  instead of using a fresh built-in default.

## 9. Phase 3: global visual identity

### 9.1 Font decision

The immutable oracle records these historical families:

- Daytona for body/headings;
- Iosevka Term SS01 for navigation/code;
- Novecento Sans Wide for the homepage name;
- a faithful small icon set for social, resume, folder, clock, and menu icons.

The user has selected GohuFont uni14 Nerd Font Mono as the final theme direction
instead of pursuing Daytona/Novecento redistribution. The one-binary model does
not forbid fonts: approved font bytes can be embedded in the binary and emitted
automatically. Users still manage no font files.

- [x] Establish Iosevka provenance under SIL OFL 1.1; retain its license when
  embedding or subsetting it.
- [x] Supersede Daytona/Novecento redistribution as a product deliverable. Their
  commercial files remain excluded. The retained research explains why the old
  bytes must not silently return: vendor licensing pages do not
  establish a right to redistribute the historical repository bytes. The
  inspected Daytona files identify Monotype ownership and prohibit copying or
  distribution except under the governing agreement; Monotype's current
  Daytona and self-hosting guidance requires an appropriate webfont license or
  approved self-hosting kit. Do not copy those historical bytes into the
  binary without written permission. Rechecked 2026-08-03: Monotype's
  [Daytona product page](https://www.monotype.com/fonts/daytona) directs users
  to purchase webfont/desktop licensing, while its
  [self-hosting guidance](https://support.monotype.com/en/articles/7859123-hosting-web-fonts-and-reporting)
  limits downloadable self-hosting kits to approved projects/plans. The
  [Novecento Sans licensing page](https://www.myfonts.com/collections/novecento-font-synthview?tab=licensing)
  states that fonts may not be distributed to others and distinguishes webfont
  from application embedding. A domain/use license is not permission to place
  either historical font in a redistributable generic generator whose users
  can emit it for arbitrary sites. Exact one-binary distribution therefore
  requires explicit written rights covering application embedding,
  redistribution, and emitted browser delivery, not merely proof of a prior
  personal-site web license. This is conservative historical engineering
  context, not legal advice and no longer a release blocker.
- [x] Embed, fingerprint, and automatically emit the exact historical Iosevka
  Term SS01 Light byte from a maintained CLI-owned path; its SHA-256 is
  `d8cac0c88366b5b7f3a7f3ca58108f11c8503bd5637005383226a8011afadb5f`.
- [x] Supersede conditional Daytona/Novecento subsetting. Neither family will be
  embedded under the selected Gohu contract.
- [x] Implement the safely packageable Gohu uni14 candidate. The selected
  source is exact unpatched `gohufont-uni-14.ttf` from
  `koemaeda/gohufont-ttf` commit
  `2e5e68a8a54a127c3f61e7ba43f0e0834bf1bed0`, with upstream WTFPL v2
  `COPYING-LICENSE`. The audited candidate is 120,776 bytes, SHA-256
  `e24c3974ec8c4d3697dafcc5af510a43d2825a7c7251ec0464a7b98f538be0d4`,
  and has internal family/style/PostScript names `GohuFont`, `uni-14`, and
  `GohuFont-uni-14`. Expose it under the requested CSS alias `GohuFont uni14
  Nerd Font Mono` at normal/400 so the renderer never depends on an OS-installed family. This
  alias intentionally provides no Nerd Font or icon-glyph claim; reviewed
  icons remain local SVG. The complete patched Nerd Font must not ship unless
  its combined glyph licenses/notices are independently resolved.
- [x] Apply maintained-source, exact-byte, fingerprint, manifest-owner,
  license/notice, `document.fonts`, computed-style, standalone-package, and
  size-budget gates to Gohu. The 68-capture visual gate was also executed
  honestly and remains red at 46 failures/22 passes; clean-checkout tracking
  proof follows when the new maintained files are committed.
- [x] Remove the historical Work Sans ExtraLight fallback byte and OFL file
  after the Gohu replacement passed its ownership, license, package, browser,
  accessibility, and print gates.
- [x] Resolve fallback weight synthesis deliberately. In the historical
  pre-Gohu A/B experiment, generated CSS embedded only Work Sans ExtraLight at
  weight 200 while components requested 300, 400, 700, and 900, so browsers
  synthesized nearly every rendered weight. That experiment found 200+300 regressed
  from 41 failures/11 passes to 42/10, and 200+500 regressed to 43/9, with
  presentation and 404 among the clear 500 regressions. Because both added
  faces independently worsened the broad matrix, the combined 200+300+500 case
  has no evidence-based justification and was not retained merely to complete
  a Cartesian experiment. The reviewed Work Sans 1.400 candidates were weight
  300 SHA-256
  `88100a10725f93950e15a8482481065d210e28d2d38a8a76e9881726d3c8173c`
  (50,784 bytes), and weight 500 SHA-256
  `0a55e6e9ff18d41cbf8fcf454e5a79cf655427dc6e0626f5484986bff606b908`
  (54,232 bytes). Both were copied only for the isolated experiment, fully
  embedded/fingerprinted/licensed/package-tested, inspected through actual
  `document.fonts` faces and computed weights, measured, then removed. Any
  future retained byte must follow that same pipeline and never be read from
  `./xtra`. The historical maintained 200 byte and its OFL have since been
  removed after Gohu passed the replacement gates. This is an
  engineering/provenance conclusion from the reviewed OFL material, not legal
  advice. It is historical candidate evidence and does not establish Gohu
  acceptance.
- [x] Embed the reviewed Font Awesome social, resume, menu, folder, clock, and globe
  glyph outlines as local Rust-rendered SVG paths instead of restoring a
  font/framework; provenance and the OFL notice are recorded and packaged.
- [x] Add `document.fonts.check()` and computed navigation-font assertions for
  Iosevka.
- [x] Update `THIRD_PARTY.md`, `LICENSES/`, package contents, manifest ownership,
  exact-byte package smoke, and release licensing gates for Iosevka.
- [x] Apply the same ownership, license, exact-byte, browser-load, and package
  gates to Gohu; the superseded Work Sans byte and license are removed.

### 9.2 Global CSS geometry

Restore the legacy structure in Rust-owned theme constants:

- `.container`: `max-width: 90rem`, horizontal `2rem` padding;
- `.estate`: up to `130rem`, horizontal `5%` padding;
- `.content`: flex behavior plus `1.6rem` top and `3.2rem` bottom margins;
- body font size, weight, and `1.8em` line height;
- heading background, padding, exact sizes, and `6.4rem/3.2rem` spacing;
- link underline/fill transitions;
- responsive rules at the original boundaries;
- no browser body frame or hidden overflow used to mask geometry bugs.

Add computed-style and bounding-box assertions rather than relying exclusively
on full screenshots.

### 9.3 Navigation and footer

- [x] Restore the exact Iosevka Term SS01 source byte, declared family/weight,
  navigation/code family use, line heights, and horizontal margins.
- [x] Restore desktop underline-to-accent-fill hover behavior.
- [x] Restore inline mobile title/menu flow, the accent-backed brand treatment,
  and the 44px menu target; browser interaction and overflow assertions pass.
  Exact whole-home phone raster parity remains open for other components.
- [x] Replace the invalid navigation `<label>` with a semantic container.
- [x] Add a labelled primary navigation landmark.
- [x] Set `aria-current="page"`.
- [x] Restore footer size, padding, shadows, line treatment, and hover fill.
- [x] Retain the oracle-confirmed mobile-hidden footer behavior.
- [x] Assert navigation, headings, estate, and progress behavior at 767px,
  768px, and 769px. The browser suite proves the hybrid 768px state: the mobile
  menu, heading sizes, and estate constraint coexist with desktop navigation
  gradients and floated progress labels. The fully narrow and fully wide
  neighboring states are also asserted without modifying visual baselines.

## 10. Phase 4: page and component parity

### 10.1 Homepage

- [x] Restore the natural-aspect 50rem desktop / 30rem phone avatar behavior.
- [x] Remove forced `width: 100%` from `.homesection` so percentage spacing is
  based on the legacy intrinsic width. Home desktop/small now pass; exact
  avatar/brand spacing at tablet/phone remains open.
- [ ] Tune author metrics from the measured shipped Gohu result. The 6em/10vw
  sizing and inherited line height are implemented; remaining work is Gohu
  wrapping/geometry parity, not font integration or Novecento redistribution. This and the latest-post
  item below are one font-integration task, not independent geometry patches.
- [x] Restore Iosevka typewriter styling.
- [x] Preserve the configured legacy role sequence and correct the historical
  `robotist` copy to `Roboticist` in the homepage typewriter:
  1. Plant Hacker
  2. Roboticist
  3. Reverse Engineer
- [x] Test typing, completed strings, deletion, smart common-prefix behavior,
  the 12-tick/approximately-one-second completed-string pause, configured
  order, and repeated infinite looping.
- [x] Restore the social layout as the legacy desktop 60% / phone 80%
  distribution with five faithful local SVG outlines. Rust pins the selected
  path subset and the phone browser lane proves order, paths, inherited color,
  accessibility hiding, and width.
- [ ] Restore exact latest-post dimensions and positioning. Its legacy local
  `900` weight is implemented without globally forcing article text. The latest
  box follows the intrinsic `.homesection` width: current versus legacy is
  660/658px desktop, 660/658px small, 544/539px tablet, and 351/336.625px
  phone. The growing narrow drift follows the 10vw author text's fallback-font
  metrics; do not introduce per-viewport box widths to conceal that unresolved
  font floor.
- [x] Preserve avatar hover/focus/click semantics and keyboard navigation;
  native-link, mouse, focus, Enter, and click behavior are interaction-tested.

### 10.2 Glitch effects

Motion now uses deterministic Rust-generated CSS derived from the reviewed
archived keyframe tables. The ordinary visual runner still forces reduced
motion, so raster RMSE values prove only the fallback; a separate motion-enabled
browser lane proves animation timing, frame density, hover, and controlled
frames without changing immutable baselines.

- [x] Implement a homepage 6-second stepped text-shadow/blur animation.
- [x] Reproduce all 101 legacy per-percent 0–100% homepage keyframes and prove
  the 6-second `steps(100)` track at a deterministic 41% frame in a
  motion-enabled browser.
- [x] Implement title pseudo-layer clipping, skew, and independent 5-second and
  3-second tracks.
- [x] Reproduce the legacy generated 31/21 title clip-state tables, 0.1-second
  base track, 5-second alternate-reverse before track, and 3-second normal after
  track. Unit and controlled-frame browser assertions prove their density and
  endpoint clips.
- [x] Preserve and browser-test the hover-specific homepage variation with its
  independent 3-second alternate-reverse before/after tracks.
- [x] Provide a stable, fully legible reduced-motion fallback that keeps the
  real author text opaque while retaining static pseudo-layer accents.
- [x] Add dedicated browser assertions for the reduced-motion opaque foreground,
  static author accents, and removed ordinary-title pseudo layers.
- [x] Test pseudo-element content, animation names, duration, direction,
  iteration count, frame-table cardinality, clipping, hover, and controlled
  homepage/title frames.
- [x] Keep the motion proof in a no-preference browser context; reduced motion
  is emulated only for the separate fallback assertions.

### 10.3 Cards, lists, folders, and taxonomies

- [x] Restore `auto-fill minmax(250px, 1fr)` and `2em` gap.
- [x] Restore card padding, margins, centering, border radii, and hover shadows.
- [x] Restore fixed 18rem image height rather than generic 16:9 layout.
- [x] Place both badges at the legacy right offset and vertical positions.
- [x] Restore the exact card separator: legacy clears other borders with
  `.gridlist .relative hr { border-style: none }` and applies the dashed top
  border inline. Browser assertions prove that all other borders are absent.
  Title padding matches; current Gohu title-height drift still needs scoped
  component tuning.
- [x] Define the missing-thumbnail ownership contract with a content-owned
  `default_card_thumbnail` setting while requiring every post/talk card to have
  either that fallback or its own page thumbnail.
  Keep an always-present image/fallback box for card geometry; never silently
  embed the old personal logo as an application asset.
- [x] Restore folder breadcrumb/tile presentation using a semantic link around
  the legacy nested visual structure: outer 1% margin, .2% padding, 5px radius,
  accent fill, inner .1em dashed border/.5% padding, floated icon, and 1em title;
  the current browser test proves structure, widths, labels, border, and float.
- [x] Treat taxonomy roots separately from folder tiles: preserve the audited
  empty grid and singular/plural heading treatment, while term pages retain
  card parity inside the legacy `.thelist` wrapper.
- [x] Audit `/tags/`, `/categories/`, `/series/`, and `/type/` separately from
  the `/tags/linux/` term fixture using the archived legacy HTML and template
  source; runtime/build output remains independent of that audit input.
- [x] Preserve the intentional six-item section limit only where it
  matches the legacy template.
- [x] Do not generate new taxonomy/section pagination beyond the retained
  legacy page-1 redirects; render all ordinary-section and taxonomy-term
  members rather than hiding them behind absent pages.
- [x] Add a canonical media-provenance regression test: every current card
  thumbnail must be sibling-relative, resolve inside `content/`, and contain no
  remote `/images` or `xtra` dependency. Pin Docker to
  `post/software/dev/nix/docker-thumbnail.jpg` and its fingerprinted output;
  never chase the oracle's broken Picsum region.

Implemented and verified: the audited Hugo list template used `first $number`
only in its `folders` branch, with `number` defaulting to six; taxonomy pages
instead used the paginator. FAQE therefore limits only folder-enabled sections
and validates explicit `number` values as 1..100. Since the canonical contract
contains no pagination pages beyond retained page-1 redirects (and the only
legacy page-2 shell is intentionally retired), ordinary sections and taxonomy
terms render every current member. The four legacy taxonomy roots each emitted
the singular taxonomy heading plus plural title and an empty `.gridlist`;
`/tags/` alone showed a paginator because its term count exceeded Hugo's 20-item
setting. FAQE retains the empty root presentation but intentionally omits dead
pagination controls, while term grids now restore `.thelist`.

The legacy card template always emitted an image box and silently substituted
the personal `logo_post.png`. FAQE now makes that ownership explicit:
`site.toml.default_card_thumbnail` resolves and fingerprints a content-owned
asset, post/talk pages without either a page image or configured fallback fail
validation, and the binary embeds no fallback logo. Current posts retain their
nine sibling-owned thumbnails; the thumbnail-less Precision Agriculture talk
uses the configured fingerprinted `content/bresilla.svg` fallback. Unit and
legacy integration tests pin effective image ownership, taxonomy-root/term
policy, the folder-only six-card limit, and rejection of invalid limits.

### 10.4 Articles and TOC

- [x] Restore the full-viewport-width article header and declared legacy title
  sizes. Gohu is integrated and measured page-wide; exact title bounding-box
  parity remains a scoped component task.
- [x] Measure and restore `.article-separator` width (60%), 2em vertical
  padding, 1px inset accent border, and light/dark rendering, scoped away from
  card `<hr>` and resume. Computed browser geometry now matches the legacy
  rule; the old neutral solid guess is removed.
- [x] Identify the exact legacy DOM component meant by “post link.” The
  archived `.content .post a` selector is dead in both the reviewed oracle and
  current renderer; the real post-index target is the already-tested
  `.gridlist .relative` card, so no guessed selector was copied globally.
- [x] Restore six-level heading numbering and indentation through
  `.faqe-heading-level-N`, preserving the one-h1 semantic DOM while matching
  legacy counter resets, prefixes, and 1–6% padding.
- [x] Restore nested TOC counters.
- [x] Restore circle-inside article lists.
- [x] Restore inline-code inverted accent treatment.
- [x] Restore the complete legacy syntax color mapping for keywords, builtins,
  functions, variables, strings, numbers, operators, comments, diff tokens,
  prompts, and highlighted lines by translating Chroma categories to current
  Syntect semantic scopes; browser tests prove visible article token colors.
- [x] Restore code-block accent overlay, padding, radius, horizontal scrolling,
  and mobile containment.
- [x] Restore disclaimer, references, and TOC widths, gradients, dashed frames,
  and desktop/phone behavior.

### 10.5 Shortcodes

Preserve every currently used shortcode and every accepted legacy argument.

#### `command`

- width, type, radius, color, and font;
- prompt presentation;
- accent overlay and positioning.

#### `block` and `note`

- full-bleed holder;
- default 48% width and 90% mobile width;
- type/color/font/radius behavior;
- filled and outline variants.

#### `tip`

- WARN tomato;
- INFO greenyellow;
- NOTE turquoise;
- split background gradient;
- dashed inner border;
- configurable width and radius.

#### `hide`

- native disclosure semantics;
- default 40% / mobile 90% width;
- 3.5em split gradient;
- dashed inner frame and title spacing.

#### `image`

- full-bleed holder;
- centered configured width;
- radius, border, padding, color, shadow, and cover behavior;
- caption geometry;
- mandatory accessible alternative or explicit decorative flag.

#### `button`

- legacy 250x50 dimensions;
- two-layer hover scaling and borders;
- preserved focus treatment and safe URL validation.

#### `hr` reading break

- reserved vertical geometry;
- dashed inactive state;
- hover percentage/read-time labels;
- active/reset behavior;
- decide whether continuous scroll progress or exact legacy hover behavior is
  the authoritative interaction.

#### `sidenote` and `sideimage`

- desktop float-right, width, negative margin, numbering, and line height;
- phone hidden state and checked 95% reveal;
- visible keyboard focus and correct label/control semantics;
- figure/caption desktop and phone geometry.

Focused structural-shortcode coverage is now complete without adding or
refreshing raster fixtures. The pinned interaction browser verifies command
prompt width, padding, accent overlay, opacity, and radius; phone full-bleed holder and 90%
inner geometry for image, filled block, NOTE/INFO/WARN tips, and hide; the
fingerprinted caption-derived image alternative; the three archived tip
colors, split gradients, and dashed frames; and hide's collapsed state, dashed
inner frame, spacing, and keyboard-open behavior. It also proves associated
sidenote/sideimage controls, their hidden phone state, keyboard reveal, 95%
sidenote geometry, local decorative side-image ownership, and the populated
taxonomy-term control case.

That audit exposed three concrete renderer mismatches which are corrected:
filled blocks now use the archived accent/background inversion; tip variants
now restore turquoise/greenyellow/tomato split surfaces with a real dashed
inner frame and 2em content offset; and native hide disclosures now retain the
legacy dashed wrapper, title styling, and 2em revealed-content offset. Image
figures now emit the existing `.imagetextframe` caption hook and their local
media fills the rounded configured frame. These are element-level compatibility
assertions derived from the reviewed shortcode oracle, not new page-wide
baseline exceptions.

### 10.6 About, PGP, resume, and 404

- [x] Finish About image alignment and content width. Responsive full-width
  clearing and content ownership are implemented. The apparent phone mismatch
  (current x=26/y=166/339px versus legacy x=7/y=185/336px) includes the rejected
  oracle body frame and fallback-font vertical drift. The accepted gap-free
  target retains the 5% estate plus `.6rem` article inset; deterministic browser
  assertions cover image width/float/source and paragraph ordering.
- [x] Keep PGP key bytes content-owned and emit `/public.asc` as a stable,
  content-selected compatibility endpoint.
- [x] Restore the intended 10% accent tint on fenced Chroma surfaces. The legacy
  negative-z pseudo overlay was hidden beneath the explicit black `<pre>`
  surface; compositing the same accent/background colors directly on
  `<pre>` preserves the reviewed appearance without copied CSS or runtime oracle
  dependency. Phone PGP RMSE improved from `0.228538` to `0.222938`; the browser
  gate asserts the full-width rounded box and its local horizontal scroller.
- [ ] Tune remaining resume typography and wrapping from the measured Gohu result. Skill
  fill animation is implemented and live-browser verified. All seven
  instantiated non-brand resume icons (envelope, phone,
  globe, user, briefcase, archive, and rocket) now use the exact reviewed local
  Font Awesome 4.5 SVG paths with browser and Rust assertions. The latest
  focused capture is desktop/small/tablet/phone
  `0.122928/0.134769/0.159036/0.0893143`, improved from
  `0.131565/0.144300/0.169892/0.0903501`. Core geometry and density are
  complete: 960px CV width, 270px absolute sidebar through 768px,
  50/300/50/50 main padding, transparent headings, circular 210px
  desktop/tablet and 290px phone profile, reviewed contact/education rhythm,
  separators, experience/project spacing, skill margins, and mobile visibility
  all match. Remaining drift is primarily exact font metrics, wrapping, and
  resulting content height. Do not add viewport-specific spacing patches to
  conceal this dependency; resume closes only after its current Gohu rendering
  has explicit component-level parity or a documented exception.
- [x] Remove nested `<main>` from resume; interaction/accessibility lanes prove
  the shell owns exactly one main landmark.
- [x] Restore ordinary legacy 404 heading hierarchy instead of the giant
  redesigned accent heading. Direct browser assertions pin the archived
  h1/h2 weight 700 and paragraph weight 900 as well as size, color, and width.
  Desktop/tablet captures passed in the historical Work Sans snapshot;
  small/phone drift there used synthesized weights from Work Sans 200. The
  current Gohu matrix supersedes that snapshot and remains the tuning input.

## 11. Phase 5: Rust presentation engine

The functional Rust deck now matches the core Reveal canvas, backgrounds,
navigation, guarded swipes, fragment-aware hashes/progress, overview, pause,
fullscreen, notes, presenter behavior, timing, and zoom/reflow used by this
site. Remaining raster parity is font-limited text metrics; real print/PDF proof
is complete. Focused post-isolation captures are desktop/small/tablet/phone
`0.0990902/0.157754/0.0891492/0.0656596`, improving all four recorded values.

### 11.1 Typed model

Introduce explicit types instead of leaving presentation settings in opaque
front matter:

```text
DeckConfig
DeckState { horizontal, vertical, fragment, overview, paused }
SlideGroup
Slide
SlideBackground
Transition
TransitionSpeed
Fragment
SpeakerNotes
```

Parse supported `[reveal_hugo]` fields into `DeckConfig`, validate unknown or
unsupported options, and preserve current slide attributes.

### 11.2 Required parity for the existing deck

- [x] Preserve a logical 960x700 coordinate system.
- [x] Compute scale from viewport, configured margin, and min/max scale. Reveal
  removes the configured margin once in total; use
  `viewport * (1 - margin)`, not `viewport * (1 - 2 * margin)`.
- [x] Recompute scale on resize and visual viewport changes, with invalid
  visual-viewport values falling back to the layout viewport.
- [x] Render a separate full-viewport background layer for each active slide.
- [x] Support background color and image with appropriate fading.
- [x] Scope the presentation theme so base article heading backgrounds/padding,
  paragraph alignment, link decoration, and list-marker defaults do not leak;
  the mode stylesheet is removed on route exit and browser assertions prove it.
- [x] Preserve the geometry established during the historical Work Sans round:
  40px/52px body text, 62px/74.4px h3, weight 400, and invariant `20px 0` slide
  padding. The shipped Gohu normal/400 face now uses those rules and is included
  in the current immutable matrix.
- [x] Honor default slide transition plus `none`, `fade`, `slide`, and `zoom`.
- [x] Honor `fast`, default, and `slow` speeds.
- [x] Make `class="side-by-side"` a legacy vertical flex stack rather than the
  accidental two-column grid; source and browser interaction assertions cover
  the corrected layout.
- [x] Remember the last vertical index for every horizontal stack.
- [x] Implement accurate next/previous, PageUp/PageDown, Shift+Space, Home, End,
  arrows, N/P, and H/J/K/L behavior.
- [x] Ignore navigation keys from editable controls, interactive elements,
  modifier chords, and scrollable code blocks.
- [x] Add four-direction swipe navigation with a documented threshold.
- [x] Finish exact arrow-control geometry and enabled/faded state while
  retaining minimum 44px touch targets. The O/B/F/? utilities are now hidden
  until hover/focus-within while remaining keyboard accessible.
- [x] Restore the progress control's slider semantics, seeking, 44px target,
  and separate 2px visual line at `bottom: 2vw`.
- [x] Use 400/800/1200ms fast/default/slow transition timing and make
  navigation, hashes, slider status, and progress fragment-aware.
- [x] Mark inactive slides hidden and `aria-hidden`.
- [x] Announce the active slide through a polite live region.
- [x] Keep presentation content readable at 200% and 400% zoom. Browser-level
  device-metric emulation proves the active slide text plus controls/progress
  remain visible and contained at effective 640px and 320px layout widths.

Implemented and focused-test verified: scale calculation uses the corrected
single-margin formula, prefers `window.visualViewport`, and subscribes to both
visual-viewport and window resize. The progress element is focusable and
seekable, maps pointer position to slide position, separates its 2px visual
from its 44px target, and keeps utility controls unobtrusive. Whole-deck raster
parity remains open; exact arrow cosmetics are browser-proven. `make presentation-print` proves
the complete 13-leaf print layout and generated PDF independently of raster
baselines.

### 11.3 General Reveal-compatible capabilities

Implement after the current deck is correct, in this order:

1. [x] initial hash/deep-link reading for horizontal and vertical positions;
   [x] resolve explicit slide-ID deep links with percent decoding, duplicate-ID
   rejection, and deterministic numeric fallback IDs;
2. [x] overview mode and clickable 2D slide map;
3. [x] pause/blackout;
4. [x] fullscreen and keyboard help;
5. [x] zoom/reflow behavior for this deck;
6. [x] ordered/grouped fragments and fragment-aware progress/navigation;
7. [x] speaker notes preservation and presenter view;
8. [x] print/PDF layout that renders all horizontal and vertical slides;
9. generic config for center, controls, progress, history, transition, and
   background options.

### 11.4 Presentation tests

- exact scale math at every viewport;
- active colored background fills the viewport;
- first-slide zoom/fast transition;
- horizontal/vertical remembered position;
- PageUp/Shift+Space previous semantics;
- swipe thresholds and directions;
- initial hash selection;
- progress seeking;
- current/non-current ARIA state and live announcement;
- target guards for code/links/controls;
- overview, pause, fullscreen, fragment, notes, and print behavior, including
  exact 13-page CDP `Page.printToPDF` output;
- reduced-motion transition removal.

## 12. Phase 6: generated HTML, accessibility, and startup resilience

### 12.1 Semantic fallback HTML

The route shell now contains route-specific semantic fallback content rather
than an empty WASM mount, and successful startup replaces it without duplicate
landmarks. Pre-WASM failures retain that readable fallback beside a durable host
alert. No-JS/crawler evidence and the complete failure/retry browser matrix
pass.

- [x] Generate route-specific semantic fallback HTML from the typed document.
- [x] Include navigation, one main landmark, one descriptive h1, article text,
  and internal links before JavaScript runs.
- [x] Replace the fallback after successful WASM startup without duplicate
  accessible content; mounted-route accessibility and package startup prove one
  main and one h1 remain.
- [x] Keep replace-after-startup rather than claiming true hydration. Package
  tests prove the generated fallback before JavaScript, one fallback main/h1 in
  every shell, and exactly one mounted main/h1 after successful replacement;
  no hydration-only transition contract is required.
- [x] Keep the fallback generated by the binary; users manage no HTML.
- [x] Add a JavaScript-disabled Chromium snapshot for a deep article and static
  crawler assertions across every generated HTML shell. Each shell has exactly
  one fallback main/h1, named navigation, and a hidden host status without
  falsely reporting WASM readiness.

### 12.2 Bootstrap lifecycle

- [x] Render a generated host-level loading element with `role="status"` and
  polite `aria-live`; keep it hidden when JavaScript is disabled and remove it
  after successful startup.
- [x] Dynamically import the WASM loader and await WASM initialization.
- [x] Catch post-WASM bundle HTTP, filename/digest, JSON, and schema failures
  and render a `role="alert"` runtime view with Retry.
- [x] Catch pre-WASM loader import, WASM fetch, compilation, and instantiation
  rejection in the generated host bootstrap.
- [x] Replace host loading UI with a durable `role="alert"` message and Retry
  action while leaving the semantic page readable.
- [x] Browser-test successful status removal plus readable missing/corrupt-WASM
  failure states in the standalone packaged binary.
- [x] Browser-test cache-busted retry-click recovery, wrong-MIME WASM fallback,
  and missing/corrupt site JSON error states.

### 12.3 Landmarks, headings, and SPA focus

- [x] Guarantee exactly one `<main>` per route.
- [x] Guarantee one descriptive `<h1>` per route, visually hidden where exact
  layout requires it.
- [x] Add a skip-to-content link.
- [x] Focus the new main/h1 after SPA push navigation.
- [x] Define back/forward focus behavior alongside scroll restoration.
- [x] Announce route-title changes in a polite live region.
- [x] Add `aria-current="page"` to current navigation.
- [x] Make loading/error/missing-data views valid landmarks and alerts.

### 12.4 Images, media, motion, and focus

- [x] Preserve non-empty informative alternatives and normalize missing/blank
  alternatives to explicit decorative semantics.
- [x] Derive a missing alt only from one direct, unambiguous plain-text caption
  on a one-image figure, normalize whitespace, cap the result at 280
  characters, and preserve explicit `alt=""` as decorative author intent.
- [x] Avoid duplicate card thumbnail/title announcements.
- [x] Add visible focus for links, buttons, summaries, sidenote controls, and
  keyboard-scrollable code blocks.
- [x] Stop or omit background video under reduced motion.
- [x] Add `playsinline`, decorative semantics, and a poster/fallback.
- [x] Reject authored background autoplay and prove runtime decorative
  `play()` rejection/media-error fallback, removal of the failed video,
  preservation of the background surface, and absence of retry churn.
- [x] Honor `Save-Data` before loading decorative video.

Implemented and verified: linked card thumbnails use empty alt text and
`aria-hidden`; document images preserve non-empty alternatives, safely derive
only narrowly unambiguous caption alternatives, and preserve explicit
decorative intent. Background video uses `preload="metadata"`, a
thumbnail-derived poster/fallback and suppresses loading and playback for
reduced motion or Save-Data. Runtime rejection and media-error fallback are
browser-proven. The remaining media decision is whether poster metadata should
be explicit rather than thumbnail-derived.

### 12.5 Contrast and responsive browser behavior

- [x] Validate page foreground/background combinations at build time and reject
  normal-text contrast below 4.5:1.
- [x] Meet 4.5:1 normal text and at least 3:1 UI/focus boundaries through
  generated, browser-verified palette variables.
- [x] Derive accessible link, focus, and badge colors separately from purely
  decorative accents when necessary.
- [x] Support dynamic viewport units with safe `vh` fallback.
- [x] Respect safe-area insets and `viewport-fit=cover` where enabled.
- [x] Test forced-colors rendering, distinguishable text, and keyboard focus in
  Chromium.
- [x] Test iPhone portrait/landscape and Android portrait text scaling through
  mobile device-metric and user-agent emulation. Generated shells use
  `width=device-width,initial-scale=1,viewport-fit=cover`, the Rust-owned theme
  pins `text-size-adjust`/`-webkit-text-size-adjust` to 100%, user zoom remains
  enabled, orientation is asserted, and representative text/reflow stays
  contained. Reduced motion and reduced data retain dedicated browser/unit
  coverage.
- [x] Test 200%/400% zoom-equivalent reflow in Chromium for representative
  standard, article, resume, and presentation pages. Assertions cover readable
  text boxes, document containment, locally scrollable wide code/tables, and
  presentation controls/progress inside the visual viewport.
- [x] Add print CSS for standard pages, resume, long URLs/code, backgrounds,
  navigation suppression, and presentation pages. `make presentation-print`
  uses headless Chromium `Page.printToPDF`, verifies 13 consecutive 960x700 CSS
  pixel slide boxes (including the flattened vertical stack), confirms all
  fragments print and speaker notes do not, and checks the exact 13-page
  720x525.12pt PDF plus extracted first/last content markers.

Build validation calculates WCAG relative luminance for every page style and
rejects foreground/background pairs below 4.5:1. Decorative accents remain
unchanged for parity, while a shared model utility derives quantization-safe
`--interactive-color` and `--accent-text-color` values. Static shells and SPA
navigation apply the same palette. The browser accessibility matrix verifies
all representative routes at desktop and phone sizes, including forced colors.

## 13. Phase 7: SEO, feeds, routes, and public files

### 13.1 Public origin and deployment path

Separate:

```text
site_url = "https://bresilla.com"
base_url = "/optional/subpath/"
```

- [x] Join origin, base path, and route in one tested URL builder.
- [x] Emit absolute canonical URLs.
- [x] Emit absolute `og:url`, `og:image`, RSS links/guid, and sitemap locs.
- [x] Avoid double base paths and duplicate slashes.
- [x] Keep local preview functional without pretending relative paths are valid
  production canonical URLs.

### 13.2 Page metadata

- [x] Use the site description when a page description is missing or empty,
  before WASM runs.
- [x] Preserve explicit/missing-title semantics for Logo, PGP, and front pages.
- [x] Add article-specific OpenGraph type, publication time, tags, and image
  metadata.
- [x] Add Twitter image and image-alt metadata.
- [x] Update all relevant metadata during SPA navigation if SPA state remains
  authoritative.
- [x] Do not generate Person/BlogPosting JSON-LD in this version. Existing
  canonical, OpenGraph, Twitter, feed, and sitemap metadata is complete; avoid
  inventing personal/schema claims that are not content-authored.
- [x] Test metadata by parsing every generated shell, not substring counts.

Implemented and verified: article shells emit `og:type=article`, normalized
UTC publication timestamps, repeated `article:tag` values, absolute thumbnail
metadata, and non-empty OpenGraph/Twitter image alternatives. Non-article
shells reset to `og:type=website` without stale article fields. The deterministic
build test tokenizes all 85 route shells plus `404.html` with `html5ever` and
asserts the structural head, canonical link, core metadata uniqueness, article
field presence/absence, and exact publication/tag values. SPA navigation
creates, updates, and removes the same route-specific fields.
Front matter records whether a title was authored. Explicit titles remain
visible; derived accessibility titles for untitled Logo/PGP/front pages are
visually hidden in both the semantic fallback and hydrated Yew tree while
remaining the single page h1.

### 13.3 RSS

- [x] Emit an ordered root `/index.xml` feed of published posts with absolute
  channel/item links and GUIDs.
- [x] Retain all 82 feed routes recorded by the immutable legacy route oracle.
- [x] Restore the selected section, taxonomy, and term feeds through the
  content-owned feed contract.
- [x] Complete title, pubDate, description/summary, language, lastBuildDate, and
  Atom self-link fields.
- [x] Preserve deterministic date/route ordering and XML escaping in the root
  feed.
- [x] Parse the root feed as XML and compare its ordered item titles and required
  field structure against the loaded content model.
- [x] Add XML parser/golden tests for every additional feed family selected by
  the compatibility contract.

Implemented and verified: `content/feeds.toml` explicitly owns the exact 82
audited feed paths without any runtime reference to `xtra`. The loader enforces
a 64 KiB manifest limit, canonical absolute `index.xml` paths, case-folded
uniqueness, and retention of the root feed. Root items remain the ordered
published posts; taxonomy and term items derive from the current taxonomy
model; section feeds derive direct published children plus canonical targets of
evidenced historical aliases. Stale audited families with no current published
members remain valid empty feeds rather than inventing content. Every feed uses
absolute base-aware channel, self, item, and GUID URLs; deterministic
date/route ordering; validated dates; and XML escaping. The deterministic build
test parses every feed as XML, checks channel/item structure and absolute URLs,
and compares ordered item titles with the loaded content model. The legacy
integration test proves the content-owned set equals all 82 immutable oracle
feed routes. Feed routes are recorded separately in the build manifest and do
not change the 85 canonical HTML-route or sitemap contracts.

### 13.4 Sitemap

- [x] Preserve the reviewed 85 canonical routes and emit one valid absolute
  sitemap location for each; legacy-content and deterministic CLI tests pin the
  exact count and set.
- [x] Restore available per-page and taxonomy/home aggregate `lastmod` values
  from validated content dates.
- [x] Keep aliases and pagination redirects out of the sitemap; the sitemap is
  the 85-route canonical discovery surface and redirect shells canonicalize to
  those entries.
- [x] Validate well-formed XML/escaping and compare the complete semantic
  location set, not only route count.

### 13.5 Stable public files and aliases

- [x] Define a content-owned public/static manifest with traversal and collision
  validation.
- [x] Retain `/CNAME`, `/keybase.txt`, `/public.txt`, and `/public.asc` as
  direct compatibility outputs selected by content.
- [x] Keep stable public-file links at their declared direct paths instead of
  fingerprinting them merely because Markdown links to them. Independently
  referenced ordinary content assets remain fingerprinted.
- [x] Inventory historical `/posts/*`, `/post/linux/*`, `/post/dev/*`,
  `/post/crypto/*`, page-1 aliases, and taxonomy pagination.
- [x] Classify each as retained redirect, canonical route, or intentionally
  retired path.
- [x] Add redirect/alias fixtures so the decision is explicit.

The immutable legacy-route oracle contains 280 historical outputs: 198 HTML
routes and 82 feeds. The HTML contract is now exhaustive: 85 canonical routes,
82 retained redirects, 30 explicitly retired stale routes, and the generated
`404.html`. Of the redirects, 67 are legacy page-1 shells whose emitted Hugo
metadata explicitly refreshed to a still-canonical parent; 15 are duplicate
article shells with a unique current slug/title equivalent. `content/aliases.toml`
owns those mappings without referring to `xtra`; sources and targets must be
canonical directory routes, targets must exist, and sources must not collide
with canonical/generated/public outputs. Redirect shells are base-aware,
`noindex`, canonicalized, readable without JavaScript, and never bootstrap
WASM. The build manifest records aliases separately so its `routes` map remains
the 85 canonical routes, and aliases remain deliberately absent from sitemap.

The feed inventory is also explicit: `content/feeds.toml` selects `/index.xml`
and all 81 additional reviewed historical feed paths. Each is emitted as XML
with content-derived semantics; no HTML redirect pretends to be a feed.

The strict public-file manifest confines normalized relative source/target paths,
canonicalizes sources, requires regular files, enforces per-file/aggregate
limits, detects case-folded/path-ancestry/generated/fingerprinted collisions,
sorts output deterministically, and records content-public ownership in the
build manifest. Oracle SHA-256 and standalone package tests prove exact output
for all four stable paths without an `xtra` dependency.

## 14. Phase 8: CLI and generator robustness

### 14.1 `check` and `build` agreement

`faqe check` and `faqe build` now share content loading, public-file specs, route
ownership, full site rendering, and bundle/generated-reference validation.
Check publishes to a process-unique scratch tree that is automatically removed
instead of committing an output directory.

- [x] Share content loading, publication, route ownership, and public-file
  validation between check and build.
- [x] Run emitted/generated-reference validation in check through the exact
  build renderer using a no-commit scratch output model.
- [x] Treat missing internal routes, menu links, resume links, talk media,
  content assets, and generated references consistently through the shared
  renderer and validators.
- [x] Reserve warnings for non-fatal conditions; check no longer reports success
  for a generated reference graph that build rejects.

The paired regression tests reject the same unresolved route from check and
build, accept the same valid reference graph, and prove scratch workspaces are
removed on drop. The standalone packaged binary also runs `faqe check` before
building and starting the generated site.

### 14.2 Output transaction

- [x] Document and implement a recoverable two-phase replacement rather than
  claiming an impossible fully crash-atomic directory swap.
- [x] Add recovery for orphaned `.faqe-backup-*` and `.faqe-tmp-*` directories.
- [x] Add fault injection between live-to-backup and temporary-to-live renames.
- [x] Prevent concurrent builds to the same output with a scoped lock.
- [x] Fsync the publication parent where durable replacement is promised.
- [x] Use exchange rename where supported or provide a recoverable two-phase
  protocol.

The transaction tests cover scoped locks, real Linux/macOS child-process lock
contention, forced-exit stale-lock reclamation, last-known-good recovery,
complete first-build promotion, rollback before install, recovery after
install, and injected failures before archive, before install, and after
install. Test-only, boundary-synchronized child processes are forcibly stopped
before archive, after archive, and after install; recovery proves a complete
old/new tree, removes temporary/backup/lock orphans, and survives six rounds of
six simultaneous contenders without replacing a live lock. Windows durability
semantics remain future platform work.

### 14.3 Watch mode

The watcher no longer tracks only the maximum modification time; it uses a
sorted content-hash snapshot. Adversarial future/coarse timestamp and rapid
edit/rename/delete churn are covered without sleep-based timing assumptions.

- [x] Track a sorted snapshot of path, type, size, and mtime, or content hashes.
- [x] Detect additions, removals, renames, content changes, and referenced asset
  target changes.
- [x] Handle internal asset symlinks deliberately: confine target bytes to the
  content root, reject escaping links, avoid duplicate Markdown discovery, and
  include internal target bytes in watcher snapshots.
- [x] Debounce batches without missing changes.
- [x] Test same-size content changes, renames, deletions, and invalid-to-valid
  recovery.
- [x] Test future timestamps, coarse timestamps, and rapid edit/rename/delete
  stress without relying on sleeps or timestamp advancement.

### 14.4 Preview HTTP server

Preview is not intended to replace a production web server, but it must be
reliable enough for development and VPN review.

- [x] Default safely to `127.0.0.1:3000` and allow explicit VPN review with
  `faqe serve CONTENT_DIR --bind 0.0.0.0:3000`.
- [x] Read until the complete bounded HTTP header is received rather than one
  TCP read.
- [x] Return 416 for unsatisfiable byte ranges.
- [x] Preserve valid single-range MP4 support.
- [x] Add MIME types for WebP, AVIF, GIF, ICO, WebM, OGG, MP3, PDF, and text.
- [x] Add write timeouts and bounded request handling.
- [x] Add content-derived ETag/If-None-Match handling for preview GET/HEAD and
  range requests, including weak/list validators and byte-change invalidation.
- [x] Keep preview responses uncompressed for now. The server is explicitly a
  local/VPN preview surface, conditional ETags are implemented, and the package
  readiness/size budgets do not justify adding compression negotiation to the
  non-production server.
- [x] Send enforceable CSP/security headers from preview responses.
- [x] Remove ineffective `frame-ancestors` from meta CSP or document required
  production headers.

## 15. Phase 9: generic content-directory contract

The core generic-directory contract was implemented early and must remain
stable while site-specific parity continues. Proposed optional fields below are
not part of the implemented schema until explicitly checked.

### 15.1 Required files and discovery

- [x] Recursive Markdown discovery.
- [x] Clear `_index.md` section semantics.
- [x] Optional `site.toml` with safe defaults.
- [x] Content-local assets resolved relative to each Markdown file.
- [x] Explicit public/static file declaration, validation, ownership, and
  emission through `content/public-files.toml` and package exact-byte checks.
- [x] Document content and asset size limits: 8 MiB per Markdown file, 64 MiB
  aggregate Markdown, 64 KiB `site.toml`, 32 MiB per asset, and 128 MiB
  aggregate assets.
- [x] UTF-8 content plus tested space/Unicode path handling.
- [x] A confined symlink policy with internal and escaping cases tested.
- [x] Deterministic discovery and ordering.

### 15.2 Configuration

Document and validate:

- site title, author, description, keywords, public origin, and base path;
- menu entries and weights;
- social entries, icons, labels, URLs, and weights;
- avatar, hover avatar, favicon, and default footer;
- default theme plus per-page inheritance;
- publication/draft rules;
- route, slug, date, taxonomies, series, and part;
- resume schema;
- presentation configuration;
- public-file options;
- default card thumbnail, aliases, and feed-route selection.

Implemented: generic `default_card_thumbnail`, alias, public-file, and feed
selection contracts. Sitemap selection is intentionally not configurable:
only canonical HTML routes belong in it, never aliases or pagination redirects.

- [x] Reject unknown `site.toml` fields where a typo would otherwise silently
  change behavior.
- [x] Produce path/line-aware TOML diagnostics.
- [x] Supply a minimal example directory fixture independent of this website.
- [x] Test empty sites, one-page sites, paths with spaces, Unicode, read-only
  trees, and internal/escaping symlinks.
- [x] Test a large-but-valid Markdown input near the per-file limit.
- [x] Enumerate the complete `site.toml` schema and all content/asset size
  limits in the public content-contract documentation.

### 15.3 Scaling

The current client fetches and parses the complete site bundle before rendering
any route. Keep it for the personal site initially, but measure it against the
declared 64 MiB content limit.

- [x] Record site JSON parse and first-readiness timing in the packaged Chromium
  gate using in-page monotonic performance markers. Process RSS is deliberately
  not budgeted: Chromium's multi-process/shared-cache RSS is not stable or a
  meaningful measure of the WASM application's retained memory in CI.
- [x] Define and enforce budgets for the native executable, gzip-compressed
  WASM, generated CSS, logical site output, site JSON, JSON parsing, and first
  browser readiness.
- [x] Keep the single fingerprinted site bundle for the current contract. Its
  size, parse time, and readiness remain well within enforced budgets; sharding
  is a future architecture change only if those measured gates fail.
- [x] Preserve content hashes and offline/static-host compatibility: content
  assets and site JSON are fingerprinted, the WASM verifies the bundle digest,
  clean builds are deterministic, and standalone static/browser startup passes.
  Any future sharding must retain this contract.

## 16. Phase 10: verification and release

### 16.1 Unit and integration tests

- parser/front matter/schema validation;
- draft filtering;
- reserved route collisions;
- symlink-safe output resolution;
- exact headings/IDs/TOC;
- shortcode arguments and safe rendering;
- asset collision and ownership graph;
- URL/color validation;
- site-style inheritance and weight sorting;
- feed/sitemap semantics;
- aliases/public files;
- deterministic clean builds;
- check/build agreement;
- invalid-build preservation and crash recovery.

### 16.2 Browser tests

- immutable legacy visual comparison;
- full-page or targeted deep-article captures;
- component captures for cards, code, every shortcode, TOC, references,
  sidenotes, resume, and presentation states;
- hover/focus/active captures;
- controlled glitch frames;
- typewriter sequence;
- mobile menu keyboard order;
- SPA focus, announcements, scroll restoration, and metadata;
- no-JS semantic fallback;
- startup failure UI;
- reduced motion/video;
- touch/swipe and zoom/reflow;
- no horizontal overflow on every required route;
- breakpoint-boundary matrix;
- print/PDF output.

### 16.3 Accessibility gates

- [x] Add a mandatory, pinned-browser accessibility lane across 12
  representative routes at 1440x900 and 390x844.
- [x] Keep targeted assertions for issues generic scanners miss: SPA focus,
  draft leakage, reduced-motion media, zoom clipping, talk announcements,
  sidenote reveal, and absolute SEO URLs. Focused
  accessibility/interaction/content/package lanes now cover the complete list.
- [x] Assert one main, one h1, named navigation, real keyboard-visible focus,
  image names/decorative semantics, control names/states/slider bounds, and
  hidden inactive slides.
- [x] Add numeric contrast validation for generated foreground, interactive,
  focus, and accent-text palettes.
- [x] Extend the ordinary unzoomed route matrix to tablet and 320px, including
  document overflow, representative layout, horizontal focus containment, and
  presentation-control containment.

### 16.4 CI

- [x] Run the current `make verify-all` surface as required pull-request jobs:
  verify, visual, interaction, accessibility, and standalone package/browser
  startup.
- [x] Keep visual and interaction lanes in the pinned Nix environment.
- [x] Add required standalone package/browser startup coverage before release.
- [x] Run presentation print/PDF compatibility as a mandatory pull-request
  workflow step rather than relying only on local `verify-all`.
- [x] Do not silently skip Chromium in the lane that claims browser startup.
- [x] Allocate ports dynamically instead of hard-coding `44180`.
- [x] Test Linux amd64/arm64 and macOS amd64/arm64 package construction in the
  required pull-request workflow, with a portable exact package-file,
  embedded-runtime, attribution, and complete license-inventory check.
- [x] Keep wasm-bindgen CLI and crate versions exactly aligned through the
  Makefile-derived `toolchain-check` and pinned CI installation.
- [x] Pin Rust `1.94.0` in `rust-toolchain.toml` and both test/release
  workflows.

### 16.5 Release artifacts

- one executable containing the application runtime;
- license and attribution files in the distribution archive;
- [x] Generate and upload SHA-256 checksums for every release archive;
- [x] Verify every generated archive checksum before upload.
- optional signatures and SBOM;
- [x] Enforce binary, compressed WASM, CSS, logical site-output, site JSON,
  parse-time, and first-readiness budgets in the standalone package/browser
  gate;
- license gate for every embedded font/icon/media asset.
- [x] Block the release matrix on the full pinned `make verify-all` gate; the
  per-architecture jobs then build and verify native package files without
  pretending macOS runners provide the Chromium tooling required by
  `make package`.

The 2026-08-03 release-package measurement is the baseline for the enforced
budgets below. Byte totals use exact logical file lengths rather than allocated
disk blocks; WASM compression uses deterministic `gzip -9`. Timings use the
browser's monotonic performance clock and have substantial CI headroom.

| Artifact/measurement | Observed | Enforced maximum |
|---|---:|---:|
| Native executable | 6,168,808 bytes | 8 MiB |
| Gzip-compressed WASM | 264,709 bytes | 320 KiB |
| Generated CSS total | 49,475 bytes | 64 KiB |
| Logical generated site | 7,408,217 bytes | 10 MiB |
| Site JSON | 927,654 bytes | 1.25 MiB |
| Site JSON parse | 14 ms | 500 ms |
| First browser readiness | 153 ms | 5,000 ms |

## 17. Documentation cleanup

- [x] Rewrite `COMPATIBILITY.md` around the immutable oracle and current
  content-owned/runtime-owned asset architecture.
- [x] Keep `README.md` short and accurate about current commands and the
  one-binary/content model.
- [x] Update `THIRD_PARTY.md` for the removal of copied frameworks and webfont
  bundles; update it again if a font/icon decision adds embedded bytes.
- [x] Remove obsolete claims that current-render snapshots are compatibility
  evidence.
- [x] Remove obsolete instructions to embed Reveal.js or copied asset trees.
- [x] Document the immutable oracle workflow.
- [x] Document deterministic generic content discovery, routing, symlinks, and
  content-local media in `docs/content-contract.md`.
- [x] Document `site_url` versus `base_url`.
- [x] Document preview-server limitations and safe VPN binding.
- [x] Remove the ignored obsolete `dist/` tree after confirming its 347 files
  were generated output or copies, with no unique source or oracle data.

## 18. Required implementation sequence

Do not polish randomly. Execute in this order:

1. Preserve immutable visual and semantic oracles.
2. Fix draft leakage.
3. Fix symlink output overlap.
4. Implement the reserved ownership/collision graph.
5. Repair Markdown block boundaries and exact heading IDs/TOC.
6. Make `check` and `build` share validation.
7. Introduce `site_url` and repair canonical/RSS/sitemap output.
8. Add semantic fallback HTML and bootstrap failure handling.
9. Resolve fonts/icons and restore global geometry.
10. Restore homepage and glitch behavior.
11. Restore cards, articles, shortcodes, sidenotes, resume, navigation, footer,
    progress, and 404.
12. Implement required presentation state/scaling/backgrounds/transitions and
    touch/accessibility.
13. Add advanced presentation behavior.
14. Restore selected feeds, public files, aliases, and redirects.
15. Harden watcher, output transaction, preview HTTP, CI, and releases.
16. Finalize the generic arbitrary-directory contract.

Every step must add the test that would have caught the defect before moving to
the next layer. Do not refresh a failing compatibility oracle to make a patch
pass.

### 18.1 Immediate continuation order from the current worktree

1. [x] Finish the concrete second-round visual patches in Section 4.5: homepage
   intrinsic geometry, mobile brand, ordinary 404, corrected deck scale,
   invariant deck typography/padding, progress visual placement, and
   unobtrusive presentation utilities.
2. [x] Run `make fmt-check`, `make test`, `make test-web`, `make clippy`,
   `make interaction`, and `make accessibility`; repair regressions before
   taking any new visual evidence.
3. [x] Run `make visual`, retain the immutable baseline, record all original 52 RMSE
   values, and inspect actual/diff captures. Add component rectangle/style
   assertions for every large improvement so parity is not screenshot-only.
4. [x] Implement and verify the selected GohuFont uni14 direction described in
   Section 9.1. Work Sans and its OFL are removed; its 200/300/500 A/B
   experiments remain historical evidence only. The reviewed
   homepage, resume, menu, folder, clock, and globe Font Awesome outlines are
   all implemented as a minimal local SVG subset whose licensing is packaged;
   no icon framework or glyph-font task remains.
5. [x] Complete the independently actionable non-font component parity.
   Nested folder tiles, card/article separators,
   canonical/default thumbnails, listing/taxonomy-root policy, the stable PGP
   link/surface, About phone media, homepage typewriter, and exact glitch
   behavior, safe caption-alt derivation, authored-autoplay rejection, and
   resume component density are complete. Taxonomy-root raster captures and
   focused block/image/tip/hide/command/button/sidenote/sideimage
   browser geometry are also complete. The shared exact article/home/resume
   typography floor is now measured under Gohu and remains scoped visual-polish
   work rather than keeping this broad component item artificially open.
   Treat known oracle artifacts as explicit exceptions, never as targets.
6. [x] Finish presentation theme isolation and exact control-arrow cosmetics. The
   real headless print/PDF page-count/layout proof, fragment-aware navigation,
   progress/hashes, Reveal timing, notes, presenter view, print visibility CSS,
   touch guards, and zoom/reflow are implemented and focused-test verified.
7. [x] Implement durable awaited bootstrap loading/failure behavior, successful
   status removal, readable missing/corrupt-WASM alerts, cache-busted Retry,
   wrong-MIME fallback, missing/corrupt site-data errors, and duplicate-free
   replacement.
   JavaScript-disabled Chromium and all-shell static-crawler assertions pass.
8. [x] Finish all reviewed legacy feed families and the historical
   alias/redirect inventory, including semantic XML tests, article metadata,
   root/section/taxonomy/term feed semantics, and sitemap `lastmod`.
9. [x] Add fingerprint/case/ancestry collision adversarial cases, watcher churn
   and timestamp stress, startup/JSON/WASM/CSS failure coverage, and numeric
   contrast. The 320px and unzoomed tablet route matrices plus conditional
   ETags and enforced startup/size budgets are also complete. Real Linux/macOS
   process contention, forced-exit stale-lock recovery, interruption at every
   publication rename boundary, and sustained multi-process stress are
   covered. Windows durability semantics remain future platform work; preview
   compression is explicitly deferred as documented in Section 5.
10. [x] Temporarily move `./xtra` outside the repository, then run `make test`,
    `make oracle-check`, and `make package` through the pinned environment.
    This proof passed and the oracle checkout was restored afterward.
11. Run `make verify-all`, all required four-platform package jobs, release
    checksum/license gates, and only then update the final acceptance evidence.

## 19. Final acceptance criteria

The project is ready when all statements below are true:

1. `./xtra` can be deleted and a clean checkout still builds, tests, packages,
   and serves the site.
2. A released `faqe` executable needs only the selected content directory at
   runtime.
3. Users manage no generated HTML, CSS, JS, WASM, font, icon, or theme files.
4. Unpublished content and draft-only assets are absent from release output.
5. Output overlap, symlink, traversal, case-folding, generated-route, asset, and
   public-file collisions are rejected before writes.
6. `faqe check` accepts exactly the inputs that `faqe build` can publish.
7. Existing page routes, approved historical aliases, downloads, heading IDs,
   TOCs, feed routes, and sitemap entries match the reviewed contract.
8. Global typography, geometry, colors, spacing, cards, articles, shortcodes,
   resume, 404, homepage, and glitch effects match immutable fixtures within
   documented geometry and image thresholds.
9. The current presentation matches the reviewed deck in scaling, styling,
   backgrounds, transitions, horizontal/vertical navigation, keyboard, touch,
   progress, and accessibility.
10. Every route has semantic generated fallback content, one main landmark, one
    h1, correct metadata, and a useful startup failure state.
11. SPA navigation manages focus, announcements, metadata, scroll, and history.
12. Reduced motion, keyboard-only use, touch, narrow screens, zoom/reflow,
    contrast, alternative text, safe areas, and print output pass dedicated
    tests.
13. Canonical, OpenGraph, sitemap, and feed URLs use the configured absolute
    public origin and deployment base exactly once.
14. Deterministic, package, browser, interaction, accessibility, visual,
    security, and release gates all pass through the Makefile.
15. Embedded asset licensing and attribution are complete.
16. Documentation describes the implementation that actually ships.

## 20. Decisions requiring explicit resolution

These decisions should be recorded before their corresponding implementation
phase:

1. **Fonts — resolved and implemented:** the user selected GohuFont uni14 Nerd
   Font Mono and superseded Daytona/Novecento redistribution. FAQE embeds the
   safe unpatched Gohu base-glyph byte and exposes it under the requested
   `GohuFont uni14 Nerd Font Mono` alias. The alias does not claim Nerd/icon
   coverage. Iosevka remains embedded for navigation/code and local SVG owns
   the reviewed icons.
   Work Sans Light (300) was also tested as an additional maintained face and
   rejected: the immutable matrix regressed from 41 failures/11 passes to 42
   failures/10 passes, with nearly every route/viewport RMSE worsening. Keep
   no Work Sans face; Gohu replaced it after maintained-byte, license, package,
   browser, accessibility, and print gates passed. Never refresh baselines merely
   to accept a font.
   Work Sans Medium (500) was then tested independently with only the 200 face
   and regressed further to 43 failures/9 passes; talk and 404 comparisons
   worsened especially, while only a few comparisons were unchanged. It is also
   rejected and must not be restored without better immutable-matrix evidence.
2. **Historical URLs — resolved:** the reviewed inventory keeps 82 aliases and
   explicitly retires 30 stale HTML routes; pagination aliases are
   content-owned in `aliases.toml` and no new pagination family is invented.
3. **Stable public files — resolved:** `/public.asc`, `/public.txt`,
   `/keybase.txt`, and `/CNAME` remain direct content-selected outputs.
4. **Presentation scope — resolved:** implement the current deck completely and
   the explicitly listed generic controls/configuration, not an undocumented
   promise to clone every Reveal.js plugin.
5. **Fallback rendering — resolved:** use replace-after-startup, not hydration;
   pre- and post-startup landmark equivalence is package-tested.
6. **Production hosting — resolved:** `faqe serve` remains a hardened preview
   server with explicit VPN binding, not a production HTTP server.
7. **Generic input contract — resolved:** arbitrary directories use the
   documented deterministic defaults, optional strict `site.toml`, and the
   independent generic fixtures; personal metadata/assets are never inferred.
8. **Video poster contract — resolved:** retain the content-owned
   thumbnail-derived poster/fallback for this version rather than adding a
   second poster field. A page without a thumbnail retains its background
   surface and readable content.
9. **Slide IDs — resolved:** authored explicit IDs are the stable public-link
   contract. Deterministic numeric fallback hashes remain navigable but are an
   implementation fallback, not a promised permanent URL vocabulary.

## 21. Current build commands

The automated test system was removed at the owner's request after accepting
the milestone. The remaining Makefile checks build, format, lint, document, and
package the product:

```sh
nix develop --impure -c make fmt-check
nix develop --impure -c make check
nix develop --impure -c make check-all
nix develop --impure -c make clippy
nix develop --impure -c make rustdoc
nix develop --impure -c make package
```

`nix develop --impure -c make verify-all` runs those build-quality checks and
constructs the release package. It does not execute automated tests.
