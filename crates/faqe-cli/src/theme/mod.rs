//! Build-time registry for complete visual themes.

use std::collections::BTreeMap;

mod bresilla;

#[derive(Clone, Copy, Debug)]
pub struct Asset {
    pub id: &'static str,
    pub stem: &'static str,
    pub extension: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Clone, Copy, Debug)]
pub struct FontFace {
    pub asset_id: &'static str,
    pub family: &'static str,
    pub style: &'static str,
    pub weight: &'static str,
    pub display: &'static str,
    pub unicode_range: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub struct StyleSheets {
    pub base: &'static str,
    pub resume: &'static str,
    pub talk: &'static str,
    pub motion: fn() -> String,
}

pub struct Definition {
    pub id: &'static str,
    pub assets: &'static [Asset],
    pub fonts: &'static [FontFace],
    pub styles: StyleSheets,
}

pub fn resolve(id: &str) -> Option<&'static Definition> {
    match id {
        bresilla::ID => Some(&bresilla::DEFINITION),
        _ => None,
    }
}

pub fn available() -> impl Iterator<Item = &'static str> {
    [bresilla::ID].into_iter()
}

pub fn render_stylesheet(
    source: &str,
    assets: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut output = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(start) = remaining.find("{{asset:") {
        output.push_str(&remaining[..start]);
        let placeholder = &remaining[start + 8..];
        let Some(end) = placeholder.find("}}") else {
            return Err("theme stylesheet has an unterminated asset placeholder".into());
        };
        let id = &placeholder[..end];
        if id.is_empty() {
            return Err("theme stylesheet has an empty asset placeholder".into());
        }
        let path = assets
            .get(id)
            .ok_or_else(|| format!("theme stylesheet references unknown asset {id:?}"))?;
        output.push_str("./");
        output.push_str(path);
        remaining = &placeholder[end + 2..];
    }
    output.push_str(remaining);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn registered_themes_have_valid_unique_assets() {
        assert!(resolve(faqe_model::DEFAULT_THEME_ID).is_some());
        for id in available() {
            let theme = resolve(id).expect("registered theme resolves");
            assert_eq!(theme.id, id);
            assert!(!theme.styles.base.trim().is_empty());
            assert!(!theme.styles.resume.trim().is_empty());
            assert!(!theme.styles.talk.trim().is_empty());

            let mut asset_ids = BTreeSet::new();
            for asset in theme.assets {
                assert!(asset_ids.insert(asset.id));
                assert!(!asset.bytes.is_empty());
            }
            for font in theme.fonts {
                assert!(asset_ids.contains(font.asset_id));
            }
        }
    }

    #[test]
    fn stylesheet_assets_are_resolved() {
        let assets = BTreeMap::from([("grid".into(), "grid-a1.svg".into())]);
        assert_eq!(
            render_stylesheet("body{background:url('{{asset:grid}}')}", &assets).unwrap(),
            "body{background:url('./grid-a1.svg')}"
        );
        assert!(render_stylesheet("{{asset:missing}}", &assets).is_err());
    }
}
