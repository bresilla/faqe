# Third-party asset inventory

The generator has no copied browser asset tree. This inventory covers the small
amount of third-party content still emitted or referenced by the binary.

| Asset family | Provenance | Status |
|---|---|---|
| YubiKey Guide diagram | `LICENSES/YUBIKEY-GUIDE-MIT.txt` | MIT |
| Iosevka Term SS01 Light | `LICENSES/IOSEVKA-OFL.txt` | SIL Open Font License 1.1 |
| GohuFont uni14 | `LICENSES/GOHUFONT-WTFPL.txt`, `LICENSES/GOHUFONT-PROVENANCE.md` | WTFPL v2 |
| DotGothic16 Regular | `LICENSES/DOTGOTHIC16-OFL.txt` | SIL Open Font License 1.1 |
| Font Awesome 4.5 selected icon outlines | `LICENSES/FONT-AWESOME-OFL.txt` | SIL Open Font License 1.1 |
| Site chrome, post, and page media | files co-located with source under `content/` | Content owner is responsible for publication rights |

The generator itself ships no third-party browser framework. The maintained
Iosevka font byte is embedded in the native binary and emitted automatically as
a fingerprinted site asset with its unmodified family name and OFL license.
The exact upstream GohuFont uni14 base-glyph byte is likewise embedded and
emitted as the maintained theme face. Generated CSS exposes it under the
requested `GohuFont uni14 Nerd Font Mono` family name at normal/400.
DotGothic16 Regular is embedded as the Katakana fallback and limited by CSS to
Japanese kana Unicode ranges. The maintained byte comes from `google/fonts`
commit `d5ef175583bb5f7a3b01bc6c4603dd4a1f445f34`; its SHA-256 is
`3ad9af88726d42b40f7f365f0dcac785af73cf20ea6f1d5b44e57cc21150b8f1`.

The renderer retains only the Font Awesome 4.5 outlines it uses: five homepage
social marks plus menu, clock, folder, and the resume contact/section marks.
They are emitted as inline SVG paths from Rust rather than as a font,
stylesheet, JavaScript package, or copied framework tree. The corresponding
OFL notice ships with the binary package.

The historical Daytona and Novecento files are commercial fonts whose old
repository bytes remain excluded. The selected Gohu direction supersedes them,
so obtaining Daytona/Novecento redistribution rights is no longer a release
blocker.

The requested complete Nerd Font combines Gohu with patched symbol glyphs whose
license and notice chain is not yet suitable for this package. The safe
candidate is the exact unpatched `gohufont-uni-14.ttf` base-glyph file from
`koemaeda/gohufont-ttf` commit
`2e5e68a8a54a127c3f61e7ba43f0e0834bf1bed0`, under the upstream WTFPL v2
`COPYING-LICENSE`. The audited candidate is 120,776 bytes with SHA-256
`e24c3974ec8c4d3697dafcc5af510a43d2825a7c7251ec0464a7b98f538be0d4`;
its internal family/style/PostScript names are `GohuFont`, `uni-14`, and
`GohuFont-uni-14`. Generated CSS exposes it under the requested normal/400
alias `GohuFont uni14 Nerd Font Mono`. That alias is not a claim
that the emitted byte is OS-installed, Nerd-patched, or includes Nerd Font
symbols; icon coverage remains owned by the separately licensed local SVG
subset.
