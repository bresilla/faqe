#![cfg(any(target_arch = "wasm32", test))]

use faqe_model::{
    accessible_color, canonical_route, contrasting_text, Document, DocumentNode, ElementNode, Page,
    PageKind, PageStyle, ResumeData, SiteBundle, SiteMetadata, TalkSlide, Theme, TocItem,
    SITE_SCHEMA_VERSION,
};
use gloo_events::EventListener;
use gloo_timers::callback::Timeout;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::fmt::Write as _;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::Response;
use yew::prelude::*;
use yew::virtual_dom::{ApplyAttributeAs, VTag};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn start() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
fn app() -> Html {
    let bundle = use_state(|| None::<SiteBundle>);
    let error = use_state(|| None::<String>);
    let active_route = use_state(current_route);
    let retry = use_state(|| 0_u32);
    let focus_after_navigation = use_state(|| false);

    {
        let bundle = bundle.clone();
        let error = error.clone();
        use_effect_with(*retry, move |_| {
            spawn_local(async move {
                match fetch_bundle().await {
                    Ok(loaded) => {
                        error.set(None);
                        bundle.set(Some(loaded));
                    }
                    Err(message) => error.set(Some(message)),
                }
            });
            || ()
        });
    }

    {
        let active_route = active_route.clone();
        let focus_after_navigation = focus_after_navigation.clone();
        let navigation_bundle = (*bundle).clone();
        use_effect_with(navigation_bundle, move |navigation_bundle| {
            let navigation_bundle = navigation_bundle.clone();
            if let Some(window) = web_sys::window() {
                if let Ok(history) = window.history() {
                    let _ = js_sys::Reflect::set(
                        history.as_ref(),
                        &JsValue::from_str("scrollRestoration"),
                        &JsValue::from_str("manual"),
                    );
                }
            }
            let click_route = active_route.clone();
            let click_focus = focus_after_navigation.clone();
            let click = EventListener::new(&gloo_utils::document(), "click", move |event| {
                let Some(event) = event.dyn_ref::<web_sys::MouseEvent>() else {
                    return;
                };
                if event.default_prevented()
                    || event.button() != 0
                    || event.ctrl_key()
                    || event.meta_key()
                    || event.shift_key()
                    || event.alt_key()
                {
                    return;
                }
                let Some(element) = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .and_then(|element| element.closest("a[href]").ok().flatten())
                else {
                    return;
                };
                if element.has_attribute("download")
                    || element
                        .get_attribute("target")
                        .is_some_and(|target| target != "_self")
                {
                    return;
                }
                let Some(href) = element.get_attribute("href") else {
                    return;
                };
                if gloo_utils::document()
                    .query_selector("link[data-faqe-mode]")
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return;
                }
                if !href.starts_with('/') || href.contains('#') || !is_page_href(&href) {
                    return;
                }
                let destination = route_from_path(&href);
                if navigation_bundle
                    .as_ref()
                    .and_then(|bundle| bundle.page(&destination))
                    .is_some_and(|page| matches!(page.kind, PageKind::Resume | PageKind::Talk))
                {
                    return;
                }
                let Some(window) = web_sys::window() else {
                    return;
                };
                let Ok(history) = window.history() else {
                    return;
                };
                let current_state = history_state(window.scroll_y().unwrap_or(0.0));
                let next_state = history_state(0.0);
                if history
                    .replace_state_with_url(&current_state, "", None)
                    .and_then(|_| history.push_state_with_url(&next_state, "", Some(&href)))
                    .is_err()
                {
                    return;
                }
                event.prevent_default();
                window.scroll_to_with_x_and_y(0.0, 0.0);
                click_focus.set(true);
                click_route.set(current_route());
            });

            let pop_route = active_route.clone();
            let pop_focus = focus_after_navigation.clone();
            let pop = web_sys::window().map(|window| {
                EventListener::new(&window, "popstate", move |event| {
                    let scroll_y = event
                        .dyn_ref::<web_sys::PopStateEvent>()
                        .and_then(|event| history_scroll(&event.state()))
                        .unwrap_or(0.0);
                    pop_focus.set(true);
                    pop_route.set(current_route());
                    if let Some(window) = web_sys::window() {
                        // Route rendering focuses the new main landmark. Apply
                        // saved history scroll after that focus so the focus
                        // operation cannot pull the viewport back to the top.
                        Timeout::new(50, move || window.scroll_to_with_x_and_y(0.0, scroll_y))
                            .forget();
                    }
                })
            });
            move || {
                drop(click);
                drop(pop);
            }
        });
    }

    {
        let route = (*active_route).clone();
        let should_focus = *focus_after_navigation;
        let focus_after_navigation = focus_after_navigation.clone();
        use_effect_with((route, should_focus), move |_| {
            let timeout = should_focus.then(|| {
                Timeout::new(0, move || {
                    if let Some(main) = gloo_utils::document()
                        .get_element_by_id("main")
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = main.focus();
                    }
                    focus_after_navigation.set(false);
                })
            });
            move || drop(timeout)
        });
    }

    if let Some(message) = (*error).as_ref() {
        let retry_request = {
            let retry = retry.clone();
            let error = error.clone();
            Callback::from(move |_| {
                error.set(None);
                retry.set(retry.wrapping_add(1));
            })
        };
        return html! {
            <main id="main" tabindex="-1" class="container centered estate faqe-error" role="alert">
                <h1>{"FAQE failed to start"}</h1>
                <p>{message}</p>
                <button type="button" onclick={retry_request}>{"Retry"}</button>
            </main>
        };
    }
    let Some(bundle) = (*bundle).as_ref() else {
        return html! { <main id="main" tabindex="-1" class="faqe-loading" role="status" aria-live="polite" aria-label="Loading website">{"_"}</main> };
    };
    if bundle.schema_version != SITE_SCHEMA_VERSION {
        return html! {
            <main id="main" tabindex="-1" class="container centered estate faqe-error" role="alert">
                <h1>{"Incompatible content"}</h1>
                <p>{format!("website schema {}, runtime schema {}", bundle.schema_version, SITE_SCHEMA_VERSION)}</p>
            </main>
        };
    }

    let route = (*active_route).clone();
    if route == "/" {
        let home = bundle.page("/").cloned();
        let style = home
            .as_ref()
            .map(|page| &page.style)
            .unwrap_or(&bundle.site.default_style);
        update_metadata(
            &bundle.site.title,
            &bundle.site.description,
            &bundle.site.keywords,
            style,
            &bundle.site,
            "/",
            home.as_ref(),
        );
        mark_ready();
        return html! { <StandardShell bundle={bundle.clone()} page={home}><HomePage bundle={bundle.clone()} /></StandardShell> };
    }
    let page = bundle.page(&route).cloned();
    if let Some(page) = page {
        update_metadata(
            &format!("{} · {}", page.title, bundle.site.title),
            page.description
                .as_deref()
                .filter(|description| !description.trim().is_empty())
                .unwrap_or(&bundle.site.description),
            &bundle.site.keywords,
            &page.style,
            &bundle.site,
            &page.route,
            Some(&page),
        );
        if page.kind == PageKind::Talk {
            let exit_route = talk_exit_route(bundle, &page);
            mark_ready();
            html! { <TalkPage page={page} {exit_route} author={bundle.site.author.clone()} /> }
        } else {
            mark_ready();
            html! {
                <StandardShell bundle={bundle.clone()} page={Some(page.clone())}>
                    <PageView bundle={bundle.clone()} page={page} />
                </StandardShell>
            }
        }
    } else if let Some(taxonomy) = taxonomy_view(bundle, &route) {
        update_metadata(
            &format!("{} · {}", taxonomy.title, bundle.site.title),
            &bundle.site.description,
            &bundle.site.keywords,
            &bundle.site.default_style,
            &bundle.site,
            &route,
            None,
        );
        mark_ready();
        html! {
            <StandardShell bundle={bundle.clone()} page={None}>
                <TaxonomyPage bundle={bundle.clone()} view={taxonomy} />
            </StandardShell>
        }
    } else {
        update_metadata(
            &format!("404 · {}", bundle.site.title),
            &bundle.site.description,
            &bundle.site.keywords,
            &bundle.site.default_style,
            &bundle.site,
            &route,
            None,
        );
        mark_ready();
        html! { <StandardShell bundle={bundle.clone()} page={None}><NotFound /></StandardShell> }
    }
}

fn talk_exit_route(bundle: &SiteBundle, talk: &Page) -> String {
    bundle
        .pages
        .iter()
        .filter(|page| {
            page.kind == PageKind::Section
                && page.route != "/"
                && page.route != talk.route
                && talk.route.starts_with(&page.route)
        })
        .min_by_key(|page| page.route.len())
        .map(|page| page.route.clone())
        .unwrap_or_else(|| "/".into())
}

fn exit_talk(exit_href: &str) {
    gloo_utils::document().exit_fullscreen();
    let Some(window) = web_sys::window() else {
        return;
    };
    let referrer = gloo_utils::document().referrer();
    let origin = window.location().origin().unwrap_or_default();
    let has_same_origin_referrer = referrer
        .strip_prefix(&origin)
        .is_some_and(|path| path.starts_with('/'));
    let history = window.history().ok();
    let can_go_back = has_same_origin_referrer
        && history
            .as_ref()
            .and_then(|history| history.length().ok())
            .is_some_and(|length| length > 1);
    if can_go_back && history.is_some_and(|history| history.back().is_ok()) {
        return;
    }
    let _ = window.location().set_href(exit_href);
}

fn is_page_href(href: &str) -> bool {
    let path = href.split(['?', '#']).next().unwrap_or(href);
    path == "/" || path.ends_with('/') || !path.rsplit('/').next().unwrap_or(path).contains('.')
}

fn history_state(scroll_y: f64) -> JsValue {
    let state = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &state,
        &JsValue::from_str("faqeScrollY"),
        &JsValue::from_f64(scroll_y),
    );
    state.into()
}

fn history_scroll(state: &JsValue) -> Option<f64> {
    js_sys::Reflect::get(state, &JsValue::from_str("faqeScrollY"))
        .ok()
        .and_then(|value| value.as_f64())
}

async fn fetch_bundle() -> Result<SiteBundle, String> {
    let window = web_sys::window().ok_or("browser window is unavailable")?;
    let bundle_path = gloo_utils::document()
        .query_selector("meta[name=faqe-bundle]")
        .ok()
        .flatten()
        .and_then(|element| element.get_attribute("content"))
        .ok_or("generated shell is missing the faqe-bundle metadata")?;
    let response = JsFuture::from(window.fetch_with_str(&format!("{}{bundle_path}", base_url())))
        .await
        .map_err(js_message)?;
    let response: Response = response
        .dyn_into()
        .map_err(|_| "fetch did not return a Response".to_owned())?;
    if !response.ok() {
        return Err(format!(
            "failed to load site.json: HTTP {}",
            response.status()
        ));
    }
    let text = response
        .text()
        .map_err(js_message)
        .map(JsFuture::from)?
        .await
        .map_err(js_message)?
        .as_string()
        .ok_or("site.json response was not text")?;
    let expected = bundle_path
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_prefix("site-"))
        .and_then(|name| name.strip_suffix(".json"))
        .filter(|digest| digest.len() == 16 && digest.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or("generated shell has an invalid content bundle filename")?;
    let actual = Sha256::digest(text.as_bytes())
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != expected {
        return Err(format!(
            "site bundle failed its SHA-256 check: expected {expected}, got {actual}"
        ));
    }
    let performance = window.performance();
    let parse_started = performance.as_ref().map(web_sys::Performance::now);
    let bundle =
        serde_json::from_str(&text).map_err(|error| format!("invalid site.json: {error}"))?;
    if let (Some(performance), Some(parse_started), Some(root)) = (
        performance,
        parse_started,
        gloo_utils::document().document_element(),
    ) {
        let _ = root.set_attribute(
            "data-faqe-json-parse-ms",
            &format!("{:.3}", performance.now() - parse_started),
        );
    }
    Ok(bundle)
}

fn js_message(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| format!("JavaScript error: {value:?}"))
}

fn current_route() -> String {
    let path = web_sys::window()
        .and_then(|window| window.location().pathname().ok())
        .unwrap_or_else(|| "/".into());
    let base = base_url();
    route_from_path_with_base(&path, &base)
}

fn route_from_path(path: &str) -> String {
    route_from_path_with_base(path, &base_url())
}

fn route_from_path_with_base(path: &str, base: &str) -> String {
    canonical_route(
        path.strip_prefix(base.trim_end_matches('/'))
            .unwrap_or(path),
    )
}

fn base_url() -> String {
    gloo_utils::document()
        .query_selector("meta[name=faqe-base]")
        .ok()
        .flatten()
        .and_then(|element| element.get_attribute("content"))
        .filter(|value| value.starts_with('/') && value.ends_with('/'))
        .unwrap_or_else(|| "/".into())
}

fn site_url(value: &str) -> String {
    if value.starts_with('/') && !value.starts_with("//") {
        format!("{}{}", base_url().trim_end_matches('/'), value)
    } else {
        value.to_owned()
    }
}

fn apply_theme(style: &faqe_model::PageStyle, theme_id: &str) {
    let Some(root) = gloo_utils::document().document_element() else {
        return;
    };
    let palette = faqe_model::accessible_palette(style)
        .expect("site bundle contains only build-validated page styles");
    let scheme = if style.theme == Theme::Light {
        "light"
    } else {
        "dark"
    };
    let variables = format!(
        "--accent-color:{};--chromatic-a:{};--chromatic-b:{};--bg-color:{};--fg-color:{};--glitch-color:{};--interactive-color:{};--accent-text-color:{};",
        style.accent,
        style.chromatic[0],
        style.chromatic[1],
        style.background,
        style.foreground,
        style.foreground,
        palette.interactive,
        palette.accent_text
    );
    let _ = root.set_attribute("style", &variables);
    let _ = root.set_attribute("data-faqe-theme", theme_id);
    let _ = root.set_attribute("data-faqe-scheme", scheme);
}

fn update_title(title: &str) {
    gloo_utils::document().set_title(title);
}

fn update_metadata(
    title: &str,
    description: &str,
    keywords: &[String],
    style: &faqe_model::PageStyle,
    site: &faqe_model::SiteMetadata,
    route: &str,
    page: Option<&Page>,
) {
    apply_theme(style, &site.theme);
    update_title(title);
    for (selector, attribute, value) in [
        ("meta[name=description]", "content", description.to_owned()),
        ("meta[name=keywords]", "content", keywords.join(", ")),
        (
            "meta[name=theme-color]",
            "content",
            style.background.clone(),
        ),
        ("meta[property='og:title']", "content", title.to_owned()),
        (
            "meta[property='og:description']",
            "content",
            description.to_owned(),
        ),
        ("meta[name='twitter:title']", "content", title.to_owned()),
        (
            "meta[name='twitter:description']",
            "content",
            description.to_owned(),
        ),
        (
            "meta[property='og:type']",
            "content",
            if page.is_some_and(|page| page.kind == PageKind::Post) {
                "article".to_owned()
            } else {
                "website".to_owned()
            },
        ),
    ] {
        set_metadata_attribute(selector, attribute, &value);
    }
    if let Some(canonical) = public_url(&site.site_url, &base_url(), route) {
        set_metadata_attribute("link[rel=canonical]", "href", &canonical);
        set_metadata_attribute("meta[property='og:url']", "content", &canonical);
    }
    if let Some(image) = page
        .and_then(|page| page.thumbnail.as_deref())
        .or(Some(site.avatar.as_str()))
        .filter(|image| !image.trim().is_empty())
        .and_then(|image| public_url(&site.site_url, &base_url(), image))
    {
        upsert_meta("property", "og:image", &image);
        upsert_meta("name", "twitter:image", &image);
        let image_alt = page.map_or(title, |page| page.title.as_str());
        upsert_meta("property", "og:image:alt", image_alt);
        upsert_meta("name", "twitter:image:alt", image_alt);
    } else {
        remove_metadata("meta[property='og:image']");
        remove_metadata("meta[property='og:image:alt']");
        remove_metadata("meta[name='twitter:image']");
        remove_metadata("meta[name='twitter:image:alt']");
    }
    remove_metadata("meta[property='article:published_time']");
    remove_metadata("meta[property='article:tag']");
    if let Some(article) = page.filter(|page| page.kind == PageKind::Post) {
        if let Some(date) = &article.date {
            upsert_meta(
                "property",
                "article:published_time",
                &format!("{date}T00:00:00Z"),
            );
        }
        for tag in &article.tags {
            append_meta("property", "article:tag", tag);
        }
    }
}

fn set_metadata_attribute(selector: &str, attribute: &str, value: &str) {
    if let Ok(Some(element)) = gloo_utils::document().query_selector(selector) {
        let _ = element.set_attribute(attribute, value);
    }
}

fn upsert_meta(key: &str, key_value: &str, content: &str) {
    let selector = format!("meta[{key}='{key_value}']");
    if let Ok(Some(element)) = gloo_utils::document().query_selector(&selector) {
        let _ = element.set_attribute("content", content);
    } else {
        append_meta(key, key_value, content);
    }
}

fn append_meta(key: &str, key_value: &str, content: &str) {
    let document = gloo_utils::document();
    let Some(head) = document.head() else {
        return;
    };
    let Ok(element) = document.create_element("meta") else {
        return;
    };
    let _ = element.set_attribute(key, key_value);
    let _ = element.set_attribute("content", content);
    let _ = head.append_child(&element);
}

fn remove_metadata(selector: &str) {
    let Ok(elements) = gloo_utils::document().query_selector_all(selector) else {
        return;
    };
    for index in (0..elements.length()).rev() {
        if let Some(element) = elements.item(index) {
            if let Some(parent) = element.parent_node() {
                let _ = parent.remove_child(&element);
            }
        }
    }
}

fn public_url(site_origin: &str, base_url: &str, path: &str) -> Option<String> {
    if site_origin.trim().is_empty() {
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
    let mut url = format!("{}/{}", site_origin.trim_end_matches('/'), joined);
    if trailing_slash && !url.ends_with('/') {
        url.push('/');
    }
    Some(url)
}

fn mark_ready() {
    if let Some(root) = gloo_utils::document().document_element() {
        if let Some(elapsed) = web_sys::window()
            .and_then(|window| window.performance())
            .map(|performance| performance.now())
        {
            let _ = root.set_attribute("data-faqe-ready-ms", &format!("{elapsed:.3}"));
        }
        let _ = root.set_attribute("data-faqe-ready", "true");
    }
}

#[derive(Properties, PartialEq)]
struct ShellProps {
    bundle: SiteBundle,
    page: Option<Page>,
    children: Children,
}

#[function_component(StandardShell)]
fn standard_shell(props: &ShellProps) -> Html {
    let open = use_state(|| false);
    let menu_button = use_node_ref();
    let toggle = {
        let open = open.clone();
        Callback::from(move |_| open.set(!*open))
    };
    let close = {
        let open = open.clone();
        Callback::from(move |_| open.set(false))
    };
    {
        let open = open.clone();
        let menu_button = menu_button.clone();
        use_effect_with(*open, move |is_open| {
            let escape = is_open.then(|| {
                let open = open.clone();
                let menu_button = menu_button.clone();
                EventListener::new(&gloo_utils::document(), "keydown", move |event| {
                    let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                        return;
                    };
                    if event.key() == "Escape" {
                        event.prevent_default();
                        open.set(false);
                        if let Some(button) = menu_button.cast::<web_sys::HtmlElement>() {
                            let _ = button.focus();
                        }
                    }
                })
            });
            let outside = is_open.then(|| {
                let open = open.clone();
                EventListener::new(&gloo_utils::document(), "click", move |event| {
                    let inside_navigation = event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                        .and_then(|element| element.closest(".navigation").ok().flatten())
                        .is_some();
                    if !inside_navigation {
                        open.set(false);
                    }
                })
            });
            move || {
                drop(escape);
                drop(outside);
            }
        });
    }
    {
        let open = open.clone();
        let route = props
            .page
            .as_ref()
            .map(|page| page.route.clone())
            .unwrap_or_else(|| "/".to_owned());
        use_effect_with(route, move |_| {
            open.set(false);
            || ()
        });
    }
    let foot = props
        .page
        .as_ref()
        .map(|page| page.display_foot(&props.bundle.site))
        .unwrap_or(&props.bundle.site.default_foot);
    let active_route = current_route();
    let quotes_route = props
        .bundle
        .page_of_type("quotes")
        .map(|page| page.route.clone());
    let page_kind = props
        .page
        .as_ref()
        .map_or("generated", |page| match page.kind {
            PageKind::Front => "front",
            PageKind::Post => "post",
            PageKind::Resume => "resume",
            PageKind::Talk => "talk",
            PageKind::Section => "section",
        });

    html! {
        <div class={classes!("wrapper", props.page.as_ref().map(|page| page.slug.clone()))} data-faqe-page-kind={page_kind}>
            <a class="faqe-skip-link" href="#main">{"Skip to content"}</a>
            {props.page.as_ref().and_then(background_video)}
            <aside class="faqe-theme-signals" aria-hidden="true">
                <span class="faqe-theme-signal" data-faqe-theme-slot="one"></span>
                <span class="faqe-theme-signal" data-faqe-theme-slot="two"></span>
                <span class="faqe-theme-signal" data-faqe-theme-slot="three"></span>
            </aside>
            <nav class="navigation" aria-label="Primary navigation">
                <section class="container">
                    <span class="title"><a class="navigation-title" href={site_url("/")} aria-current={(active_route == "/").then_some("page")}>
                        <ScrambleTitle text={props.bundle.site.title.clone()} profile={GlitchProfile::Navigation} />
                    </a></span>
                    <button ref={menu_button} class="menu-button float-right" aria-label="Toggle navigation" aria-controls="primary-navigation" aria-expanded={open.to_string()} onclick={toggle}>{icon_view("menu")}</button>
                    <ul id="primary-navigation" class={classes!("navigation-list", open.then_some("is-open"))}>
                        {for props.bundle.site.menu.iter().map(|item| html! {
                            <li class="navigation-item"><a class="navigation-link" href={site_url(&item.url)} aria-current={(item.url.starts_with('/') && canonical_route(&item.url) == active_route).then_some("page")} onclick={close.clone()}>{&item.name}</a></li>
                        })}
                    </ul>
                </section>
            </nav>
            <main id="main" tabindex="-1" class="content">{props.children.clone()}</main>
            <div class="faqe-route-status" role="status" aria-live="polite" aria-atomic="true">{gloo_utils::document().title()}</div>
            <footer class="footer">
                <section class="container">{
                    quotes_route
                        .map(|route| html! { <a href={site_url(&route)}><p>{foot}</p></a> })
                        .unwrap_or_else(|| html! { <p>{foot}</p> })
                }</section>
            </footer>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct BundleProps {
    bundle: SiteBundle,
}

#[function_component(HomePage)]
fn home_page(props: &BundleProps) -> Html {
    let hovered = use_state(|| false);
    let onmouseenter = {
        let hovered = hovered.clone();
        Callback::from(move |_| hovered.set(true))
    };
    let onmouseleave = {
        let hovered = hovered.clone();
        Callback::from(move |_| hovered.set(false))
    };
    let onfocus = {
        let hovered = hovered.clone();
        Callback::from(move |_| hovered.set(true))
    };
    let onblur = {
        let hovered = hovered.clone();
        Callback::from(move |_| hovered.set(false))
    };
    let avatar = if *hovered && !props.bundle.site.avatar_hover.is_empty() {
        &props.bundle.site.avatar_hover
    } else {
        &props.bundle.site.avatar
    };
    let roles = props
        .bundle
        .site
        .info
        .split('·')
        .map(|role| title_case(role.trim()))
        .filter(|role| !role.is_empty())
        .collect::<Vec<_>>();
    let identity_route = props
        .bundle
        .page_of_type("identity")
        .map(|page| page.route.clone())
        .unwrap_or_else(|| "/".into());

    html! {
        <section class="container centered estate">
            <h1 class="faqe-visually-hidden">{&props.bundle.site.title}</h1>
            <section class="homesection"><section class="me"><div class="about" id="nodec">
                <div class="home-identity">
                    <div class="avatar"><a class="logo glitch-logo" href={site_url(&identity_route)} aria-label="Open identity page" {onfocus} {onblur}>
                        {if avatar.is_empty() {
                            html! { <span class="avatar-fallback" {onmouseenter} {onmouseleave}>{props.bundle.site.author.chars().next().unwrap_or('?')}</span> }
                        } else {
                            html! {
                                <span class="glitch-logo-frame" {onmouseenter} {onmouseleave}>
                                    <img class="glitch-logo-image glitch-logo-base" src={site_url(avatar)} alt={format!("{} logo", props.bundle.site.author)} />
                                    <img class="glitch-logo-image glitch-logo-layer glitch-logo-cyan" src={site_url(avatar)} alt="" aria-hidden="true" />
                                    <img class="glitch-logo-image glitch-logo-layer glitch-logo-magenta" src={site_url(avatar)} alt="" aria-hidden="true" />
                                    <img class="glitch-logo-image glitch-logo-layer glitch-logo-spectrum" src={site_url(avatar)} alt="" aria-hidden="true" />
                                </span>
                            }
                        }}
                    </a></div>
                    <div class="brand">
                        <div class="author"><a href={site_url(&identity_route)} class="nametext glitch"><ScrambleTitle text={props.bundle.site.author.clone()} profile={GlitchProfile::Brand} /></a></div>
                        <div class="info"><div class="infochild"><Typewriter roles={roles} /></div></div>
                    </div>
                </div>
                <div class="home-end">
                    <div class="socials">
                        {for props.bundle.site.socials.iter().map(|social| html! {
                            <li><a class="textglitch" data-text={social.glyph.clone()} href={social.url.clone()} aria-label={social.name.clone()}>
                                {social_icon_view(&social.name, &social.glyph)}
                            </a></li>
                        })}
                    </div>
                    {if let Some(latest) = props.bundle.latest_post() {
                        html! { <div class="latest"><a href={site_url(&latest.route)}><div class="latestchild"><div class="latesttag"><ScrambleTitle text={format!("LATEST POST: {}", latest.title)} profile={GlitchProfile::PostTitle} /></div></div></a></div> }
                    } else { Html::default() }}
                </div>
            </div></section></section>
        </section>
    }
}

fn save_data_enabled() -> bool {
    web_sys::window()
        .and_then(|window| {
            js_sys::Reflect::get(
                window.navigator().as_ref(),
                &JsValue::from_str("connection"),
            )
            .ok()
        })
        .and_then(|connection| {
            js_sys::Reflect::get(&connection, &JsValue::from_str("saveData")).ok()
        })
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn background_video(page: &Page) -> Option<Html> {
    let video = page.style.video.as_deref()?;
    let poster = page.thumbnail.as_deref().unwrap_or_default();
    let autoplay = background_video_autoplays(prefers_reduced_motion(), save_data_enabled());
    Some(
        html! { <DecorativeBackgroundVideo video={site_url(video)} poster={if !poster.is_empty() { site_url(poster) } else { String::new() }} {autoplay} /> },
    )
}

fn background_video_autoplays(reduced_motion: bool, save_data: bool) -> bool {
    !reduced_motion && !save_data
}

#[derive(Properties, PartialEq)]
struct DecorativeBackgroundVideoProps {
    video: String,
    poster: String,
    autoplay: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum DecorativeVideoState {
    Suppressed,
    Attempting,
    Playing,
    Fallback,
}

#[function_component(DecorativeBackgroundVideo)]
fn decorative_background_video(props: &DecorativeBackgroundVideoProps) -> Html {
    let video_ref = use_node_ref();
    let initial_state = if props.autoplay {
        DecorativeVideoState::Attempting
    } else {
        DecorativeVideoState::Suppressed
    };
    let playback_state = use_state(move || initial_state);

    {
        let video_ref = video_ref.clone();
        let playback_state = playback_state.clone();
        let source = props.video.clone();
        let autoplay = props.autoplay;
        use_effect_with((source, autoplay), move |_| {
            playback_state.set(if autoplay {
                DecorativeVideoState::Attempting
            } else {
                DecorativeVideoState::Suppressed
            });
            let attempt = autoplay.then(|| {
                gloo_timers::callback::Timeout::new(0, move || {
                    let Some(video) = video_ref.cast::<web_sys::HtmlVideoElement>() else {
                        playback_state.set(DecorativeVideoState::Fallback);
                        return;
                    };
                    let playback_state = playback_state.clone();
                    match video.play() {
                        Ok(playback) => wasm_bindgen_futures::spawn_local(async move {
                            match wasm_bindgen_futures::JsFuture::from(playback).await {
                                Ok(_) => playback_state.set(DecorativeVideoState::Playing),
                                Err(_) => playback_state.set(DecorativeVideoState::Fallback),
                            }
                        }),
                        Err(_) => playback_state.set(DecorativeVideoState::Fallback),
                    }
                })
            });
            move || drop(attempt)
        });
    }

    let onerror = {
        let playback_state = playback_state.clone();
        Callback::from(move |_| playback_state.set(DecorativeVideoState::Fallback))
    };
    let poster_style = if !props.poster.is_empty() {
        format!("background-image:url('{}')", props.poster)
    } else {
        String::new()
    };
    let state = match *playback_state {
        DecorativeVideoState::Suppressed => "suppressed",
        DecorativeVideoState::Attempting => "attempting",
        DecorativeVideoState::Playing => "playing",
        DecorativeVideoState::Fallback => "fallback",
    };

    html! {
        <div class="fullscreen-bg" style={poster_style} aria-hidden="true" data-faqe-video-state={state}>
            {if props.autoplay && *playback_state != DecorativeVideoState::Fallback {
                html! { <video ref={video_ref} loop=true muted=true playsinline=true preload="metadata" poster={props.poster.clone()} tabindex="-1" class="fullscreen-bg__video" {onerror}><source src={props.video.clone()} type="video/mp4" /></video> }
            } else {
                Html::default()
            }}
        </div>
    }
}

fn icon_view(name: &str) -> Html {
    let normalized = name.trim_start_matches("fa-").to_ascii_lowercase();
    if let Some((width, path)) = brand_icon(&normalized) {
        return html! {
            <svg class="faqe-icon fa faqe-brand-icon" viewBox={format!("0 0 {width} 1792")} fill="currentColor" aria-hidden="true" focusable="false" data-faqe-icon={normalized}>
                <path d={path} transform="translate(0 1536) scale(1 -1)" />
            </svg>
        };
    }
    if let Some((width, path)) = legacy_ui_icon(&normalized) {
        return html! {
            <svg class="faqe-icon fa faqe-local-icon" viewBox={format!("0 0 {width} 1792")} fill="currentColor" aria-hidden="true" focusable="false" data-faqe-icon={normalized}>
                <path d={path} transform="translate(0 1536) scale(1 -1)" />
            </svg>
        };
    }
    let label = match normalized.as_str() {
        "menu" | "bars" => "☰",
        "clock" => "◷",
        "folder" | "archive" => "▰",
        "envelope" => "✉",
        "phone" => "☎",
        "globe" => "◎",
        "user" => "●",
        "briefcase" => "▣",
        "rocket" => "↑",
        _ => "•",
    };
    html! { <span class="faqe-icon fa" data-faqe-icon={label} aria-hidden="true"></span> }
}

fn social_icon_view(name: &str, authored_glyph: &str) -> Html {
    let normalized = name.trim_start_matches("fa-").to_ascii_lowercase();
    if brand_icon(&normalized).is_some() {
        icon_view(name)
    } else {
        html! { <span class="faqe-icon fa" data-faqe-icon={authored_glyph.to_owned()} aria-hidden="true"></span> }
    }
}

/// The non-brand marks which are actually instantiated by the resume. These
/// paths come from the archived, SIL-OFL Font Awesome 4.5 font, retained as a
/// reviewed SVG subset rather than restoring the font or icon framework.
fn legacy_ui_icon(name: &str) -> Option<(u16, &'static str)> {
    match name {
        "menu" | "bars" => Some((1536, "M1536 192v-128q0 -26 -19 -45t-45 -19h-1408q-26 0 -45 19t-19 45v128q0 26 19 45t45 19h1408q26 0 45 -19t19 -45zM1536 704v-128q0 -26 -19 -45t-45 -19h-1408q-26 0 -45 19t-19 45v128q0 26 19 45t45 19h1408q26 0 45 -19t19 -45zM1536 1216v-128q0 -26 -19 -45t-45 -19h-1408q-26 0 -45 19t-19 45v128q0 26 19 45t45 19h1408q26 0 45 -19t19 -45z")),
        "clock" => Some((1536, "M896 992v-448q0 -14 -9 -23t-23 -9h-320q-14 0 -23 9t-9 23v64q0 14 9 23t23 9h224v352q0 14 9 23t23 9h64q14 0 23 -9t9 -23zM1312 640q0 148 -73 273t-198 198t-273 73t-273 -73t-198 -198t-73 -273t73 -273t198 -198t273 -73t273 73t198 198t73 273zM1536 640q0 -209 -103 -385.5t-279.5 -279.5t-385.5 -103t-385.5 103t-279.5 279.5t-103 385.5t103 385.5t279.5 279.5t385.5 103t385.5 -103t279.5 -279.5t103 -385.5z")),
        "folder" => Some((1664, "M1664 928v-704q0 -92 -66 -158t-158 -66h-1216q-92 0 -158 66t-66 158v960q0 92 66 158t158 66h320q92 0 158 -66t66 -158v-32h672q92 0 158 -66t66 -158z")),
        "user" => Some((1408, "M1408 131q0 -120 -73 -189.5t-194 -69.5h-874q-121 0 -194 69.5t-73 189.5q0 53 3.5 103.5t14 109t26.5 108.5t43 97.5t62 81t85.5 53.5t111.5 20q9 0 42 -21.5t74.5 -48t108 -48t133.5 -21.5t133.5 21.5t108 48t74.5 48t42 21.5q61 0 111.5 -20t85.5 -53.5t62 -81t43 -97.5t26.5 -108.5t14 -109t3.5 -103.5zM1088 1024q0 -159 -112.5 -271.5t-271.5 -112.5t-271.5 112.5t-112.5 271.5t112.5 271.5t271.5 112.5t271.5 -112.5t112.5 -271.5z")),
        "phone" => Some((1408, "M1408 296q0 -27 -10 -70.5t-21 -68.5q-21 -50 -122 -106q-94 -51 -186 -51q-27 0 -52.5 3.5t-57.5 12.5t-47.5 14.5t-55.5 20.5t-49 18q-98 35 -175 83q-128 79 -264.5 215.5t-215.5 264.5q-48 77 -83 175q-3 9 -18 49t-20.5 55.5t-14.5 47.5t-12.5 57.5t-3.5 52.5q0 92 51 186q56 101 106 122q25 11 68.5 21t70.5 10q14 0 21 -3q18 -6 53 -76q11 -19 30 -54t35 -63.5t31 -53.5q3 -4 17.5 -25t21.5 -35.5t7 -28.5q0 -20 -28.5 -50t-62 -55t-62 -53t-28.5 -46q0 -9 5 -22.5t8.5 -20.5t14 -24t11.5 -19q76 -137 174 -235t235 -174q2 -1 19 -11.5t24 -14t20.5 -8.5t22.5 -5q18 0 46 28.5t53 62t55 62t50 28.5q14 0 28.5 -7t35.5 -21.5t25 -17.5q25 -15 53.5 -31t63.5 -35t54 -30q70 -35 76 -53q3 -7 3 -21z")),
        "briefcase" => Some((1792, "M640 1280h512v128h-512v-128zM1792 640v-480q0 -66 -47 -113t-113 -47h-1472q-66 0 -113 47t-47 113v480h672v-160q0 -26 19 -45t45 -19h320q26 0 45 19t19 45v160h672zM1024 640v-128h-256v128h256zM1792 1120v-384h-1792v384q0 66 47 113t113 47h352v160q0 40 28 68t68 28h576q40 0 68 -28t28 -68v-160h352q66 0 113 -47t47 -113z")),
        "envelope" => Some((1792, "M1792 826v-794q0 -66 -47 -113t-113 -47h-1472q-66 0 -113 47t-47 113v794q44 -49 101 -87q362 -246 497 -345q57 -42 92.5 -65.5t94.5 -48t110 -24.5h1h1q51 0 110 24.5t94.5 48t92.5 65.5q170 123 498 345q57 39 100 87zM1792 1120q0 -79 -49 -151t-122 -123q-376 -261 -468 -325q-10 -7 -42.5 -30.5t-54 -38t-52 -32.5t-57.5 -27t-50 -9h-1h-1q-23 0 -50 9t-57.5 27t-52 32.5t-54 38t-42.5 30.5q-91 64 -262 182.5t-205 142.5q-62 42 -117 115.5t-55 136.5q0 78 41.5 130t118.5 52h1472q65 0 112.5 -47t47.5 -113z")),
        "rocket" => Some((1664, "M1440 1088q0 40 -28 68t-68 28t-68 -28t-28 -68t28 -68t68 -28t68 28t28 68zM1664 1376q0 -249 -75.5 -430.5t-253.5 -360.5q-81 -80 -195 -176l-20 -379q-2 -16 -16 -26l-384 -224q-7 -4 -16 -4q-12 0 -23 9l-64 64q-13 14 -8 32l85 276l-281 281l-276 -85q-3 -1 -9 -1q-14 0 -23 9l-64 64q-17 19 -5 39l224 384q10 14 26 16l379 20q96 114 176 195q188 187 358 258t431 71q14 0 24 -9.5t10 -22.5z")),
        "archive" => Some((1792, "M1088 704q0 26 -19 45t-45 19h-256q-26 0 -45 -19t-19 -45t19 -45t45 -19h256q26 0 45 19t19 45zM1664 896v-960q0 -26 -19 -45t-45 -19h-1408q-26 0 -45 19t-19 45v960q0 26 19 45t45 19h1408q26 0 45 -19t19 -45zM1728 1344v-256q0 -26 -19 -45t-45 -19h-1536q-26 0 -45 19t-19 45v256q0 26 19 45t45 19h1536q26 0 45 -19t19 -45z")),
        "globe" => Some((1536, "M768 1408q209 0 385.5 -103t279.5 -279.5t103 -385.5t-103 -385.5t-279.5 -279.5t-385.5 -103t-385.5 103t-279.5 279.5t-103 385.5t103 385.5t279.5 279.5t385.5 103zM1042 887q-2 -1 -9.5 -9.5t-13.5 -9.5q2 0 4.5 5t5 11t3.5 7q6 7 22 15q14 6 52 12q34 8 51 -11 q-2 2 9.5 13t14.5 12q3 2 15 4.5t15 7.5l2 22q-12 -1 -17.5 7t-6.5 21q0 -2 -6 -8q0 7 -4.5 8t-11.5 -1t-9 -1q-10 3 -15 7.5t-8 16.5t-4 15q-2 5 -9.5 10.5t-9.5 10.5q-1 2 -2.5 5.5t-3 6.5t-4 5.5t-5.5 2.5t-7 -5t-7.5 -10t-4.5 -5q-3 2 -6 1.5t-4.5 -1t-4.5 -3t-5 -3.5 q-3 -2 -8.5 -3t-8.5 -2q15 5 -1 11q-10 4 -16 3q9 4 7.5 12t-8.5 14h5q-1 4 -8.5 8.5t-17.5 8.5t-13 6q-8 5 -34 9.5t-33 0.5q-5 -6 -4.5 -10.5t4 -14t3.5 -12.5q1 -6 -5.5 -13t-6.5 -12q0 -7 14 -15.5t10 -21.5q-3 -8 -16 -16t-16 -12q-5 -8 -1.5 -18.5t10.5 -16.5 q2 -2 1.5 -4t-3.5 -4.5t-5.5 -4t-6.5 -3.5l-3 -2q-11 -5 -20.5 6t-13.5 26q-7 25 -16 30q-23 8 -29 -1q-5 13 -41 26q-25 9 -58 4q6 1 0 15q-7 15 -19 12q3 6 4 17.5t1 13.5q3 13 12 23q1 1 7 8.5t9.5 13.5t0.5 6q35 -4 50 11q5 5 11.5 17t10.5 17q9 6 14 5.5t14.5 -5.5 t14.5 -5q14 -1 15.5 11t-7.5 20q12 -1 3 17q-5 7 -8 9q-12 4 -27 -5q-8 -4 2 -8q-1 1 -9.5 -10.5t-16.5 -17.5t-16 5q-1 1 -5.5 13.5t-9.5 13.5q-8 0 -16 -15q3 8 -11 15t-24 8q19 12 -8 27q-7 4 -20.5 5t-19.5 -4q-5 -7 -5.5 -11.5t5 -8t10.5 -5.5t11.5 -4t8.5 -3 q14 -10 8 -14q-2 -1 -8.5 -3.5t-11.5 -4.5t-6 -4q-3 -4 0 -14t-2 -14q-5 5 -9 17.5t-7 16.5q7 -9 -25 -6l-10 1q-4 0 -16 -2t-20.5 -1t-13.5 8q-4 8 0 20q1 4 4 2q-4 3 -11 9.5t-10 8.5q-46 -15 -94 -41q6 -1 12 1q5 2 13 6.5t10 5.5q34 14 42 7l5 5q14 -16 20 -25 q-7 4 -30 1q-20 -6 -22 -12q7 -12 5 -18q-4 3 -11.5 10t-14.5 11t-15 5q-16 0 -22 -1q-146 -80 -235 -222q7 -7 12 -8q4 -1 5 -9t2.5 -11t11.5 3q9 -8 3 -19q1 1 44 -27q19 -17 21 -21q3 -11 -10 -18q-1 2 -9 9t-9 4q-3 -5 0.5 -18.5t10.5 -12.5q-7 0 -9.5 -16t-2.5 -35.5 t-1 -23.5l2 -1q-3 -12 5.5 -34.5t21.5 -19.5q-13 -3 20 -43q6 -8 8 -9q3 -2 12 -7.5t15 -10t10 -10.5q4 -5 10 -22.5t14 -23.5q-2 -6 9.5 -20t10.5 -23q-1 0 -2.5 -1t-2.5 -1q3 -7 15.5 -14t15.5 -13q1 -3 2 -10t3 -11t8 -2q2 20 -24 62q-15 25 -17 29q-3 5 -5.5 15.5 t-4.5 14.5q2 0 6 -1.5t8.5 -3.5t7.5 -4t2 -3q-3 -7 2 -17.5t12 -18.5t17 -19t12 -13q6 -6 14 -19.5t0 -13.5q9 0 20 -10t17 -20q5 -8 8 -26t5 -24q2 -7 8.5 -13.5t12.5 -9.5l16 -8t13 -7q5 -2 18.5 -10.5t21.5 -11.5q10 -4 16 -4t14.5 2.5t13.5 3.5q15 2 29 -15t21 -21 q36 -19 55 -11q-2 -1 0.5 -7.5t8 -15.5t9 -14.5t5.5 -8.5q5 -6 18 -15t18 -15q6 4 7 9q-3 -8 7 -20t18 -10q14 3 14 32q-31 -15 -49 18q0 1 -2.5 5.5t-4 8.5t-2.5 8.5t0 7.5t5 3q9 0 10 3.5t-2 12.5t-4 13q-1 8 -11 20t-12 15q-5 -9 -16 -8t-16 9q0 -1 -1.5 -5.5t-1.5 -6.5 q-13 0 -15 1q1 3 2.5 17.5t3.5 22.5q1 4 5.5 12t7.5 14.5t4 12.5t-4.5 9.5t-17.5 2.5q-19 -1 -26 -20q-1 -3 -3 -10.5t-5 -11.5t-9 -7q-7 -3 -24 -2t-24 5q-13 8 -22.5 29t-9.5 37q0 10 2.5 26.5t3 25t-5.5 24.5q3 2 9 9.5t10 10.5q2 1 4.5 1.5t4.5 0t4 1.5t3 6q-1 1 -4 3 q-3 3 -4 3q7 -3 28.5 1.5t27.5 -1.5q15 -11 22 2q0 1 -2.5 9.5t-0.5 13.5q5 -27 29 -9q3 -3 15.5 -5t17.5 -5q3 -2 7 -5.5t5.5 -4.5t5 0.5t8.5 6.5q10 -14 12 -24q11 -40 19 -44q7 -3 11 -2t4.5 9.5t0 14t-1.5 12.5l-1 8v18l-1 8q-15 3 -18.5 12t1.5 18.5t15 18.5q1 1 8 3.5 t15.5 6.5t12.5 8q21 19 15 35q7 0 11 9q-1 0 -5 3t-7.5 5t-4.5 2q9 5 2 16q5 3 7.5 11t7.5 10q9 -12 21 -2q7 8 1 16q5 7 20.5 10.5t18.5 9.5q7 -2 8 2t1 12t3 12q4 5 15 9t13 5l17 11q3 4 0 4q18 -2 31 11q10 11 -6 20q3 6 -3 9.5t-15 5.5q3 1 11.5 0.5t10.5 1.5 q15 10 -7 16q-17 5 -43 -12zM879 10q206 36 351 189q-3 3 -12.5 4.5t-12.5 3.5q-18 7 -24 8q1 7 -2.5 13t-8 9t-12.5 8t-11 7q-2 2 -7 6t-7 5.5t-7.5 4.5t-8.5 2t-10 -1l-3 -1q-3 -1 -5.5 -2.5t-5.5 -3t-4 -3t0 -2.5q-21 17 -36 22q-5 1 -11 5.5t-10.5 7t-10 1.5t-11.5 -7 q-5 -5 -6 -15t-2 -13q-7 5 0 17.5t2 18.5q-3 6 -10.5 4.5t-12 -4.5t-11.5 -8.5t-9 -6.5t-8.5 -5.5t-8.5 -7.5q-3 -4 -6 -12t-5 -11q-2 4 -11.5 6.5t-9.5 5.5q2 -10 4 -35t5 -38q7 -31 -12 -48q-27 -25 -29 -40q-4 -22 12 -26q0 -7 -8 -20.5t-7 -21.5q0 -6 2 -16z")),
        _ => None,
    }
}

/// The five social marks used by the migrated homepage, retained as the small
/// Font Awesome 4.5 outline subset rather than shipping a font or framework.
/// Font coordinates are transformed from their original y-up 1792-unit em.
fn brand_icon(name: &str) -> Option<(u16, &'static str)> {
    match name {
        "github" => Some((1536, "M1536 640q0 -251 -146.5 -451.5t-378.5 -277.5q-27 -5 -39.5 7t-12.5 30v211q0 97 -52 142q57 6 102.5 18t94 39t81 66.5t53 105t20.5 150.5q0 121 -79 206q37 91 -8 204q-28 9 -81 -11t-92 -44l-38 -24q-93 26 -192 26t-192 -26q-16 11 -42.5 27t-83.5 38.5t-86 13.5q-44 -113 -7 -204q-79 -85 -79 -206q0 -85 20.5 -150t52.5 -105t80.5 -67t94 -39t102.5 -18q-40 -36 -49 -103q-21 -10 -45 -15t-57 -5t-65.5 21.5t-55.5 62.5q-19 32 -48.5 52t-49.5 24l-20 3q-21 0 -29 -4.5t-5 -11.5t9 -14t13 -12l7 -5q22 -10 43.5 -38t31.5 -51l10 -23q13 -38 44 -61.5t67 -30t69.5 -7t55.5 3.5l23 4q0 -38 .5 -89t.5 -54q0 -18 -13 -30t-40 -7q-232 77 -378.5 277.5t-146.5 451.5q0 209 103 385.5t279.5 279.5t385.5 103t385.5 -103t279.5 -279.5t103 -385.5z")),
        "linkedin" => Some((1536, "M237 122h231v694h-231v-694zM483 1030q-1 52 -36 86t-93 34t-94.5 -34t-36.5 -86q0 -51 35.5 -85.5t92.5 -34.5h1q59 0 95 34.5t36 85.5zM1068 122h231v398q0 154 -73 233t-193 79q-136 0 -209 -117h2v101h-231q3 -66 0 -694h231v388q0 38 7 56q15 35 45 59.5t74 24.5q116 0 116 -157v-371zM1536 1120v-960q0 -119 -84.5 -203.5t-203.5 -84.5h-960q-119 0 -203.5 84.5t-84.5 203.5v960q0 119 84.5 203.5t203.5 84.5h960q119 0 203.5 -84.5t84.5 -203.5z")),
        "twitter" => Some((1664, "M1620 1128q-67 -98 -162 -167q1 -14 1 -42q0 -130 -38 -259.5t-115.5 -248.5t-184.5 -210.5t-258 -146t-323 -54.5q-271 0 -496 145q35 -4 78 -4q225 0 401 138q-105 2 -188 64.5t-114 159.5q33 -5 61 -5q43 0 85 11q-112 23 -185.5 111.5t-73.5 205.5v4q68 -38 146 -41q-66 44 -105 115t-39 154q0 88 44 163q121 -149 294.5 -238.5t371.5 -99.5q-8 38 -8 74q0 134 94.5 228.5t228.5 94.5q140 0 236 -102q109 21 205 78q-37 -115 -142 -178q93 10 186 50z")),
        "youtube" | "youtube-square" => Some((1536, "M971 292v-211q0 -67 -39 -67q-23 0 -45 22v301q22 22 45 22q39 0 39 -67zM1309 291v-46h-90v46q0 68 45 68t45 -68zM343 509h107v94h-312v-94h105v-569h100v569zM631 -60h89v494h-89v-378q-30 -42 -57 -42q-18 0 -21 21q-1 3 -1 35v364h-89v-391q0 -49 8 -73q12 -37 58 -37q48 0 102 61v-54zM1060 88v197q0 73 -9 99q-17 56 -71 56q-50 0 -93 -54v217h-89v-663h89v48q45 -55 93 -55q54 0 71 55q9 27 9 100zM1398 98v13h-91q0 -51 -2 -61q-7 -36 -40 -36q-46 0 -46 69v87h179v103q0 79 -27 116q-39 51 -106 51q-68 0 -107 -51q-28 -37 -28 -116v-173q0 -79 29 -116q39 -51 108 -51q72 0 108 53q18 27 21 54q2 9 2 58zM790 1011v210q0 69 -43 69t-43 -69v-210q0 -70 43 -70t43 70zM1509 260q0 -234 -26 -350q-14 -59 -58 -99t-102 -46q-184 -21 -555 -21t-555 21q-58 6 -102.5 46t-57.5 99q-26 112 -26 350q0 234 26 350q14 59 58 99t103 47q183 20 554 20t555 -20q58 -7 102.5 -47t57.5 -99q26 -112 26 -350zM511 1536h102l-121 -399v-271h-100v271q-14 74 -61 212q-37 103 -65 187h106l71 -263zM881 1203v-175q0 -81 -28 -118q-37 -51 -106 -51q-67 0 -105 51q-28 38 -28 118v175q0 80 28 117q38 51 105 51q69 0 106 -51q28 -37 28 -117zM1216 1365v-499h-91v55q-53 -62 -103 -62q-46 0 -59 37q-8 24 -8 75v394h91v-367q0 -33 1 -35q3 -22 21 -22q27 0 57 43v381h91z")),
        "reddit" => Some((1792, "M1095 369q16 -16 0 -31q-62 -62 -199 -62t-199 62q-16 15 0 31q6 6 15 6t15 -6q48 -49 169 -49q120 0 169 49q6 6 15 6t15 -6zM788 550q0 -37 -26 -63t-63 -26t-63.5 26t-26.5 63q0 38 26.5 64t63.5 26t63 -26.5t26 -63.5zM1183 550q0 -37 -26.5 -63t-63.5 -26t-63 26t-26 63t26 63.5t63 26.5t63.5 -26t26.5 -64zM1434 670q0 49 -35 84t-85 35t-86 -36q-130 90 -311 96l63 283l200 -45q0 -37 26 -63t63 -26t63.5 26.5t26.5 63.5t-26.5 63.5t-63.5 26.5q-54 0 -80 -50l-221 49q-19 5 -25 -16l-69 -312q-180 -7 -309 -97q-35 37 -87 37q-50 0 -85 -35t-35 -84q0 -35 18.5 -64t49.5 -44q-6 -27 -6 -56q0 -142 140 -243t337 -101q198 0 338 101t140 243q0 32 -7 57q30 15 48 43.5t18 63.5zM1792 640q0 -182 -71 -348t-191 -286t-286 -191t-348 -71t-348 71t-286 191t-191 286t-71 348t71 348t191 286t286 191t348 71t348 -71t286 -191t191 -286t71 -348z")),
        _ => None,
    }
}

#[function_component(Typewriter)]
fn typewriter(props: &TypewriterProps) -> Html {
    let first = props.roles.first().cloned().unwrap_or_default();
    let state = use_state(|| {
        if prefers_reduced_motion() {
            TypewriterState {
                visible: first,
                ..TypewriterState::default()
            }
        } else {
            TypewriterState::default()
        }
    });
    {
        let state = state.clone();
        let roles = props.roles.clone();
        use_effect_with((*state).clone(), move |current| {
            let mut next = current.clone();
            advance_typewriter(&mut next, &roles);
            let reduced = prefers_reduced_motion();
            // Typed.js adds a 0..70 ms humanizer to the configured 50 ms
            // speed. A deterministic 85 ms midpoint preserves the observed
            // legacy pace while keeping screenshots and tests reproducible.
            let timeout = (!reduced).then(|| Timeout::new(85, move || state.set(next)));
            move || drop(timeout)
        });
    }
    html! { <h2 class="typewriter" data-text={state.visible.clone()}>{&state.visible}</h2> }
}

#[derive(Properties, PartialEq)]
struct TypewriterProps {
    roles: Vec<String>,
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}

const SCRAMBLE_GLYPHS: [char; 32] = [
    '!', '<', '>', '-', '_', '/', '[', ']', '{', '}', '=', '+', '*', '^', '?', '#', 'ｱ', 'ｲ', 'ｳ',
    'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｼ', 'ｽ', 'ﾀ', 'ﾂ', 'ﾅ', 'ﾒ', 'ﾗ', 'ﾘ',
];

#[derive(Clone, Copy, PartialEq)]
enum ScramblePhase {
    Decoding(u16),
    Waiting(u16),
    Glitching { cycle: u16, frame: u8 },
}

#[derive(Clone, Copy, Default, PartialEq)]
enum GlitchProfile {
    #[default]
    Title,
    Navigation,
    Brand,
    PostTitle,
    Subtitle,
}

#[derive(Properties, PartialEq)]
struct ScrambleTitleProps {
    text: String,
    #[prop_or_default]
    profile: GlitchProfile,
}

#[function_component(ScrambleTitle)]
fn scramble_title(props: &ScrambleTitleProps) -> Html {
    let initial_profile = props.profile;
    let phase = use_state(move || {
        if prefers_reduced_motion() || initial_profile == GlitchProfile::Subtitle {
            ScramblePhase::Waiting(0)
        } else {
            ScramblePhase::Decoding(0)
        }
    });
    {
        let phase = phase.clone();
        let profile = props.profile;
        use_effect_with((props.text.clone(), profile), move |_| {
            phase.set(
                if prefers_reduced_motion() || profile == GlitchProfile::Subtitle {
                    ScramblePhase::Waiting(0)
                } else {
                    ScramblePhase::Decoding(0)
                },
            );
            || ()
        });
    }
    {
        let phase = phase.clone();
        let text = props.text.clone();
        let profile = props.profile;
        use_effect_with((*phase, profile), move |(current, profile)| {
            let timeout = if prefers_reduced_motion() {
                None
            } else {
                match *current {
                    ScramblePhase::Decoding(frame) => {
                        let next = if frame >= scramble_decode_frames(&text, *profile) {
                            ScramblePhase::Waiting(0)
                        } else {
                            ScramblePhase::Decoding(frame + 1)
                        };
                        Some(Timeout::new(scramble_frame_delay(*profile), move || {
                            phase.set(next)
                        }))
                    }
                    ScramblePhase::Waiting(cycle) => {
                        let delay = scramble_glitch_delay(&text, cycle, *profile);
                        Some(Timeout::new(delay, move || {
                            phase.set(ScramblePhase::Glitching { cycle, frame: 0 })
                        }))
                    }
                    ScramblePhase::Glitching { cycle, frame } => {
                        let last_frame = match profile {
                            GlitchProfile::Brand
                            | GlitchProfile::Navigation
                            | GlitchProfile::Title
                            | GlitchProfile::PostTitle => 1,
                            GlitchProfile::Subtitle => 1,
                        };
                        let next = if frame >= last_frame {
                            ScramblePhase::Waiting(cycle.wrapping_add(1))
                        } else {
                            ScramblePhase::Glitching {
                                cycle,
                                frame: frame + 1,
                            }
                        };
                        Some(Timeout::new(60, move || phase.set(next)))
                    }
                }
            };
            move || drop(timeout)
        });
    }
    let display = if prefers_reduced_motion() {
        props.text.clone()
    } else {
        scramble_display(&props.text, *phase, props.profile)
    };
    let state_class = match *phase {
        ScramblePhase::Decoding(_) => "is-decoding",
        ScramblePhase::Waiting(_) => "is-locked",
        ScramblePhase::Glitching { .. } => "is-glitching",
    };
    html! {
        <span class={classes!("faqe-scramble", state_class, (props.profile == GlitchProfile::Subtitle).then_some("is-subtitle"))} aria-label={props.text.clone()}>
            <span class="faqe-scramble-channel faqe-scramble-channel-a" aria-hidden="true">{scramble_text_view(&display, &props.text)}</span>
            <span class="faqe-scramble-label" aria-hidden="true">{scramble_text_view(&display, &props.text)}</span>
            <span class="faqe-scramble-channel faqe-scramble-channel-b" aria-hidden="true">{scramble_text_view(&display, &props.text)}</span>
        </span>
    }
}

fn scramble_text_view(text: &str, original: &str) -> Html {
    let original = original.chars().collect::<Vec<_>>();
    html! {
        <>{for text.chars().enumerate().map(|(index, character)| {
            if original.get(index).is_some_and(|value| *value != character) {
                let kana = matches!(
                    character,
                    '\u{30a0}'..='\u{30ff}' | '\u{31f0}'..='\u{31ff}' | '\u{ff65}'..='\u{ff9f}'
                );
                html! { <span class={classes!("faqe-scramble-symbol", kana.then_some("faqe-scramble-kana"))}>{character}</span> }
            } else {
                Html::from(character.to_string())
            }
        })}</>
    }
}

fn scramble_decode_frames(text: &str, profile: GlitchProfile) -> u16 {
    let characters = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count()
        .min(u16::MAX as usize) as u16;
    match profile {
        GlitchProfile::Brand => 7_u16.saturating_add(characters),
        GlitchProfile::Navigation => 6_u16.saturating_add(characters),
        GlitchProfile::Title => 6_u16.saturating_add(characters),
        GlitchProfile::PostTitle => 14,
        GlitchProfile::Subtitle => 0,
    }
}

fn scramble_frame_delay(profile: GlitchProfile) -> u32 {
    match profile {
        GlitchProfile::Brand => 70,
        GlitchProfile::Navigation => 55,
        GlitchProfile::Title => 55,
        GlitchProfile::PostTitle => 48,
        GlitchProfile::Subtitle => 60,
    }
}

fn scramble_display(text: &str, phase: ScramblePhase, profile: GlitchProfile) -> String {
    match phase {
        ScramblePhase::Decoding(frame) => scramble_decoding_text(text, frame, profile),
        ScramblePhase::Waiting(_) => text.to_owned(),
        ScramblePhase::Glitching { cycle, frame } => {
            scramble_glitched_text(text, cycle, frame, profile)
        }
    }
}

fn scramble_decoding_text(text: &str, frame: u16, profile: GlitchProfile) -> String {
    if frame >= scramble_decode_frames(text, profile) {
        return text.to_owned();
    }
    if profile != GlitchProfile::Subtitle {
        let warmup = match profile {
            GlitchProfile::Brand => 7,
            GlitchProfile::Navigation => 6,
            GlitchProfile::Title => 6,
            GlitchProfile::PostTitle => 3,
            GlitchProfile::Subtitle => unreachable!(),
        };
        let characters = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .count();
        let reveal_frame = frame.saturating_sub(warmup) as usize;
        let revealed = if profile == GlitchProfile::PostTitle {
            let reveal_frames =
                scramble_decode_frames(text, profile).saturating_sub(warmup) as usize;
            characters.saturating_mul(reveal_frame) / reveal_frames.max(1)
        } else {
            reveal_frame
        };
        let seed = scramble_hash(text);
        let mut visible_index = 0;
        return text
            .chars()
            .enumerate()
            .map(|(index, character)| {
                if character.is_whitespace() {
                    character
                } else {
                    let output = if visible_index < revealed {
                        character
                    } else {
                        scramble_glyph(seed, index, frame as usize)
                    };
                    visible_index += 1;
                    output
                }
            })
            .collect();
    }
    text.to_owned()
}

fn scramble_glitched_text(text: &str, cycle: u16, frame: u8, profile: GlitchProfile) -> String {
    let positions = text
        .chars()
        .enumerate()
        .filter_map(|(index, character)| character.is_alphanumeric().then_some(index))
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return text.to_owned();
    }
    let seed = scramble_hash(text).wrapping_add(cycle as u64 * 0x9e37_79b9);
    let count = match profile {
        GlitchProfile::Navigation => 1,
        _ => 1 + seed as usize % 3,
    }
    .min(positions.len());
    let start = seed as usize % positions.len();
    let step = ((seed >> 17) as usize % positions.len()).max(1);
    let mut glitched = Vec::with_capacity(count);
    for offset in 0..positions.len() {
        let candidate = positions[(start + offset * step) % positions.len()];
        if !glitched.contains(&candidate) {
            glitched.push(candidate);
        }
        if glitched.len() == count {
            break;
        }
    }
    for candidate in positions.iter().copied() {
        if glitched.len() == count {
            break;
        }
        if !glitched.contains(&candidate) {
            glitched.push(candidate);
        }
    }
    text.chars()
        .enumerate()
        .map(|(index, character)| {
            if glitched.contains(&index) {
                match profile {
                    GlitchProfile::Brand
                    | GlitchProfile::Navigation
                    | GlitchProfile::Title
                    | GlitchProfile::PostTitle => scramble_glyph(seed, index, frame as usize),
                    GlitchProfile::Subtitle => {
                        shuffled_letter(seed, index, frame as usize, character)
                    }
                }
            } else {
                character
            }
        })
        .collect()
}

fn scramble_glitch_delay(text: &str, cycle: u16, profile: GlitchProfile) -> u32 {
    let entropy = scramble_hash(text).wrapping_add(cycle as u64 * 2_654_435_761) as u32;
    match profile {
        GlitchProfile::Brand => 1_800 + entropy % 1_801,
        GlitchProfile::Navigation => 6_500 + entropy % 5_501,
        GlitchProfile::Title => 1_500 + entropy % 1_801,
        GlitchProfile::PostTitle => 1_700 + entropy % 2_001,
        GlitchProfile::Subtitle => 10_000 + entropy % 10_001,
    }
}

fn scramble_glyph(seed: u64, index: usize, frame: usize) -> char {
    let offset = seed
        .wrapping_add(index as u64 * 97)
        .wrapping_add(frame as u64 * 53) as usize;
    SCRAMBLE_GLYPHS[offset % SCRAMBLE_GLYPHS.len()]
}

fn shuffled_letter(seed: u64, index: usize, frame: usize, original: char) -> char {
    let offset = seed
        .wrapping_add(index as u64 * 67)
        .wrapping_add(frame as u64 * 41) as u8;
    if original.is_ascii_digit() {
        char::from(b'0' + offset % 10)
    } else if original.is_uppercase() {
        char::from(b'A' + offset % 26)
    } else {
        char::from(b'a' + offset % 26)
    }
}

fn scramble_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x100_0000_01b3)
    })
}

#[derive(Clone, Default, PartialEq)]
struct TypewriterState {
    role: usize,
    visible: String,
    deleting: bool,
    pause_ticks: u8,
}

fn advance_typewriter(state: &mut TypewriterState, roles: &[String]) {
    if roles.is_empty() {
        return;
    }
    if state.pause_ticks > 0 {
        state.pause_ticks -= 1;
        return;
    }
    let target = &roles[state.role % roles.len()];
    if state.deleting {
        let next_role = (state.role + 1) % roles.len();
        let next_target = &roles[next_role];
        let shared_prefix = target
            .chars()
            .zip(next_target.chars())
            .take_while(|(left, right)| left == right)
            .count();
        if state.visible.chars().count() > shared_prefix {
            state.visible.pop();
        } else {
            state.deleting = false;
            state.role = next_role;
        }
    } else if state.visible.chars().count() < target.chars().count() {
        if let Some(character) = target.chars().nth(state.visible.chars().count()) {
            state.visible.push(character);
        }
    } else {
        state.pause_ticks = 12;
        state.deleting = true;
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Properties, PartialEq)]
struct PageProps {
    bundle: SiteBundle,
    page: Page,
}

#[function_component(PageView)]
fn page_view(props: &PageProps) -> Html {
    if props.page.content_type == "identity" {
        let key_route = props
            .bundle
            .page_of_type("key")
            .map(|page| page.route.clone())
            .unwrap_or_else(|| "/".into());
        return html! { <LogoPage page={props.page.clone()} key_route={key_route} /> };
    }
    match props.page.kind {
        PageKind::Post => {
            html! { <PostPage bundle={props.bundle.clone()} page={props.page.clone()} /> }
        }
        PageKind::Section => {
            html! { <ListPage bundle={props.bundle.clone()} page={props.page.clone()} /> }
        }
        PageKind::Resume => {
            html! { <ResumePage bundle={props.bundle.clone()} page={props.page.clone()} /> }
        }
        PageKind::Front => html! { <FrontPage page={props.page.clone()} /> },
        PageKind::Talk => Html::default(),
    }
}

#[function_component(LogoPage)]
fn logo_page(props: &LogoPageProps) -> Html {
    html! {
        <section class="container centered estate logo-page">
            <h1 class="faqe-visually-hidden">{&props.page.title}</h1>
            {logo_document_view(&props.page.document, &props.key_route)}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct OnePageProps {
    page: Page,
}

#[derive(Properties, PartialEq)]
struct LogoPageProps {
    page: Page,
    key_route: String,
}

#[function_component(FrontPage)]
fn front_page(props: &OnePageProps) -> Html {
    let title_missing = !props.page.has_explicit_title || props.page.title.is_empty();
    let hidden_title = if title_missing {
        html! { <h1 class="faqe-visually-hidden">{&props.page.title}</h1> }
    } else {
        Html::default()
    };
    let title = if title_missing {
        Html::default()
    } else {
        html! { <h1 class={classes!(props.page.slug.clone(), "title", "glitchtitle")}><ScrambleTitle text={props.page.title.clone()} /></h1> }
    };
    html! {
        <section class="container list estate"><article>
            {hidden_title}
            {title}
            <span class="frontpost">{document_view(&props.page.document)}</span>
        </article></section>
    }
}

#[function_component(PostPage)]
fn post_page(props: &PageProps) -> Html {
    let page = &props.page;
    let site = &props.bundle.site;
    {
        let route = page.route.clone();
        use_effect_with(route, move |_| {
            update_reading_progress();
            let scroll = web_sys::window().map(|window| {
                EventListener::new(&window, "scroll", move |_| update_reading_progress())
            });
            let resize = web_sys::window().map(|window| {
                EventListener::new(&window, "resize", move |_| update_reading_progress())
            });
            move || {
                drop(scroll);
                drop(resize);
            }
        });
    }
    html! {
        <section class="container page estate"><article>
            <header>
                <h1 class="title glitchtitle"><ScrambleTitle text={page.title.clone()} profile={GlitchProfile::PostTitle} /></h1>
                <div class="undertitle"><details class="dropdown"><summary><span id="title_part" class="post-index-label">{" INDEX "}</span><span>{" | "}</span>{taxonomy_summary(page)}</summary>
                    <nav id="TableOfContents" aria-label="Table of contents">{table_of_contents(&page.table_of_contents)}</nav>
                </details></div>
            </header>
            <span class="blogpost">
                {page.punchline.as_ref().map(|text| html! { <div class="punchline"><ScrambleTitle text={text.clone()} profile={GlitchProfile::Subtitle} /></div> }).unwrap_or_default()}
                {page.punchline.as_ref().map(|_| disclaimer(site)).unwrap_or_default()}
                {page.description.as_ref().map(|text| html! { <div class="description">{"In this short post, were gonna talk how to: "}{text}</div> }).unwrap_or_default()}
                {page.tldr.as_ref().filter(|text| !text.is_empty()).map(|text| html! { <div class="tldr"><ScrambleTitle text={"TL;DR".to_owned()} profile={GlitchProfile::Subtitle} /><div class="tldr-body">{text}</div></div> }).unwrap_or_default()}
                <div class="article-separator"><hr /></div>
                {article_document_view(&page.document)}
            </span>
            {references(page, site)}
        </article></section>
    }
}

fn taxonomy_summary(page: &Page) -> Html {
    let categories = taxonomy_text("Category", "categories", &page.categories);
    let tags = taxonomy_text("Tags", "tags", &page.tags);
    html! {
        <span class="taxonomy-summary">
            {categories}
            {tags}
            {page.external_link.as_ref().map(|link| html! { <span class="title_posturl">{"Url: "}<a href={safe_href(link)}>{link}</a><span>{" | "}</span></span> }).unwrap_or_default()}
            <span class="series-summary">
                {if page.series.is_empty() { Html::default() } else { html! { <>{"Series: "}{for page.series.iter().map(|value| html! { <><a href={site_url(&format!("/series/{}/", faqe_model::slugify(value)))}>{value}</a>{" "}</> })}</> } }}
                {page.part.as_ref().map(|part| html! { <span class="title-part">{format!(" PART {part} ")}</span> }).unwrap_or_default()}
                <span>{"| "}</span>
            </span>
        </span>
    }
}

fn taxonomy_text(label: &str, taxonomy: &str, values: &[String]) -> Html {
    if values.is_empty() {
        Html::default()
    } else {
        html! { <span>{format!("{label}: ")}{for values.iter().map(|value| html! { <><a href={site_url(&format!("/{taxonomy}/{}/", faqe_model::slugify(value)))}>{value}</a>{" "}</> })}<span>{"| "}</span></span> }
    }
}

fn disclaimer(site: &faqe_model::SiteMetadata) -> Html {
    if site.disclaimer_title.is_empty() || site.disclaimer_paragraphs.is_empty() {
        return Html::default();
    }
    html! {
        <div class="hideframeholder disclaimer"><div class="hide_box"><div class="disclaimer-inner"><details><summary>{&site.disclaimer_title}</summary><div class="disclaimer-content">
            {for site.disclaimer_paragraphs.iter().enumerate().map(|(index, paragraph)| html! { <>{(index > 0).then_some(html! { <hr /> }).unwrap_or_default()}<div>{paragraph}</div></> })}
        </div></details></div></div></div>
    }
}

fn references(page: &Page, site: &faqe_model::SiteMetadata) -> Html {
    if page.credits.is_empty() {
        return Html::default();
    }
    html! {
        <div class="references-wrap"><div class="references"><details class="references-details"><summary>{"References"}</summary>
            <div class="references-links">{for page.credits.iter().map(|credit| html! { <div>{"- "}<a href={credit.clone()}>{credit}</a></div> })}</div>
            {if site.references_copyright.is_empty() && site.references_notice.is_empty() { Html::default() } else { html! {
                <div class="references-license"><div class="references-license-inner">
                    {(!site.references_copyright.is_empty()).then_some(html! { <div>{&site.references_copyright}</div> }).unwrap_or_default()}
                    {(!site.references_copyright.is_empty() && !site.references_notice.is_empty()).then_some(html! { <hr /> }).unwrap_or_default()}
                    {(!site.references_notice.is_empty()).then_some(html! { <div>{&site.references_notice}</div> }).unwrap_or_default()}
                </div></div>
            }}}
        </details></div></div>
    }
}

fn table_of_contents(items: &[TocItem]) -> Html {
    if items.is_empty() {
        return Html::default();
    }
    let mut index = 0;
    let level = items[0].level;
    html! { <ul class={format!("toc-level-{level}")}>{for toc_level(items, &mut index, level)}</ul> }
}

fn toc_level(items: &[TocItem], index: &mut usize, level: u8) -> Vec<Html> {
    let mut entries = Vec::new();
    while *index < items.len() && items[*index].level == level {
        let item = &items[*index];
        *index += 1;
        let mut child_groups = Vec::new();
        while *index < items.len() && items[*index].level > level {
            let nested_level = items[*index].level;
            let children = toc_level(items, index, nested_level);
            child_groups
                .push(html! { <ul class={format!("toc-level-{nested_level}")}>{children}</ul> });
        }
        entries.push(
            html! { <li><a href={format!("#{}", item.id)}>{&item.title}</a>{child_groups}</li> },
        );
    }
    entries
}

fn update_reading_progress() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let document = gloo_utils::document();
    let Some(root) = document.document_element() else {
        return;
    };
    let viewport = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let total = (root.scroll_height() as f64 - viewport).max(1.0);
    let progress = (window.scroll_y().unwrap_or(0.0) / total * 100.0).clamp(0.0, 100.0);
    let Ok(elements) = document.query_selector_all(".progress_hr") else {
        return;
    };
    for index in 0..elements.length() {
        if let Some(element) = elements
            .item(index)
            .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
        {
            let _ = element.set_attribute("style", &format!("width:{progress:.2}%"));
            let _ = element.set_attribute("data-percent", &format!("{progress:.0}% complete"));
            let _ = element.set_attribute("aria-valuenow", &format!("{progress:.0}"));
        }
    }
}

#[function_component(ListPage)]
fn list_page(props: &PageProps) -> Html {
    let children = list_children(&props.bundle, &props.page);
    let folders = if props.page.folders {
        props
            .bundle
            .pages
            .iter()
            .filter(|page| {
                page.kind == PageKind::Section
                    && page.route != props.page.route
                    && direct_child_route(&props.page.route, &page.route)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    html! {
        <section class="container list estate"><article>
            <h1 class="title glitchtitle"><ScrambleTitle text={props.page.title.clone()} /></h1>
            <div class="thelist"><div class="gridlist">
                {for children.into_iter().map(|page| content_card(page, &props.bundle.site.default_card_thumbnail))}
            </div></div>
            {if folders.is_empty() {
                Html::default()
            } else {
                html! {
                    <>
                        <h3 class="folder-heading"><ScrambleTitle text={"Folders on: ".to_owned()} profile={GlitchProfile::Subtitle} />{folder_breadcrumb(&props.page.route)}</h3>
                        <div class="mainflex"><div class="innerflex folder-list">
                            {for folders.into_iter().map(|folder| {
                                let name = folder
                                    .route
                                    .trim_matches('/')
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or(&folder.title)
                                    .to_owned();
                                html! {
                                    <a class="folder-tile" href={site_url(&folder.route)}>
                                        <span class="folder-tile-inner">
                                            <span class="folder-icon">{icon_view("folder")}</span>
                                            <span class="listertitle"><ScrambleTitle text={name} /></span>
                                        </span>
                                    </a>
                                }
                            })}
                        </div></div>
                    </>
                }
            }}
            {document_view(&props.page.document)}
        </article></section>
    }
}

fn list_children<'a>(bundle: &'a SiteBundle, section: &Page) -> Vec<&'a Page> {
    let mut children = bundle
        .pages
        .iter()
        .filter(|page| {
            page.is_published()
                && page.kind != PageKind::Section
                && page.route.starts_with(&section.route)
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| left.route.cmp(&right.route))
    });
    if section.folders {
        children.truncate(section.page_size);
    }
    children
}

fn folder_breadcrumb(route: &str) -> Html {
    let mut path = String::new();
    html! {
        <span class="folder-path">
            {for route.trim_matches('/').split('/').filter(|part| !part.is_empty()).map(|part| {
                path.push('/');
                path.push_str(part);
                let href = format!("{path}/");
                html! { <><a href={site_url(&href)}><ScrambleTitle text={part.to_owned()} profile={GlitchProfile::Subtitle} /></a>{" / "}</> }
            })}
        </span>
    }
}

fn direct_child_route(parent: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(parent)
        .is_some_and(|remainder| !remainder.trim_matches('/').contains('/'))
}

fn content_card(page: &Page, default_thumbnail: &str) -> Html {
    let palette = faqe_model::accessible_palette(&page.style)
        .expect("site bundle contains only build-validated page styles");
    let thumbnail = page
        .thumbnail
        .as_deref()
        .filter(|thumbnail| !thumbnail.is_empty())
        .unwrap_or(default_thumbnail);
    let card_style = format!(
        "--accent-color:{};--chromatic-a:{};--chromatic-b:{};--card-text-color:{}",
        page.style.accent, page.style.chromatic[0], page.style.chromatic[1], palette.accent_text
    );
    html! {
        <a class="p-2" href={site_url(&page.route)}><div class="relative" style={card_style}>
            <div class="card_image" aria-hidden="true"><img class="card_image" src={site_url(thumbnail)} alt="" /></div>
            <div class="card_date">{format_legacy_date(page.date.as_deref())}</div>
            <div class="card_time">{icon_view("clock")}{" "}{format!("{}'", page.reading_minutes)}</div>
            <div><hr class="card-rule" /></div>
            <div class="card_box"><div class="card_title">
                <ScrambleTitle text={page.title.clone()} profile={GlitchProfile::PostTitle} />
            </div></div>
        </div></a>
    }
}

fn format_legacy_date(date: Option<&str>) -> String {
    let Some(date) = date else {
        return String::new();
    };
    let parts = date.split('-').collect::<Vec<_>>();
    let [year, month, day] = parts.as_slice() else {
        return date.to_owned();
    };
    let month = match *month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return date.to_owned(),
    };
    let day = day.trim_start_matches('0');
    let year = year.get(2..).unwrap_or(year);
    format!("{month} {day}, {year}")
}

#[derive(Clone, Debug, PartialEq)]
struct TaxonomyView {
    title: String,
    routes: Vec<String>,
    root_label: Option<String>,
}

fn taxonomy_view(bundle: &SiteBundle, route: &str) -> Option<TaxonomyView> {
    let parts = route.trim_matches('/').split('/').collect::<Vec<_>>();
    let ([taxonomy] | [taxonomy, _]) = parts.as_slice() else {
        return None;
    };
    if *taxonomy == "folder" {
        return (parts.len() == 1).then(|| TaxonomyView {
            title: "Folder".to_owned(),
            routes: Vec::new(),
            root_label: None,
        });
    }
    let index = match *taxonomy {
        "tags" => &bundle.taxonomies.tags,
        "categories" => &bundle.taxonomies.categories,
        "series" => &bundle.taxonomies.series,
        "type" => &bundle.taxonomies.kinds,
        _ => return None,
    };
    if parts.len() == 1 {
        return Some(TaxonomyView {
            title: humanize(taxonomy),
            routes: Vec::new(),
            root_label: Some(
                match *taxonomy {
                    "tags" => "Tag",
                    "categories" => "Category",
                    "series" => "Series",
                    // The archived Hugo template had no singular label for
                    // this custom taxonomy, so its first heading was exactly
                    // `: ` rather than `Type: `. Keep that oddity as visual
                    // compatibility evidence instead of normalizing it.
                    "type" => "",
                    _ => unreachable!(),
                }
                .to_owned(),
            ),
        });
    }
    let term = parts[1];
    let routes = index.get(term)?.clone();
    Some(TaxonomyView {
        title: taxonomy_label(bundle, taxonomy, term),
        routes,
        root_label: None,
    })
}

fn taxonomy_label(bundle: &SiteBundle, taxonomy: &str, slug: &str) -> String {
    let mut terms = bundle.pages.iter().flat_map(|page| match taxonomy {
        "tags" => page.tags.iter(),
        "categories" => page.categories.iter(),
        "series" => page.series.iter(),
        _ => [].iter(),
    });
    terms
        .find(|term| faqe_model::slugify(term) == slug)
        .cloned()
        .unwrap_or_else(|| humanize(slug))
}

fn humanize(value: &str) -> String {
    let mut value = value.replace('-', " ");
    if let Some(first) = value.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    value
}

#[derive(Properties, PartialEq)]
struct TaxonomyProps {
    bundle: SiteBundle,
    view: TaxonomyView,
}

#[function_component(TaxonomyPage)]
fn taxonomy_page(props: &TaxonomyProps) -> Html {
    let mut pages = props
        .view
        .routes
        .iter()
        .filter_map(|route| props.bundle.page(route))
        .collect::<Vec<_>>();
    pages.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| left.route.cmp(&right.route))
    });
    html! {
        <section class="container list estate"><article>
            {props.view.root_label.as_ref().map(|label| html! { <div class="title taxonomy-root-label" aria-hidden="true"><ScrambleTitle text={format!("{label}: ")} profile={GlitchProfile::Subtitle} /></div> }).unwrap_or_default()}
            <h1 class="title glitchtitle"><ScrambleTitle text={props.view.title.clone()} /></h1>
            <div class="thelist"><div class="gridlist">{for pages.into_iter().map(|page| content_card(page, &props.bundle.site.default_card_thumbnail))}</div></div>
        </article></section>
    }
}

#[function_component(ResumePage)]
fn resume_page(props: &PageProps) -> Html {
    let Some(resume) = props.page.resume.as_ref() else {
        return html! { <section class="container estate faqe-error"><p>{"Resume data is missing."}</p></section> };
    };
    resume_view(resume, &props.bundle.site)
}

fn resume_view(resume: &ResumeData, site: &SiteMetadata) -> Html {
    html! {
        <div class="wrapper resume">
        <div class="cv estate">
            <aside class="sidebar-wrapper">
                <div class="profile-container">
                    {if resume.profile.is_empty() { Html::default() } else { html! { <img class="profile img-circle" src={site_url(&resume.profile)} alt={site.author.clone()} /> } }}
                    <h1 class="name"><ScrambleTitle text={site.author.clone()} /></h1>
                    <h3 class="tagline"><ScrambleTitle text={site.info.clone()} profile={GlitchProfile::Subtitle} /></h3>
                </div>
                {if resume.contact.enable { html! {
                    <div class="contact-container container-block"><ul class="list-unstyled contact-list">
                        {for resume.contact.list.iter().map(|item| html! {
                            <li class={item.class.clone()}>{icon_view(&item.icon)}<a href={safe_href(&item.url)}>{&item.title}</a></li>
                        })}
                    </ul></div>
                }} else { Html::default() }}
                {if resume.education.enable { html! {
                    <div class="education-container container-block">
                        <h2 class="container-block-title">{icon_view("archive")}<ScrambleTitle text={resume.education.title.to_uppercase()} profile={GlitchProfile::Subtitle} /></h2>
                        {for resume.education.list.iter().map(|item| html! { <>
                            <div class="item"><h4 class="degree"><ScrambleTitle text={item.degree.clone()} profile={GlitchProfile::Subtitle} /></h4><h5 class="meta"><ScrambleTitle text={item.college.clone()} profile={GlitchProfile::Subtitle} /></h5><div class="time">{&item.dates}</div></div><hr />
                        </> })}
                    </div>
                }} else { Html::default() }}
                {if resume.language.enable { html! {
                    <div class="language-container container-block">
                        <h2 class="container-block-title">{icon_view("globe")}<ScrambleTitle text={resume.language.title.to_uppercase()} profile={GlitchProfile::Subtitle} /></h2>
                        <ul class="list-unstyled interests-list">{for resume.language.list.iter().map(|item| html! { <li>{&item.language}{" "}<span class="lang-desc">{format!("({})", item.level)}</span></li> })}</ul>
                    </div>
                }} else { Html::default() }}
                {if resume.interests.enable { html! {
                    <div class="interests-container container-block">
                        <h2 class="container-block-title">{icon_view("rocket")}<ScrambleTitle text={resume.interests.title.to_uppercase()} profile={GlitchProfile::Subtitle} /></h2>
                        <ul class="list-unstyled interests-list">{for resume.interests.list.iter().map(|item| html! { <li>{&item.interest}</li> })}</ul>
                    </div>
                }} else { Html::default() }}
            </aside>
            <div class="main-wrapper">
                {if resume.summary.enable { html! {
                    <section class="section summary-section"><ResumeHeading icon={resume.summary.icon.clone()} title={resume.summary.title.clone()} /><div class="summary">{document_view(&resume.summary.summary_document)}</div></section>
                }} else { Html::default() }}
                {if resume.experiences.enable { html! {
                    <section class="section experiences-section"><ResumeHeading icon={resume.experiences.icon.clone()} title={resume.experiences.title.clone()} />
                        {for resume.jobs.list.iter().map(|job| html! { <div class="item"><div class="meta"><div class="upper-row"><h3 class="job-title"><ScrambleTitle text={job.position.clone()} profile={GlitchProfile::Subtitle} /></h3><div class="time">{&job.dates}</div></div><div class="company">{&job.company}</div></div><div class="details">{document_view(&job.details_document)}</div></div> })}
                    </section>
                }} else { Html::default() }}
                {if resume.projects.enable { html! {
                    <section class="section projects-section"><ResumeHeading icon={resume.projects.icon.clone()} title={resume.projects.title.clone()} /><div class="intro">{document_view(&resume.projects.intro_document)}</div>
                        {for resume.projects.list.iter().map(|project| html! { <><div class="item"><span class="project-title"><a href={safe_href(&project.url)}><ScrambleTitle text={project.title.clone()} profile={GlitchProfile::Subtitle} /></a></span>{" - "}<span class="project-tagline">{inline_document_view(&project.tagline_document)}</span></div><hr /></> })}
                    </section>
                }} else { Html::default() }}
                {if resume.skills.enable { html! {
                    <section class="skills-section section"><ResumeHeading icon={resume.skills.icon.clone()} title={resume.skills.title.clone()} /><div class="skillset">
                        {for resume.skills.list.iter().map(|skill| html! { <div class="item"><h3 class="level-title"><ScrambleTitle text={skill.skill.clone()} profile={GlitchProfile::Subtitle} /></h3><div class="level-bar"><div class="level-bar-inner faqe-skill" style={format!("--skill-level:{}", safe_percentage(&skill.level))}></div></div></div> })}
                    </div></section>
                }} else { Html::default() }}
            </div>
        </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ResumeHeadingProps {
    icon: String,
    title: String,
}

#[function_component(ResumeHeading)]
fn resume_heading(props: &ResumeHeadingProps) -> Html {
    html! { <h2 class="section-title">{icon_view(&props.icon)}<ScrambleTitle text={props.title.clone()} profile={GlitchProfile::Subtitle} /></h2> }
}

fn safe_percentage(value: &str) -> String {
    let number = value
        .trim_end_matches('%')
        .parse::<u8>()
        .unwrap_or(0)
        .min(100);
    format!("{number}%")
}

fn safe_href(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
    {
        "#".into()
    } else {
        trimmed.to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DeckConfig {
    width: f64,
    height: f64,
    margin: f64,
    min_scale: f64,
    max_scale: f64,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            width: 960.0,
            height: 700.0,
            margin: 0.10,
            min_scale: 0.2,
            max_scale: 1.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DeckState {
    horizontal: usize,
    vertical: usize,
    /// Number of fragment groups revealed on the active slide.
    fragment: usize,
    remembered_vertical: Vec<usize>,
}

impl DeckState {
    fn new(groups: &[usize]) -> Self {
        Self {
            horizontal: 0,
            vertical: 0,
            fragment: 0,
            remembered_vertical: vec![0; groups.len()],
        }
    }

    fn position(&self) -> (usize, usize) {
        (self.horizontal, self.vertical)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SlideTransition {
    None,
    Fade,
    #[default]
    Slide,
    Zoom,
}

impl SlideTransition {
    fn parse(value: Option<&String>) -> Self {
        match value.map(String::as_str) {
            Some("none") => Self::None,
            Some("fade") => Self::Fade,
            Some("zoom") => Self::Zoom,
            _ => Self::Slide,
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::None => "transition-none",
            Self::Fade => "transition-fade",
            Self::Slide => "transition-slide",
            Self::Zoom => "transition-zoom",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TransitionSpeed {
    Fast,
    #[default]
    Default,
    Slow,
}

impl TransitionSpeed {
    fn parse(value: Option<&String>) -> Self {
        match value.map(String::as_str) {
            Some("fast") => Self::Fast,
            Some("slow") => Self::Slow,
            _ => Self::Default,
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Fast => "transition-speed-fast",
            Self::Default => "transition-speed-default",
            Self::Slow => "transition-speed-slow",
        }
    }

    fn duration_ms(self) -> u16 {
        match self {
            Self::Fast => 400,
            Self::Default => 800,
            Self::Slow => 1_200,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SlideBackground {
    color: Option<String>,
    image: Option<String>,
}

fn deck_scale(viewport_width: f64, viewport_height: f64, config: DeckConfig) -> f64 {
    let available_width = viewport_width.max(0.0) * (1.0 - config.margin);
    let available_height = viewport_height.max(0.0) * (1.0 - config.margin);
    (available_width / config.width)
        .min(available_height / config.height)
        .clamp(config.min_scale, config.max_scale)
}

fn viewport_size() -> (f64, f64) {
    web_sys::window()
        .map(|window| {
            let layout = (
                window
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(960.0),
                window
                    .inner_height()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(700.0),
            );
            preferred_viewport_size(
                window
                    .visual_viewport()
                    .map(|viewport| (viewport.width(), viewport.height())),
                layout,
            )
        })
        .unwrap_or((960.0, 700.0))
}

fn preferred_viewport_size(visual: Option<(f64, f64)>, layout: (f64, f64)) -> (f64, f64) {
    visual
        .filter(|(width, height)| *width > 0.0 && *height > 0.0)
        .unwrap_or(layout)
}

fn class_has_token(attributes: &std::collections::BTreeMap<String, String>, token: &str) -> bool {
    attributes
        .get("class")
        .is_some_and(|class| class.split_ascii_whitespace().any(|part| part == token))
}

fn is_speaker_notes(element: &ElementNode) -> bool {
    element.tag.eq_ignore_ascii_case("aside") && class_has_token(&element.attributes, "notes")
}

fn collect_fragment_indexes(node: &DocumentNode, indexes: &mut Vec<Option<usize>>) {
    let DocumentNode::Element(element) = node else {
        return;
    };
    // Notes are presenter-only and must never add steps to audience navigation.
    if is_speaker_notes(element) {
        return;
    }
    if class_has_token(&element.attributes, "fragment") {
        indexes.push(
            element
                .attributes
                .get("data-fragment-index")
                .and_then(|value| value.parse::<usize>().ok()),
        );
    }
    for child in &element.children {
        collect_fragment_indexes(child, indexes);
    }
}

/// Return one zero-based reveal group for every fragment in document order.
/// Explicit indexes share a step and sort before unindexed fragments. This
/// mirrors Reveal's useful semantics while keeping malformed indexes harmless.
fn talk_fragment_groups(document: &Document) -> Vec<usize> {
    let mut indexes = Vec::new();
    for node in &document.nodes {
        collect_fragment_indexes(node, &mut indexes);
    }
    let mut explicit = indexes.iter().flatten().copied().collect::<Vec<_>>();
    explicit.sort_unstable();
    explicit.dedup();
    let explicit_groups = explicit.len();
    let mut unindexed = 0;
    indexes
        .into_iter()
        .map(|index| {
            index.map_or_else(
                || {
                    let group = explicit_groups + unindexed;
                    unindexed += 1;
                    group
                },
                |index| explicit.binary_search(&index).unwrap_or(0),
            )
        })
        .collect()
}

fn talk_fragment_count(slide: &TalkSlide) -> usize {
    talk_fragment_groups(&slide.document)
        .into_iter()
        .max()
        .map_or(0, |group| group + 1)
}

fn talk_fragment_counts(slides: &[TalkSlide]) -> Vec<usize> {
    slides.iter().map(talk_fragment_count).collect()
}

fn prepare_talk_node(
    node: &DocumentNode,
    groups: &[usize],
    revealed: usize,
    cursor: &mut usize,
) -> DocumentNode {
    let DocumentNode::Element(element) = node else {
        return node.clone();
    };
    let mut element = element.clone();
    if is_speaker_notes(&element) {
        let class = element.attributes.entry("class".into()).or_default();
        if !class_has_word(class, "faqe-speaker-notes") {
            class.push_str(if class.is_empty() {
                "faqe-speaker-notes"
            } else {
                " faqe-speaker-notes"
            });
        }
        element
            .attributes
            .insert("aria-hidden".into(), "true".into());
        element.attributes.insert("hidden".into(), String::new());
        return DocumentNode::Element(element);
    }
    if class_has_token(&element.attributes, "fragment") {
        let group = groups.get(*cursor).copied().unwrap_or(usize::MAX);
        *cursor += 1;
        let visible = group < revealed;
        let class = element.attributes.entry("class".into()).or_default();
        if visible && !class_has_word(class, "visible") {
            class.push_str(" visible");
        }
        if visible && group + 1 == revealed && !class_has_word(class, "current-fragment") {
            class.push_str(" current-fragment");
        }
        element
            .attributes
            .insert("aria-hidden".into(), (!visible).to_string());
        element
            .attributes
            .insert("data-faqe-fragment-step".into(), (group + 1).to_string());
    }
    element.children = element
        .children
        .iter()
        .map(|child| prepare_talk_node(child, groups, revealed, cursor))
        .collect();
    DocumentNode::Element(element)
}

fn class_has_word(class: &str, word: &str) -> bool {
    class.split_ascii_whitespace().any(|part| part == word)
}

fn talk_document(document: &Document, revealed: usize) -> Document {
    let groups = talk_fragment_groups(document);
    let mut cursor = 0;
    Document {
        nodes: document
            .nodes
            .iter()
            .map(|node| prepare_talk_node(node, &groups, revealed, &mut cursor))
            .collect(),
    }
}

fn collect_speaker_notes(node: &DocumentNode, notes: &mut Vec<DocumentNode>) {
    let DocumentNode::Element(element) = node else {
        return;
    };
    if is_speaker_notes(element) {
        notes.extend(element.children.iter().cloned());
        return;
    }
    for child in &element.children {
        collect_speaker_notes(child, notes);
    }
}

fn speaker_notes(slide: &TalkSlide) -> Document {
    let mut nodes = Vec::new();
    for node in &slide.document.nodes {
        collect_speaker_notes(node, &mut nodes);
    }
    Document { nodes }
}

#[derive(Properties, PartialEq)]
struct TalkPageProps {
    page: Page,
    exit_route: String,
    author: String,
}

#[function_component(TalkPage)]
fn talk_page(props: &TalkPageProps) -> Html {
    let Some(deck) = props.page.talk.as_ref() else {
        return html! { <main id="main" tabindex="-1" class="reveal faqe-error" role="alert"><h1>{&props.page.title}</h1><p>{"Presentation data is missing."}</p></main> };
    };
    let group_lengths = talk_group_lengths(&deck.slides);
    let fragment_counts = talk_fragment_counts(&deck.slides);
    let initial_groups = group_lengths.clone();
    let initial_slides = deck.slides.clone();
    let initial_fragment_counts = fragment_counts.clone();
    let deck_state = use_state(move || {
        let location = web_sys::window()
            .and_then(|window| window.location().hash().ok())
            .and_then(|hash| talk_location_from_hash(&hash, &initial_slides))
            .unwrap_or_default();
        deck_state_at_location(location, &initial_groups, &initial_fragment_counts)
    });
    let viewport = use_state(viewport_size);
    let talk_root = use_node_ref();
    let overview = use_state(|| false);
    let paused = use_state(|| false);
    let help_open = use_state(|| false);
    let presenter_open = use_state(|| false);
    let fullscreen = use_state(|| false);
    let exit_href = site_url(&props.exit_route);
    let exit_click = {
        let exit_href = exit_href.clone();
        Callback::from(move |event: MouseEvent| {
            event.prevent_default();
            exit_talk(&exit_href);
        })
    };

    {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        let slides = deck.slides.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().map(|window| {
                EventListener::new(&window, "hashchange", move |_| {
                    if let Some(location) = web_sys::window()
                        .and_then(|window| window.location().hash().ok())
                        .and_then(|hash| talk_location_from_hash(&hash, &slides))
                    {
                        deck_state.set(deck_state_at_location(
                            location,
                            &group_lengths,
                            &fragment_counts,
                        ));
                    }
                })
            });
            move || drop(listener)
        });
    }

    {
        let current = TalkLocation {
            position: deck_state.position(),
            fragment: deck_state.fragment,
        };
        let slides = deck.slides.clone();
        use_effect_with(current, move |location| {
            write_talk_hash(*location, &slides);
            || ()
        });
    }

    {
        let fullscreen = fullscreen.clone();
        use_effect_with((), move |_| {
            let listener =
                EventListener::new(&gloo_utils::document(), "fullscreenchange", move |_| {
                    fullscreen.set(gloo_utils::document().fullscreen_element().is_some())
                });
            move || drop(listener)
        });
    }

    {
        let viewport = viewport.clone();
        use_effect_with((), move |_| {
            let listeners = web_sys::window().map(|window| {
                let window_viewport = viewport.clone();
                let resize = EventListener::new(&window, "resize", move |_| {
                    window_viewport.set(viewport_size())
                });
                let visual_resize = window.visual_viewport().map(|visual| {
                    let viewport = viewport.clone();
                    EventListener::new(&visual, "resize", move |_| viewport.set(viewport_size()))
                });
                (resize, visual_resize)
            });
            move || drop(listeners)
        });
    }

    {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        let overview = overview.clone();
        let paused = paused.clone();
        let help_open = help_open.clone();
        let presenter_open = presenter_open.clone();
        let talk_root = talk_root.clone();
        let exit_href = exit_href.clone();
        use_effect_with(
            (
                exit_href.clone(),
                (*deck_state).clone(),
                *overview,
                *paused,
                *help_open,
                *presenter_open,
            ),
            move |_| {
                let listener =
                    EventListener::new(&gloo_utils::document(), "keydown", move |event| {
                        let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                            return;
                        };
                        let guarded = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                            .is_some_and(|target| talk_navigation_target_is_guarded(&target));
                        if let Some(action) = talk_action_for_key(
                            &event.key(),
                            event.shift_key(),
                            event.ctrl_key() || event.alt_key() || event.meta_key(),
                            guarded,
                        ) {
                            if !talk_action_allowed(action, *overview, *paused, *help_open) {
                                return;
                            }
                            if event.repeat() && action != TalkAction::Escape {
                                return;
                            }
                            event.prevent_default();
                            match action {
                                TalkAction::Overview => overview.set(!*overview),
                                TalkAction::Pause => paused.set(!*paused),
                                TalkAction::Help => help_open.set(!*help_open),
                                TalkAction::Presenter => presenter_open.set(!*presenter_open),
                                TalkAction::Escape => {
                                    exit_talk(&exit_href);
                                }
                                TalkAction::Fullscreen => toggle_fullscreen(&talk_root),
                            }
                            return;
                        }
                        if *overview || *paused || *help_open {
                            return;
                        }
                        let movement = talk_move_for_key(
                            &event.key(),
                            event.shift_key(),
                            event.ctrl_key() || event.alt_key() || event.meta_key(),
                            guarded,
                        );
                        if let Some(movement) = movement {
                            event.prevent_default();
                            deck_state.set(move_deck_state_with_fragments(
                                &deck_state,
                                &group_lengths,
                                &fragment_counts,
                                movement,
                            ));
                        }
                    });
                move || drop(listener)
            },
        );
    }

    {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        let talk_root = talk_root.clone();
        let navigation_blocked = *overview || *paused || *help_open;
        use_effect_with(
            (
                props.page.route.clone(),
                (*deck_state).clone(),
                navigation_blocked,
            ),
            move |_| {
                let start = Rc::new(Cell::new(None::<(i32, i32)>));
                let listeners = talk_root.cast::<web_sys::Element>().map(|root| {
                    let start_point = start.clone();
                    let touch_start = EventListener::new(&root, "touchstart", move |event| {
                        let guarded = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                            .is_some_and(|target| talk_navigation_target_is_guarded(&target));
                        let touch_event = event.dyn_ref::<web_sys::TouchEvent>();
                        let point = touch_event
                            .filter(|event| {
                                touch_navigation_can_start(
                                    event.touches().length(),
                                    navigation_blocked,
                                    guarded,
                                )
                            })
                            .and_then(|event| event.touches().get(0))
                            .map(|touch| (touch.client_x(), touch.client_y()));
                        start_point.set(point);
                    });
                    let end_point = start.clone();
                    let touch_end = EventListener::new(&root, "touchend", move |event| {
                        let Some((start_x, start_y)) = end_point.take() else {
                            return;
                        };
                        let Some((end_x, end_y)) = event
                            .dyn_ref::<web_sys::TouchEvent>()
                            .and_then(|event| event.changed_touches().get(0))
                            .map(|touch| (touch.client_x(), touch.client_y()))
                        else {
                            return;
                        };
                        if let Some(movement) =
                            talk_move_for_swipe(end_x - start_x, end_y - start_y)
                        {
                            event.prevent_default();
                            deck_state.set(move_deck_state_with_fragments(
                                &deck_state,
                                &group_lengths,
                                &fragment_counts,
                                movement,
                            ));
                        }
                    });
                    let cancel_point = start.clone();
                    let touch_cancel = EventListener::new(&root, "touchcancel", move |_| {
                        cancel_point.set(None);
                    });
                    (touch_start, touch_end, touch_cancel)
                });
                move || drop(listeners)
            },
        );
    }

    let current = normalize_talk_position(deck_state.position(), &group_lengths);
    let move_callback = |movement| {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        Callback::from(move |_| {
            deck_state.set(move_deck_state_with_fragments(
                &deck_state,
                &group_lengths,
                &fragment_counts,
                movement,
            ));
        })
    };
    let (completed, total, progress_percent) =
        talk_progress_with_fragments(&deck_state, &group_lengths, &fragment_counts);
    let progress = format!("width:{progress_percent:.3}%");
    let config = DeckConfig::default();
    let scale = deck_scale(viewport.0, viewport.1, config);
    let slides_style = format!(
        "height:{}px;width:{}px;transform:translate(-50%,-50%) scale({scale:.6})",
        config.height, config.width
    );
    let active_slide = talk_slide_at(&deck.slides, current);
    let background = active_slide.map(slide_background).unwrap_or_default();
    let background_style = slide_background_style(&background);
    let active_fragment_count = active_slide.map(talk_fragment_count).unwrap_or(0);
    let slide_number = talk_slide_index(current, &group_lengths).unwrap_or(0) + 1;
    let slide_total = deck.slides.len();
    let at_last = current.0 + 1 == group_lengths.len()
        && current.1 + 1 == group_lengths.get(current.0).copied().unwrap_or(1)
        && deck_state.fragment == active_fragment_count;
    let select_slide = {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        let overview = overview.clone();
        Callback::from(move |position: (usize, usize)| {
            deck_state.set(deck_state_at_location(
                TalkLocation {
                    position,
                    fragment: 0,
                },
                &group_lengths,
                &fragment_counts,
            ));
            overview.set(false);
        })
    };
    let toggle_overview = {
        let overview = overview.clone();
        let help_open = help_open.clone();
        Callback::from(move |_| {
            help_open.set(false);
            overview.set(!*overview);
        })
    };
    let toggle_pause = {
        let paused = paused.clone();
        let overview = overview.clone();
        let help_open = help_open.clone();
        Callback::from(move |_| {
            overview.set(false);
            help_open.set(false);
            paused.set(!*paused);
        })
    };
    let toggle_help = {
        let help_open = help_open.clone();
        let overview = overview.clone();
        Callback::from(move |_| {
            overview.set(false);
            help_open.set(!*help_open);
        })
    };
    let toggle_presenter = {
        let presenter_open = presenter_open.clone();
        Callback::from(move |_| presenter_open.set(!*presenter_open))
    };
    let toggle_fullscreen_click = {
        let talk_root = talk_root.clone();
        Callback::from(move |_| toggle_fullscreen(&talk_root))
    };
    let seek_progress = {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        Callback::from(move |event: MouseEvent| {
            let Some(width) = event
                .current_target()
                .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                .map(|element| element.client_width())
                .filter(|width| *width > 0)
            else {
                return;
            };
            let ratio = f64::from(event.offset_x()) / f64::from(width);
            let location = talk_location_for_progress(ratio, &group_lengths, &fragment_counts);
            deck_state.set(deck_state_at_location(
                location,
                &group_lengths,
                &fragment_counts,
            ));
        })
    };
    let seek_progress_key = {
        let deck_state = deck_state.clone();
        let group_lengths = group_lengths.clone();
        let fragment_counts = fragment_counts.clone();
        Callback::from(move |event: KeyboardEvent| {
            if let Some(movement) = talk_progress_move_for_key(&event.key()) {
                event.prevent_default();
                deck_state.set(move_deck_state_with_fragments(
                    &deck_state,
                    &group_lengths,
                    &fragment_counts,
                    movement,
                ));
            }
        })
    };

    html! {
        <main
            ref={talk_root}
            id="main"
            tabindex="-1"
            class="reveal bordeline faqe-talk"
            data-faqe-talk-h={current.0.to_string()}
            data-faqe-talk-v={current.1.to_string()}
            data-faqe-talk-f={deck_state.fragment.to_string()}
            data-faqe-talk-scale={format!("{scale:.6}")}
        >
            <h1 class="faqe-visually-hidden">{&props.page.title}</h1>
            <div class="line top"></div><div class="line bottom"></div><div class="line left"></div><div class="line right"></div>
            <div class="faqe-talk-canvas-meta" aria-hidden="true">
                <span>{&props.author}</span>
                <span class="faqe-talk-canvas-count">{format!("{slide_number:02} / {slide_total:02}")}</span>
            </div>
            <div class="faqe-slide-background" style={background_style} aria-hidden="true"></div>
            <div class="slides" style={slides_style} aria-hidden={paused.to_string()}>
                {if deck.slides.is_empty() { html! { <section class="present"><p>{"This presentation has no slides."}</p></section> } } else { html! { {for talk_sections(&deck.slides, current, deck_state.fragment, &props.page.style)} } }}
            </div>
            <nav class="controls" aria-label="Presentation controls" data-controls-layout="bottom-right" data-controls-back-arrows="faded">
                <button class="navigate-previous" aria-label="Previous slide" disabled={completed <= 1} onclick={move_callback(TalkMove::Previous)}>{"<"}</button>
                <button class="navigate-next" aria-label="Next slide" disabled={at_last} onclick={move_callback(TalkMove::Next)}>{">"}</button>
                <a class="faqe-talk-exit" aria-label="Exit presentation" href={site_url(&props.exit_route)} onclick={exit_click}>{"EXIT"}</a>
            </nav>
            <nav class="faqe-talk-utility" aria-label="Presentation utilities">
                <button type="button" aria-label="Toggle slide overview" aria-pressed={overview.to_string()} onclick={toggle_overview}>{"O"}</button>
                <button type="button" aria-label="Pause or resume presentation" aria-pressed={paused.to_string()} onclick={toggle_pause}>{"B"}</button>
                <button type="button" aria-label="Toggle fullscreen" aria-pressed={fullscreen.to_string()} onclick={toggle_fullscreen_click}>{"F"}</button>
                <button type="button" aria-label="Show keyboard help" aria-pressed={help_open.to_string()} onclick={toggle_help.clone()}>{"?"}</button>
                <button type="button" aria-label="Toggle speaker view" aria-pressed={presenter_open.to_string()} onclick={toggle_presenter.clone()}>{"S"}</button>
            </nav>
            {if *overview { talk_overview(&group_lengths, current, select_slide) } else { Html::default() }}
            {if *help_open { talk_help(toggle_help) } else { Html::default() }}
            {if *presenter_open { talk_presenter(&deck.slides, current, deck_state.fragment, toggle_presenter) } else { Html::default() }}
            {if *paused { html! { <div class="faqe-talk-paused" role="status" aria-live="polite" aria-label="Presentation paused"></div> } } else { Html::default() }}
            <button type="button" class="progress" role="slider" aria-label="Seek presentation" aria-orientation="horizontal" aria-valuemin="1" aria-valuemax={total.to_string()} aria-valuenow={completed.to_string()} onclick={seek_progress} onkeydown={seek_progress_key}><span style={progress}></span></button>
            <div class="faqe-slide-status" role="status" aria-live="polite" aria-atomic="true">{talk_status_text(current, &group_lengths, deck_state.fragment, active_fragment_count)}</div>
        </main>
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TalkMove {
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
    First,
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TalkAction {
    Overview,
    Pause,
    Fullscreen,
    Help,
    Presenter,
    Escape,
}

fn talk_action_for_key(
    key: &str,
    _shift: bool,
    has_modifier: bool,
    guarded_target: bool,
) -> Option<TalkAction> {
    if key == "Escape" {
        return Some(TalkAction::Escape);
    }
    if has_modifier || guarded_target {
        return None;
    }
    match key {
        "o" | "O" => Some(TalkAction::Overview),
        "b" | "B" | "." => Some(TalkAction::Pause),
        "f" | "F" => Some(TalkAction::Fullscreen),
        "?" => Some(TalkAction::Help),
        "s" | "S" => Some(TalkAction::Presenter),
        _ => None,
    }
}

fn talk_action_allowed(action: TalkAction, overview: bool, paused: bool, help_open: bool) -> bool {
    if action == TalkAction::Escape {
        return true;
    }
    if help_open {
        return action == TalkAction::Help;
    }
    if overview {
        return action == TalkAction::Overview;
    }
    if paused {
        return action == TalkAction::Pause;
    }
    true
}

fn decode_slide_hash_id(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digits = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(digits, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn explicit_slide_id(slide: &TalkSlide) -> Option<&str> {
    slide
        .attributes
        .get("id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|id| {
            !id.is_empty()
                && id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
}

fn talk_position_for_slide_id(id: &str, slides: &[TalkSlide]) -> Option<(usize, usize)> {
    let groups = talk_group_lengths(slides);
    let mut matches = slides
        .iter()
        .enumerate()
        .filter(|(_, slide)| explicit_slide_id(slide) == Some(id));
    let (index, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let mut remaining = index;
    for (horizontal, count) in groups.iter().copied().enumerate() {
        if remaining < count {
            return Some((horizontal, remaining));
        }
        remaining -= count;
    }
    None
}

fn talk_hash(position: (usize, usize)) -> String {
    format!("#/{}/{}", position.0, position.1)
}

fn talk_hash_for_slide(position: (usize, usize), slides: &[TalkSlide]) -> String {
    talk_slide_at(slides, position)
        .and_then(explicit_slide_id)
        .filter(|id| talk_position_for_slide_id(id, slides) == Some(position))
        .map(|id| format!("#/{id}"))
        .unwrap_or_else(|| talk_hash(position))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TalkLocation {
    position: (usize, usize),
    fragment: usize,
}

fn talk_location_from_hash(hash: &str, slides: &[TalkSlide]) -> Option<TalkLocation> {
    let value = hash.trim().trim_start_matches('#').trim_start_matches('/');
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.is_empty() || parts[0].is_empty() || parts.len() > 3 {
        return None;
    }
    if let Ok(horizontal) = parts[0].parse::<usize>() {
        let vertical = parts.get(1).map_or(Some(0), |value| value.parse().ok())?;
        let fragment = parts.get(2).map_or(Some(0), |value| value.parse().ok())?;
        return Some(TalkLocation {
            position: (horizontal, vertical),
            fragment,
        });
    }
    let id = decode_slide_hash_id(parts[0])?;
    let position = talk_position_for_slide_id(&id, slides)?;
    let fragment = parts.get(1).map_or(Some(0), |value| value.parse().ok())?;
    (parts.len() <= 2).then_some(TalkLocation { position, fragment })
}

fn talk_hash_for_location(location: TalkLocation, slides: &[TalkSlide]) -> String {
    let base = talk_hash_for_slide(location.position, slides);
    if location.fragment == 0 {
        base
    } else {
        format!("{base}/{}", location.fragment)
    }
}

fn write_talk_hash(position: TalkLocation, slides: &[TalkSlide]) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let next = talk_hash_for_location(position, slides);
    if location.hash().ok().as_deref() != Some(next.as_str()) {
        if window.history().is_ok_and(|history| {
            history
                .replace_state_with_url(&JsValue::NULL, "", Some(&next))
                .is_ok()
        }) {
            return;
        }
        let _ = location.replace(&next);
    }
}

fn deck_state_at(position: (usize, usize), groups: &[usize]) -> DeckState {
    let mut state = DeckState::new(groups);
    let position = normalize_talk_position(position, groups);
    state.horizontal = position.0;
    state.vertical = position.1;
    if let Some(remembered) = state.remembered_vertical.get_mut(position.0) {
        *remembered = position.1;
    }
    state
}

fn deck_state_at_location(
    location: TalkLocation,
    groups: &[usize],
    fragment_counts: &[usize],
) -> DeckState {
    let mut state = deck_state_at(location.position, groups);
    let index = talk_slide_index(state.position(), groups).unwrap_or(0);
    state.fragment = location
        .fragment
        .min(fragment_counts.get(index).copied().unwrap_or(0));
    state
}

fn toggle_fullscreen(root: &NodeRef) {
    let document = gloo_utils::document();
    if document.fullscreen_element().is_some() {
        document.exit_fullscreen();
    } else if let Some(element) = root.cast::<web_sys::Element>() {
        let _ = element.request_fullscreen();
    }
}

fn talk_overview(
    groups: &[usize],
    current: (usize, usize),
    select: Callback<(usize, usize)>,
) -> Html {
    html! {
        <section class="faqe-talk-overview" role="dialog" aria-modal="true" aria-labelledby="faqe-overview-title">
            <h2 id="faqe-overview-title">{"Slide overview"}</h2>
            <ol class="faqe-talk-overview-grid">
                {for groups.iter().enumerate().map(|(horizontal, count)| html! {
                    <li class="faqe-talk-overview-stack">
                        {for (0..*count).map(|vertical| {
                            let select = select.clone();
                            html! {
                                <button
                                    type="button"
                                    aria-current={(current == (horizontal, vertical)).then_some("step")}
                                    onclick={Callback::from(move |_| select.emit((horizontal, vertical)))}
                                >{format!("Slide {}.{}", horizontal + 1, vertical + 1)}</button>
                            }
                        })}
                    </li>
                })}
            </ol>
            <p>{"Select a slide, press O to close the overview, or press Escape to exit."}</p>
        </section>
    }
}

fn talk_help(close: Callback<MouseEvent>) -> Html {
    html! {
        <section class="faqe-talk-help" role="dialog" aria-modal="true" aria-labelledby="faqe-help-title">
            <h2 id="faqe-help-title">{"Presentation keyboard help"}</h2>
            <dl>
                <dt>{"Next / previous"}</dt><dd>{"N, P, Space, Shift+Space, PageDown, PageUp"}</dd>
                <dt>{"Directions"}</dt><dd>{"Arrow keys or H, J, K, L"}</dd>
                <dt>{"First / last"}</dt><dd>{"Home, End"}</dd>
                <dt>{"Overview"}</dt><dd>{"O"}</dd>
                <dt>{"Pause / blackout"}</dt><dd>{"B or ."}</dd>
                <dt>{"Fullscreen"}</dt><dd>{"F"}</dd>
                <dt>{"Speaker view"}</dt><dd>{"S"}</dd>
                <dt>{"Exit presentation"}</dt><dd>{"Escape"}</dd>
            </dl>
            <button type="button" onclick={close}>{"Close keyboard help"}</button>
        </section>
    }
}

fn document_text_node(node: &DocumentNode, text: &mut String) {
    match node {
        DocumentNode::Text { value } => text.push_str(value),
        DocumentNode::Element(element) if is_speaker_notes(element) => {}
        DocumentNode::Element(element) => {
            for child in &element.children {
                document_text_node(child, text);
            }
            if matches!(element.tag.as_str(), "p" | "li" | "h1" | "h2" | "h3" | "h4") {
                text.push(' ');
            }
        }
    }
}

fn slide_preview(slide: Option<&TalkSlide>) -> String {
    let Some(slide) = slide else {
        return "End of presentation".into();
    };
    let mut text = String::new();
    for node in &slide.document.nodes {
        document_text_node(node, &mut text);
    }
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = collapsed.chars();
    let preview = characters.by_ref().take(180).collect::<String>();
    if characters.next().is_some() {
        format!("{preview}…")
    } else if preview.is_empty() {
        "Untitled slide".into()
    } else {
        preview
    }
}

fn next_talk_slide(slides: &[TalkSlide], current: (usize, usize)) -> Option<&TalkSlide> {
    let groups = talk_group_lengths(slides);
    let index = talk_slide_index(current, &groups)?;
    slides.get(index + 1)
}

fn talk_status_text(
    current: (usize, usize),
    groups: &[usize],
    fragment: usize,
    fragment_count: usize,
) -> String {
    let (slide, slides, _) = talk_progress(current, groups);
    if fragment_count == 0 {
        format!("Slide {slide} of {slides}")
    } else {
        format!(
            "Slide {slide} of {slides}, fragment {} of {fragment_count}",
            fragment.min(fragment_count)
        )
    }
}

fn talk_presenter(
    slides: &[TalkSlide],
    current: (usize, usize),
    fragment: usize,
    close: Callback<MouseEvent>,
) -> Html {
    let notes = talk_slide_at(slides, current)
        .map(speaker_notes)
        .unwrap_or_default();
    let fragment_count = talk_slide_at(slides, current)
        .map(talk_fragment_count)
        .unwrap_or(0);
    html! {
        <section
            class="faqe-talk-presenter"
            role="dialog"
            aria-modal="false"
            aria-labelledby="faqe-presenter-title"
        >
            <h2 id="faqe-presenter-title">{"Speaker view"}</h2>
            <p aria-live="polite">{format!(
                "Current slide {}.{}, fragment {} of {}",
                current.0 + 1,
                current.1 + 1,
                fragment.min(fragment_count),
                fragment_count,
            )}</p>
            <section class="faqe-talk-presenter-notes" aria-labelledby="faqe-presenter-notes-title">
                <h3 id="faqe-presenter-notes-title">{"Speaker notes"}</h3>
                {if notes.nodes.is_empty() {
                    html! { <p>{"No speaker notes for this slide."}</p> }
                } else {
                    document_view(&notes)
                }}
            </section>
            <section aria-labelledby="faqe-presenter-next-title">
                <h3 id="faqe-presenter-next-title">{"Next slide"}</h3>
                <p>{slide_preview(next_talk_slide(slides, current))}</p>
            </section>
            <p>{"Presentation navigation remains active while speaker view is open."}</p>
            <button type="button" onclick={close}>{"Close speaker view"}</button>
        </section>
    }
}

fn talk_move_for_key(
    key: &str,
    shift: bool,
    has_modifier: bool,
    guarded_target: bool,
) -> Option<TalkMove> {
    if has_modifier || guarded_target {
        return None;
    }
    match key {
        "ArrowLeft" | "h" | "H" => Some(TalkMove::Left),
        "ArrowRight" | "l" | "L" => Some(TalkMove::Right),
        "ArrowUp" | "k" | "K" => Some(TalkMove::Up),
        "ArrowDown" | "j" | "J" => Some(TalkMove::Down),
        "PageUp" | "p" | "P" => Some(TalkMove::Previous),
        "PageDown" | "n" | "N" => Some(TalkMove::Next),
        " " if shift => Some(TalkMove::Previous),
        " " => Some(TalkMove::Next),
        "Home" => Some(TalkMove::First),
        "End" => Some(TalkMove::Last),
        _ => None,
    }
}

fn talk_navigation_target_is_guarded(target: &web_sys::Element) -> bool {
    target
        .closest(
            "a[href], button, input, select, textarea, summary, pre, code, video, \
             [contenteditable]:not([contenteditable='false']), [role='button'], \
             [role='link'], [role='textbox']",
        )
        .ok()
        .flatten()
        .is_some()
}

const TALK_SWIPE_THRESHOLD: i32 = 50;

fn talk_move_for_swipe(delta_x: i32, delta_y: i32) -> Option<TalkMove> {
    if delta_x.abs().max(delta_y.abs()) < TALK_SWIPE_THRESHOLD {
        return None;
    }
    if delta_x.abs() >= delta_y.abs() {
        Some(if delta_x < 0 {
            TalkMove::Right
        } else {
            TalkMove::Left
        })
    } else {
        Some(if delta_y < 0 {
            TalkMove::Down
        } else {
            TalkMove::Up
        })
    }
}

fn touch_navigation_can_start(touch_count: u32, blocked: bool, guarded: bool) -> bool {
    touch_count == 1 && !blocked && !guarded
}

fn talk_group_lengths(slides: &[TalkSlide]) -> Vec<usize> {
    let mut groups = Vec::new();
    let mut index = 0;
    while index < slides.len() {
        if let Some(group) = slides[index].vertical_group {
            let start = index;
            while index < slides.len() && slides[index].vertical_group == Some(group) {
                index += 1;
            }
            groups.push(index - start);
        } else {
            groups.push(1);
            index += 1;
        }
    }
    groups
}

fn normalize_talk_position(position: (usize, usize), groups: &[usize]) -> (usize, usize) {
    if groups.is_empty() {
        return (0, 0);
    }
    let horizontal = position.0.min(groups.len() - 1);
    let vertical = position.1.min(groups[horizontal].saturating_sub(1));
    (horizontal, vertical)
}

fn move_deck_state(state: &DeckState, groups: &[usize], movement: TalkMove) -> DeckState {
    if groups.is_empty() {
        return DeckState::new(groups);
    }
    let (horizontal, vertical) = normalize_talk_position(state.position(), groups);
    let mut next = state.clone();
    next.remembered_vertical.resize(groups.len(), 0);
    next.remembered_vertical[horizontal] = vertical;

    let position = match movement {
        TalkMove::Left => {
            let target = horizontal.saturating_sub(1);
            (
                target,
                next.remembered_vertical[target].min(groups[target] - 1),
            )
        }
        TalkMove::Right => {
            let target = (horizontal + 1).min(groups.len() - 1);
            (
                target,
                next.remembered_vertical[target].min(groups[target] - 1),
            )
        }
        TalkMove::Up => (horizontal, vertical.saturating_sub(1)),
        TalkMove::Down => (horizontal, (vertical + 1).min(groups[horizontal] - 1)),
        TalkMove::Next if vertical + 1 < groups[horizontal] => (horizontal, vertical + 1),
        TalkMove::Next => {
            let target = (horizontal + 1).min(groups.len() - 1);
            (
                target,
                next.remembered_vertical[target].min(groups[target] - 1),
            )
        }
        TalkMove::Previous if vertical > 0 => (horizontal, vertical - 1),
        TalkMove::Previous if horizontal > 0 => {
            let target = horizontal - 1;
            (target, groups[target] - 1)
        }
        TalkMove::Previous | TalkMove::First => (0, 0),
        TalkMove::Last => {
            let target = groups.len() - 1;
            (target, groups[target] - 1)
        }
    };
    next.horizontal = position.0;
    next.vertical = position.1;
    next.remembered_vertical[position.0] = position.1;
    next
}

fn talk_slide_index(position: (usize, usize), groups: &[usize]) -> Option<usize> {
    if groups.is_empty() {
        return None;
    }
    let position = normalize_talk_position(position, groups);
    Some(groups.iter().take(position.0).sum::<usize>() + position.1)
}

fn fragment_count_at(
    position: (usize, usize),
    groups: &[usize],
    fragment_counts: &[usize],
) -> usize {
    talk_slide_index(position, groups)
        .and_then(|index| fragment_counts.get(index))
        .copied()
        .unwrap_or(0)
}

fn move_deck_state_with_fragments(
    state: &DeckState,
    groups: &[usize],
    fragment_counts: &[usize],
    movement: TalkMove,
) -> DeckState {
    if groups.is_empty() {
        return DeckState::new(groups);
    }
    let mut current = state.clone();
    current.fragment = current.fragment.min(fragment_count_at(
        current.position(),
        groups,
        fragment_counts,
    ));
    if matches!(movement, TalkMove::Next | TalkMove::Right)
        && current.fragment < fragment_count_at(current.position(), groups, fragment_counts)
    {
        current.fragment += 1;
        return current;
    }
    if matches!(movement, TalkMove::Previous | TalkMove::Left) && current.fragment > 0 {
        current.fragment -= 1;
        return current;
    }

    let mut next = move_deck_state(&current, groups, movement);
    next.fragment = if matches!(
        movement,
        TalkMove::Last | TalkMove::Previous | TalkMove::Left | TalkMove::Up
    ) {
        fragment_count_at(next.position(), groups, fragment_counts)
    } else {
        0
    };
    next
}

fn talk_progress(position: (usize, usize), groups: &[usize]) -> (usize, usize, f64) {
    if groups.is_empty() {
        return (1, 1, 0.0);
    }
    let current = normalize_talk_position(position, groups);
    let completed = groups.iter().take(current.0).sum::<usize>() + current.1 + 1;
    let total = groups.iter().sum::<usize>();
    let percent = if total <= 1 {
        0.0
    } else {
        (completed - 1) as f64 * 100.0 / (total - 1) as f64
    };
    (completed, total, percent)
}

fn talk_progress_with_fragments(
    state: &DeckState,
    groups: &[usize],
    fragment_counts: &[usize],
) -> (usize, usize, f64) {
    let Some(index) = talk_slide_index(state.position(), groups) else {
        return (1, 1, 0.0);
    };
    let total = groups.iter().sum::<usize>().max(fragment_counts.len());
    let total = (0..total)
        .map(|slide| fragment_counts.get(slide).copied().unwrap_or(0) + 1)
        .sum::<usize>();
    let completed = (0..index)
        .map(|slide| fragment_counts.get(slide).copied().unwrap_or(0) + 1)
        .sum::<usize>()
        + state
            .fragment
            .min(fragment_counts.get(index).copied().unwrap_or(0))
        + 1;
    let percent = if total <= 1 {
        0.0
    } else {
        (completed - 1) as f64 * 100.0 / (total - 1) as f64
    };
    (completed, total.max(1), percent)
}

fn talk_location_for_progress(
    ratio: f64,
    groups: &[usize],
    fragment_counts: &[usize],
) -> TalkLocation {
    let slide_total = groups.iter().sum::<usize>();
    if slide_total == 0 {
        return TalkLocation::default();
    }
    let weights = (0..slide_total)
        .map(|slide| fragment_counts.get(slide).copied().unwrap_or(0) + 1)
        .collect::<Vec<_>>();
    let total = weights.iter().sum::<usize>();
    let mut step = if total <= 1 {
        0
    } else {
        (ratio.clamp(0.0, 1.0) * (total - 1) as f64).round() as usize
    };
    let mut slide_index = 0;
    for (index, weight) in weights.iter().copied().enumerate() {
        if step < weight {
            slide_index = index;
            break;
        }
        step -= weight;
        slide_index = index;
    }
    let mut remaining = slide_index;
    for (horizontal, count) in groups.iter().copied().enumerate() {
        if remaining < count {
            return TalkLocation {
                position: (horizontal, remaining),
                fragment: step.min(fragment_counts.get(slide_index).copied().unwrap_or(0)),
            };
        }
        remaining -= count;
    }
    TalkLocation::default()
}

fn talk_progress_move_for_key(key: &str) -> Option<TalkMove> {
    match key {
        "ArrowLeft" | "ArrowDown" | "PageUp" => Some(TalkMove::Previous),
        "ArrowRight" | "ArrowUp" | "PageDown" => Some(TalkMove::Next),
        "Home" => Some(TalkMove::First),
        "End" => Some(TalkMove::Last),
        _ => None,
    }
}

fn talk_slide_at(slides: &[TalkSlide], position: (usize, usize)) -> Option<&TalkSlide> {
    let groups = talk_group_lengths(slides);
    if groups.is_empty() {
        return None;
    }
    let current = normalize_talk_position(position, &groups);
    let index = groups.iter().take(current.0).sum::<usize>() + current.1;
    slides.get(index)
}

fn slide_background(slide: &TalkSlide) -> SlideBackground {
    SlideBackground {
        color: slide.attributes.get("background-color").cloned(),
        image: slide
            .attributes
            .get("background-image")
            .map(|image| site_url(image.trim().trim_matches(['\'', '"']))),
    }
}

fn css_url_value(value: &str) -> String {
    value
        .replace('\\', "%5C")
        .replace('\'', "%27")
        .replace('"', "%22")
        .replace('(', "%28")
        .replace(')', "%29")
}

fn slide_background_style(background: &SlideBackground) -> String {
    let mut style = String::new();
    if let Some(color) = &background.color {
        style.push_str(&format!("background-color:{color};"));
    }
    if let Some(image) = &background.image {
        style.push_str(&format!(
            "background-image:url('{}');",
            css_url_value(image)
        ));
    }
    style
}

fn slide_palette_style(background: &SlideBackground, page_style: &PageStyle) -> String {
    let surface = background
        .color
        .as_deref()
        .unwrap_or(&page_style.background);
    let foreground = if background.color.is_some() {
        contrasting_text(surface).unwrap_or_else(|| page_style.foreground.clone())
    } else {
        page_style.foreground.clone()
    };
    let accent =
        accessible_color(&page_style.accent, surface).unwrap_or_else(|| page_style.accent.clone());
    let accent_text = contrasting_text(&accent).unwrap_or_else(|| page_style.background.clone());
    format!(
        "--faqe-slide-fg:{foreground};--faqe-slide-accent:{accent};--faqe-slide-accent-text:{accent_text};"
    )
}

fn talk_sections(
    slides: &[TalkSlide],
    current: (usize, usize),
    active_fragment: usize,
    page_style: &PageStyle,
) -> Vec<Html> {
    let mut sections = Vec::new();
    let mut index = 0;
    let mut horizontal = 0;
    while index < slides.len() {
        let state = talk_state(horizontal, current.0);
        if let Some(group) = slides[index].vertical_group {
            let start = index;
            while index < slides.len() && slides[index].vertical_group == Some(group) {
                index += 1;
            }
            sections.push(
                html! { <section class={state} aria-hidden={(state != "present").to_string()}>{for slides[start..index].iter().enumerate().map(|(vertical, slide)| talk_slide(slide, talk_nested_state(state, vertical, current.1), (horizontal, vertical), slides, active_fragment, page_style))}</section> },
            );
        } else {
            sections.push(talk_slide(
                &slides[index],
                state,
                (horizontal, 0),
                slides,
                active_fragment,
                page_style,
            ));
            index += 1;
        }
        horizontal += 1;
    }
    sections
}

fn talk_state(index: usize, current: usize) -> &'static str {
    match index.cmp(&current) {
        std::cmp::Ordering::Less => "past",
        std::cmp::Ordering::Equal => "present",
        std::cmp::Ordering::Greater => "future",
    }
}

fn talk_nested_state(
    horizontal_state: &'static str,
    vertical: usize,
    current_vertical: usize,
) -> &'static str {
    if horizontal_state == "present" {
        talk_state(vertical, current_vertical)
    } else {
        horizontal_state
    }
}

fn talk_slide(
    slide: &TalkSlide,
    state: &str,
    position: (usize, usize),
    slides: &[TalkSlide],
    active_fragment: usize,
    page_style: &PageStyle,
) -> Html {
    let mut class = slide.attributes.get("class").cloned().unwrap_or_default();
    if !class.is_empty() {
        class.push(' ');
    }
    class.push_str(state);
    class.push(' ');
    class.push_str(SlideTransition::parse(slide.attributes.get("transition")).class());
    class.push(' ');
    class.push_str(TransitionSpeed::parse(slide.attributes.get("transition-speed")).class());
    let background = slide_background(slide);
    let mut print_style = slide_palette_style(&background, page_style);
    if let Some(color) = background.color.as_deref() {
        let _ = write!(print_style, "--faqe-print-background-color:{color};");
    }
    if let Some(image) = background.image.as_deref() {
        let _ = write!(
            print_style,
            "--faqe-print-background-image:url(\"{}\");",
            css_url_value(image)
        );
    }
    let id = explicit_slide_id(slide)
        .filter(|id| talk_position_for_slide_id(id, slides) == Some(position))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("slide-{}-{}", position.0, position.1));
    let transition = TransitionSpeed::parse(slide.attributes.get("transition-speed"));
    let revealed = match state {
        "past" => talk_fragment_count(slide),
        "present" => active_fragment.min(talk_fragment_count(slide)),
        _ => 0,
    };
    let document = talk_document(&slide.document, revealed);
    html! {
        <section
            id={id}
            class={class}
            role="group"
            aria-roledescription="slide"
            aria-hidden={(state != "present").to_string()}
            data-transition={slide.attributes.get("transition").cloned().unwrap_or_default()}
            data-transition-speed={slide.attributes.get("transition-speed").cloned().unwrap_or_default()}
            data-transition-duration-ms={transition.duration_ms().to_string()}
            data-faqe-fragment={revealed.to_string()}
            data-background-color={background.color.unwrap_or_default()}
            data-background-image={background.image.unwrap_or_default()}
            style={print_style}
        >{document_view(&document)}</section>
    }
}

fn document_view(document: &Document) -> Html {
    html! { <>{for document.nodes.iter().map(render_document_node)}</> }
}

fn article_document_view(document: &Document) -> Html {
    html! { <>{for document.nodes.iter().map(render_article_document_node)}</> }
}

fn inline_document_view(document: &Document) -> Html {
    let meaningful = document
        .nodes
        .iter()
        .filter(|node| !matches!(node, DocumentNode::Text { value } if value.trim().is_empty()))
        .collect::<Vec<_>>();
    if meaningful.len() == 1 {
        if let DocumentNode::Element(element) = meaningful[0] {
            if element.tag == "p" {
                return html! { <>{for element.children.iter().map(render_document_node)}</> };
            }
        }
    }
    document_view(document)
}

fn logo_document_view(document: &Document, key_route: &str) -> Html {
    html! { <>{for document.nodes.iter().map(|node| render_logo_node(node, key_route))}</> }
}

fn render_logo_node(node: &DocumentNode, key_route: &str) -> Html {
    match node {
        DocumentNode::Text { value } => Html::from(value.clone()),
        DocumentNode::Element(element) => {
            let is_key = element
                .attributes
                .get("class")
                .is_some_and(|class| class.split_whitespace().any(|name| name == "svgmiddle"));
            let mut tag = VTag::new(if is_key {
                "a".to_owned()
            } else {
                element.tag.clone()
            });
            for (name, value) in &element.attributes {
                let value = if matches!(name.as_str(), "href" | "src") && value.starts_with('/') {
                    site_url(value)
                } else {
                    value.clone()
                };
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from(name.clone()),
                    (AttrValue::from(value), ApplyAttributeAs::Attribute),
                );
            }
            if is_key {
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from("href"),
                    (
                        AttrValue::from(site_url(key_route)),
                        ApplyAttributeAs::Attribute,
                    ),
                );
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from("aria-label"),
                    (
                        AttrValue::from("Open public PGP key"),
                        ApplyAttributeAs::Attribute,
                    ),
                );
            }
            for child in &element.children {
                tag.add_child(render_logo_node(child, key_route));
            }
            tag.into()
        }
    }
}

fn render_document_node(node: &DocumentNode) -> Html {
    render_document_node_with_article_headings(node, false, None)
}

fn render_article_document_node(node: &DocumentNode) -> Html {
    render_document_node_with_article_headings(node, true, None)
}

fn render_document_node_with_article_headings(
    node: &DocumentNode,
    article_headings: bool,
    text_profile: Option<GlitchProfile>,
) -> Html {
    match node {
        DocumentNode::Text { value } => {
            if value.trim().is_empty() {
                Html::from(value.clone())
            } else if let Some(profile) = text_profile {
                html! { <ScrambleTitle text={value.clone()} {profile} /> }
            } else {
                Html::from(value.clone())
            }
        }
        DocumentNode::Element(element) => {
            let shifted_heading = article_headings
                .then(|| shifted_article_heading(&element.tag))
                .flatten();
            let heading_profile = if shifted_heading.is_some() {
                Some(GlitchProfile::Subtitle)
            } else {
                match element.tag.as_str() {
                    "h1" => Some(GlitchProfile::Title),
                    "h2" | "h3" | "h4" | "h5" | "h6" => Some(GlitchProfile::Subtitle),
                    _ => text_profile,
                }
            };
            let mut tag = VTag::new(
                shifted_heading
                    .map(|(tag, _)| tag.to_owned())
                    .unwrap_or_else(|| element.tag.clone()),
            );
            for (name, value) in &element.attributes {
                let value = if matches!(name.as_str(), "href" | "src") && value.starts_with('/') {
                    site_url(value)
                } else if name == "class" {
                    shifted_heading
                        .map(|(_, level)| article_heading_class(value, level))
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                };
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from(name.clone()),
                    (AttrValue::from(value), ApplyAttributeAs::Attribute),
                );
            }
            if element.tag == "input" && element.attributes.contains_key("checked") {
                tag.set_checked(true);
            }
            if element.tag == "img" && image_alt_is_decorative(element.attributes.get("alt")) {
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from("alt"),
                    (AttrValue::from(""), ApplyAttributeAs::Attribute),
                );
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from("aria-hidden"),
                    (AttrValue::from("true"), ApplyAttributeAs::Attribute),
                );
            }
            if let Some((_, level)) = shifted_heading {
                if !element.attributes.contains_key("class") {
                    tag.attributes.get_mut_index_map().insert(
                        AttrValue::from("class"),
                        (
                            AttrValue::from(article_heading_class("", level)),
                            ApplyAttributeAs::Attribute,
                        ),
                    );
                }
                tag.attributes.get_mut_index_map().insert(
                    AttrValue::from("data-faqe-source-heading-level"),
                    (
                        AttrValue::from(level.to_string()),
                        ApplyAttributeAs::Attribute,
                    ),
                );
            }
            for child in &element.children {
                tag.add_child(render_document_node_with_article_headings(
                    child,
                    article_headings,
                    heading_profile,
                ));
            }
            tag.into()
        }
    }
}

fn shifted_article_heading(tag: &str) -> Option<(&'static str, u8)> {
    match tag {
        "h1" => Some(("h2", 1)),
        "h2" => Some(("h3", 2)),
        "h3" => Some(("h4", 3)),
        "h4" => Some(("h5", 4)),
        "h5" => Some(("h6", 5)),
        "h6" => Some(("h6", 6)),
        _ => None,
    }
}

fn article_heading_class(existing: &str, level: u8) -> String {
    format!(
        "{}faqe-heading faqe-heading-level-{level}",
        if existing.is_empty() {
            String::new()
        } else {
            format!("{existing} ")
        }
    )
}

fn image_alt_is_decorative(alt: Option<&String>) -> bool {
    alt.is_none_or(|alt| alt.trim().is_empty())
}

#[function_component(NotFound)]
fn not_found() -> Html {
    html! {
        <section class="container centered"><div class="fourofour">
            <h1><ScrambleTitle text={"404".to_owned()} /></h1><h2><ScrambleTitle text={"Page Not Found".to_owned()} profile={GlitchProfile::Subtitle} /></h2>
            <p>{"Sorry, this page does not exist."}<br />{"You can head back to "}<a href={site_url("/")}>{"homepage"}</a>{"."}</p>
        </div></section>
    }
}
