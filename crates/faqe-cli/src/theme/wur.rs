//! Wageningen University & Research talk template.
//!
//! Layout geometry, palette, and artwork follow the WUR Quarto format
//! (`drapado/Quarto-WUR`) at half of its 1920 × 1080 canvas. The artwork and
//! derived styling are covered by `LICENSES/WUR-PROPRIETARY.txt`.

use super::{Asset, TalkTemplate};

pub const ID: &str = "wur";

const ASSETS: &[Asset] = &[
    Asset {
        id: "wur-logo-light",
        stem: "wur-logo-light",
        extension: "svg",
        bytes: include_bytes!("../../assets/wur/wur-logo-light.svg"),
    },
    Asset {
        id: "wur-title-wide",
        stem: "wur-title-wide",
        extension: "svg",
        bytes: include_bytes!("../../assets/wur/wur-title-wide.svg"),
    },
    Asset {
        id: "wur-title-left",
        stem: "wur-title-left",
        extension: "svg",
        bytes: include_bytes!("../../assets/wur/wur-title-left.svg"),
    },
    Asset {
        id: "wur-title-right",
        stem: "wur-title-right",
        extension: "svg",
        bytes: include_bytes!("../../assets/wur/wur-title-right.svg"),
    },
];

pub const TEMPLATE: TalkTemplate = TalkTemplate {
    id: ID,
    width: 960,
    height: 540,
    assets: ASSETS,
    style: STYLE,
};

const STYLE: &str = r#"
/* Palette, typography, and presentation surface */
.faqe-talk[data-faqe-talk-template="wur"]{--wur-blue:#002b47;--wur-green:#004d00;--wur-lime:#97c600;--wur-sky:#88c9e4;--wur-cream:#fbf6e5;--wur-grey:#f2f2f2;--wur-x:28px;--wur-top:22px;--wur-bottom:54px;--faqe-talk-chrome-primary:var(--wur-green);--faqe-talk-chrome-secondary:#4d6b7e;--faqe-talk-line:rgba(0,77,0,.38);font-family:Verdana,Arial,sans-serif;font-size:16px;line-height:1.28}
.faqe-talk[data-faqe-talk-template="wur"]::before,.faqe-talk[data-faqe-talk-template="wur"]::after,.faqe-talk[data-faqe-talk-template="wur"] .faqe-slide-background::after{display:none}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-slide-background{background-image:none!important}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section{align-items:stretch;background:#fff;box-shadow:none;color:var(--wur-blue);justify-content:flex-start;overflow:hidden;padding:var(--wur-top) var(--wur-x) var(--wur-bottom);text-align:left;transform:none!important;transition:opacity .45s ease,visibility .45s}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section>*{position:relative;z-index:2}

/* Headings render as static text */
.faqe-talk[data-faqe-talk-template="wur"] :is(h1,h2,h3,h4,h5,h6){color:var(--wur-green);font-family:inherit;font-weight:400;letter-spacing:0;line-height:1.08;margin:0 0 .5em;max-width:100%;text-shadow:none;text-transform:none}
.faqe-talk[data-faqe-talk-template="wur"] :is(h1,h2,h3)::after{display:none}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-scramble{display:inline}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-scramble>*{display:none!important}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-scramble::before{content:attr(aria-label)}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section>h1{font-size:1.6em;letter-spacing:-.02em;margin-bottom:.72em}
.faqe-talk[data-faqe-talk-template="wur"] h2{font-size:1.18em;font-weight:700}
.faqe-talk[data-faqe-talk-template="wur"] :is(h3,h4,h5,h6){font-size:1em;font-weight:700}

/* Body text */
.faqe-talk[data-faqe-talk-template="wur"] :is(p,ul,ol){font-size:1em;line-height:1.28;margin:0 0 .65em;max-width:100%;overflow-wrap:break-word;width:auto}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section>:is(p,ul,ol,blockquote){max-width:590px}
.faqe-talk[data-faqe-talk-template="wur"] :is(ul,ol){padding-left:1.25em}
.faqe-talk[data-faqe-talk-template="wur"] :is(ul,ol) :is(ul,ol){margin:.2em 0 0}
.faqe-talk[data-faqe-talk-template="wur"] li{line-height:1.28;margin:.25em 0}
.faqe-talk[data-faqe-talk-template="wur"] li::marker{color:var(--wur-blue)}
.faqe-talk[data-faqe-talk-template="wur"] strong{color:var(--wur-green);font-weight:700}
.faqe-talk[data-faqe-talk-template="wur"] a{color:var(--wur-blue);text-decoration:underline;text-decoration-color:var(--wur-lime);text-decoration-thickness:.08em;text-underline-offset:.12em}
.faqe-talk[data-faqe-talk-template="wur"] .fragment.current-fragment{color:inherit}
.faqe-talk[data-faqe-talk-template="wur"] :is(pre,code){font-family:"Iosevka Term SS01",ui-monospace,Menlo,Consolas,monospace}
.faqe-talk[data-faqe-talk-template="wur"] :not(pre)>code{background:var(--wur-grey);border:0;border-radius:3px;color:var(--wur-blue);padding:.05em .3em}
.faqe-talk[data-faqe-talk-template="wur"] blockquote{background:rgba(251,246,229,.58);border:0;border-top:4.5px solid var(--wur-lime);box-shadow:none;color:var(--wur-blue);font-style:normal;margin:.4em 0 .8em;padding:12px 15px}
.faqe-talk[data-faqe-talk-template="wur"] blockquote p{margin:0}
.faqe-talk[data-faqe-talk-template="wur"] .highlight{background:var(--wur-grey);border:0;border-top:3px solid var(--wur-green);box-shadow:none;margin:.4em 0;max-width:100%;width:100%}
.faqe-talk[data-faqe-talk-template="wur"] .code-toolbar{background:transparent;border-bottom:1px solid rgba(0,43,71,.12);color:var(--wur-blue)}
.faqe-talk[data-faqe-talk-template="wur"] .chroma{background:var(--wur-grey);color:var(--wur-blue);font-size:11px;line-height:1.4;max-height:300px}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-keyword,.ts-function-builtin){color:#6b2380}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-function,.ts-constructor){color:#00589c}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-type,.ts-type-builtin,.ts-attribute,.ts-tag){color:#004d00}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-string,.ts-string-special,.ts-string-special-key){color:#8a3c00}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-number,.ts-boolean,.ts-constant,.ts-constant-builtin,.ts-escape){color:#a33a00}
.faqe-talk[data-faqe-talk-template="wur"] :is(.ts-variable,.ts-variable-builtin,.ts-variable-parameter,.ts-property,.ts-property-builtin,.ts-operator,.ts-punctuation-special){color:var(--wur-blue)}
.faqe-talk[data-faqe-talk-template="wur"] .ts-comment{color:#5a6b76;font-style:italic}

/* Markdown tables */
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column,.fragment)>table{border-collapse:collapse;font-size:.72em;margin:.4em 0;max-width:100%;width:100%}
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column,.fragment)>table :is(th,td){border:0;border-bottom:1px solid rgba(0,43,71,.16);color:var(--wur-blue);padding:.55em .7em;text-align:left}
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column,.fragment)>table th{background:var(--wur-green);color:var(--wur-cream);font-weight:700}
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column,.fragment)>table tbody tr:nth-child(even){background:rgba(136,201,228,.14)}

/* Columns and media */
.faqe-talk[data-faqe-talk-template="wur"] .presenterm-columns{align-items:stretch;display:grid;flex:1 1 auto;gap:28px;grid-template-columns:var(--faqe-columns,1fr)!important;margin:0 0 .4em;min-height:0}
.faqe-talk[data-faqe-talk-template="wur"] .presenterm-column{display:flex;flex-direction:column;min-height:0;min-width:0}
.faqe-talk[data-faqe-talk-template="wur"] .presenterm-column>*{max-width:100%}
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column)>p:has(>img:only-child){align-items:center;display:flex;flex:1 1 auto;justify-content:center;margin:0;max-width:none;min-height:0;width:100%}
.faqe-talk[data-faqe-talk-template="wur"] :is(section,.presenterm-column)>p:has(>img[style*="--faqe-image-width"]:only-child){flex:0 0 auto;margin-bottom:8px}
.faqe-talk[data-faqe-talk-template="wur"] .slides img{background:none;border:0;box-shadow:none;height:auto;margin:0;max-height:100%;max-width:100%;object-fit:contain;width:var(--faqe-image-width,auto)}
.faqe-talk[data-faqe-talk-template="wur"] :is(.imageframeholder,.mediaframeholder){margin:.4em 0;width:100%}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-embedded-video{border:0;box-shadow:none;max-height:360px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-primitive{font-family:Verdana,Arial,sans-serif}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-graph-canvas{max-height:270px}

/* Data primitives at slide scale */
.faqe-talk[data-faqe-talk-template="wur"] .faqe-primitive:not(.faqe-graph){font-size:14px;margin:6px 0 10px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-primitive:not(.faqe-graph) *{font-family:Verdana,Arial,sans-serif}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-primitive-caption>strong{font-size:14px;letter-spacing:0}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-primitive-caption>span{font-size:12px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-item>small{font-size:10.5px;letter-spacing:.08em}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-item>strong{font-size:16px;line-height:1.25}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-items.variant-metrics .faqe-grid-item>strong{font-size:17px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-item>div,.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-bullets{font-size:13.5px;line-height:1.4}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-grid-item>span{font-size:11px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-timeline-content>strong{font-size:15px;line-height:1.25}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-timeline-content>small{font-size:11.5px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-timeline-content>div{font-size:13px}
.faqe-talk[data-faqe-talk-template="wur"] :is(.faqe-progress-label,.faqe-progress-value){font-size:13px;white-space:nowrap}
.faqe-talk[data-faqe-talk-template="wur"] :is(.faqe-progress-bar,.faqe-progress-marker){font-size:11px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-data-table table{font-size:13.5px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-data-table thead th{font-size:12px;letter-spacing:.02em}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-data-table caption{font-size:12px}
.faqe-talk[data-faqe-talk-template="wur"] .faqe-data-table :is(th,td){padding:7px 10px}
.faqe-talk[data-faqe-talk-template="wur"] pre,.faqe-talk[data-faqe-talk-template="wur"] pre code{font-size:11px;line-height:1.34}
.faqe-talk[data-faqe-talk-template="wur"] .highlight{flex-shrink:0}
.faqe-talk[data-faqe-talk-template="wur"] pre{margin:0;padding:8px 12px}
.faqe-talk[data-faqe-talk-template="wur"] .chroma{max-height:390px}
.faqe-talk[data-faqe-talk-template="wur"] .code-toolbar{font-size:9px;padding:3px 12px}

/* Full-slide image */
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.image-only{padding:0}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.image-only>p{height:100%}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.image-only img:not([style*="--faqe-image-width"]){height:100%;object-fit:cover;width:100%}

/* Footer bar, logo, and slide number */
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:not(.title-slide):not(.jump-middle):not(.no-footer):not(.image-only)::before{background:url('{{asset:wur-logo-light}}') 14px center/113px 23px no-repeat,var(--wur-green);border:0;border-radius:7px;bottom:6px;content:"";display:block;font-size:0;height:36px;left:20px;opacity:1;pointer-events:none;position:absolute;right:20px;top:auto;z-index:0}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:not(.title-slide):not(.jump-middle):not(.no-footer):not(.image-only)::after{bottom:19.5px;color:var(--wur-cream);content:attr(data-faqe-slide-number);font-size:9px;line-height:1;pointer-events:none;position:absolute;right:34px;z-index:1}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section>p:has(>img.faqe-footer-image:only-child){align-items:center;bottom:6px;display:flex;height:36px;justify-content:center;left:170px;margin:0;position:absolute;right:70px;width:auto;z-index:2}
.faqe-talk[data-faqe-talk-template="wur"] .slides img.faqe-footer-image{height:26px;max-height:26px;max-width:100%;width:auto}

/* Title designs: wide band, left panel, right panel */
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide,.jump-middle){background:#fff var(--faqe-print-background-image,none) center/cover no-repeat;text-align:left}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide,.jump-middle)::before{background:none;border:1.5px solid #fff;border-radius:50%;bottom:auto;content:"";display:block;font-size:0;opacity:1;pointer-events:none;position:absolute;z-index:1}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.jump-middle>p:has(>img:only-child){inset:0;margin:0;max-width:none;position:absolute;width:auto;z-index:0}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.jump-middle>p:has(>img:only-child) img{height:100%;max-height:none;max-width:none;object-fit:cover;width:100%}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide,.jump-middle)::after{background-position:left bottom;background-repeat:no-repeat;background-size:100% 100%;content:"";pointer-events:none;position:absolute;z-index:1}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center){align-content:start;column-gap:24px;display:grid;grid-auto-rows:min-content;grid-template-columns:auto 1fr;justify-content:stretch;padding:328px 48px 0}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center)::before{height:265px;left:347.5px;top:21px;width:265px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center)::after{background-image:url('{{asset:wur-title-wide}}');bottom:36px;height:180px;left:38px;right:38px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center)>*{color:var(--wur-green);grid-column:1/-1;margin:0;max-width:none;text-align:left}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center)>h1{font-size:32px;line-height:1.05}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center)>:not(h1){font-size:19px;line-height:1.1;margin-top:14px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.title-slide:not(.title-left):not(.title-right)>:is(.title-date,.title-author){font-size:13px;grid-column:auto;letter-spacing:0;line-height:1;margin-top:28px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center)){align-items:flex-start;display:flex;flex-direction:column;justify-content:flex-start;padding:52px 48px 133px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-right,.jump-middle.align-right){align-items:flex-end}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center))::before{height:465px;top:21px;width:465px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.jump-middle:not(.align-center):not(.align-right))::before{left:auto;right:36px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-right,.jump-middle.align-right)::before{left:36px;right:auto}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center))::after{bottom:36px;top:21.5px;width:368px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.jump-middle:not(.align-center):not(.align-right))::after{background-image:url('{{asset:wur-title-left}}');left:38px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-right,.jump-middle.align-right)::after{background-image:url('{{asset:wur-title-right}}');right:38px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center))>*{color:var(--wur-cream);margin:0;max-width:320px;text-align:left;width:320px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center))>h1{font-size:31px;line-height:1.05}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center))>:not(h1){font-size:19px;line-height:1.2;margin-top:24px}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right)>:is(.title-date,.title-author){font-size:12.5px;line-height:1.7;margin-top:0}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right)>.title-date{margin-top:auto}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center)) :is(strong,a){color:inherit}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center)) li::marker{color:inherit}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.title-slide>.title-date{display:block}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.title-slide :is(.title-date,.title-author),.faqe-talk[data-faqe-talk-template="wur"] .slides>section.title-slide .title-author strong{color:inherit;font-weight:400;letter-spacing:0}

/* Print as 16:9 pages */
@page faqe-wur{size:960px 540px;margin:0}
@media print{
.faqe-talk[data-faqe-talk-template="wur"] .slides>section{background-color:#fff!important;height:540px!important;max-height:540px!important;min-height:540px!important;padding:var(--wur-top) var(--wur-x) var(--wur-bottom)!important;page:faqe-wur}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section.image-only{padding:0!important}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide:not(.title-left):not(.title-right),.jump-middle.align-center){display:grid!important;padding:328px 48px 0!important}
.faqe-talk[data-faqe-talk-template="wur"] .slides>section:is(.title-slide.title-left,.title-slide.title-right,.jump-middle:not(.align-center)){padding:52px 48px 133px!important}
}
@media(prefers-reduced-motion:reduce){.faqe-talk[data-faqe-talk-template="wur"] .slides>section{transition:none}
}
"#;
