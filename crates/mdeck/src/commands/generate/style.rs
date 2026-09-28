//! Which image and icon style a deck's generated pictures use.

use crate::config::Config;
use crate::parser::PresentationMeta;

use super::{DEFAULT_ICON_STYLE, DEFAULT_IMAGE_STYLE};

/// The resolved style descriptions for images and diagram icons.
pub(super) struct Styles {
    pub image: String,
    pub icon: String,
}

/// `--style` > the deck's `@image-style` / `@icon-style` > config default > hardcoded.
pub(super) fn resolve_styles(
    config: &Config,
    meta: &PresentationMeta,
    style_override: Option<&str>,
) -> Styles {
    let defaults = config.defaults.as_ref();
    let image = resolve_style(
        style_override,
        meta.image_style.as_deref(),
        defaults.and_then(|d| d.image_style.as_deref()),
        |name| config.get_style(name),
        DEFAULT_IMAGE_STYLE,
    );

    // `--style` names an *image* style; only apply it to icons when it is
    // also (or only) an icon style name, or a literal description.
    let icon_override = icon_style_override(
        style_override,
        |n| config.get_style(n).is_some(),
        |n| config.get_icon_style(n).is_some(),
    );
    let icon = resolve_style(
        icon_override.as_deref(),
        meta.icon_style.as_deref(),
        defaults.and_then(|d| d.icon_style.as_deref()),
        |name| config.get_icon_style(name),
        DEFAULT_ICON_STYLE,
    );
    Styles { image, icon }
}

fn resolve_style<'a>(
    cli_override: Option<&str>,
    frontmatter: Option<&str>,
    config_default_name: Option<&str>,
    lookup: impl Fn(&str) -> Option<&'a str>,
    hardcoded: &str,
) -> String {
    // --style CLI flag
    if let Some(s) = cli_override {
        // Try as a named style first, otherwise use as literal
        if let Some(desc) = lookup(s) {
            return desc.to_string();
        }
        return s.to_string();
    }
    // @image-style frontmatter
    if let Some(s) = frontmatter {
        if let Some(desc) = lookup(s) {
            return desc.to_string();
        }
        return s.to_string();
    }
    // defaults.image_style config
    if let Some(name) = config_default_name
        && let Some(desc) = lookup(name)
    {
        return desc.to_string();
    }
    hardcoded.to_string()
}

/// Decide whether the `--style` override also applies to diagram icons.
///
/// - a name that exists as an icon style: use it for icons
/// - a name that exists only as an image style: do NOT apply to icons
/// - anything else is a literal description: applies to both
fn icon_style_override(
    cli_override: Option<&str>,
    is_image_style: impl Fn(&str) -> bool,
    is_icon_style: impl Fn(&str) -> bool,
) -> Option<String> {
    let s = cli_override?;
    if is_icon_style(s) {
        Some(s.to_string())
    } else if is_image_style(s) {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_style_override() {
        let image = |n: &str| n == "photo";
        let icon = |n: &str| n == "flat" || n == "both";
        let both_image = |n: &str| n == "photo" || n == "both";
        // Image-only style name: not applied to icons
        assert_eq!(icon_style_override(Some("photo"), image, icon), None);
        // Icon style name: applied
        assert_eq!(
            icon_style_override(Some("flat"), image, icon),
            Some("flat".to_string())
        );
        // Name that exists as both: applied (icon variant resolves it)
        assert_eq!(
            icon_style_override(Some("both"), both_image, icon),
            Some("both".to_string())
        );
        // Literal description: applied to both
        assert_eq!(
            icon_style_override(Some("watercolor sketch"), image, icon),
            Some("watercolor sketch".to_string())
        );
        // No override
        assert_eq!(icon_style_override(None, image, icon), None);
    }

    #[test]
    fn resolve_style_prefers_override_then_frontmatter_then_default() {
        let lookup = |n: &str| (n == "named").then_some("described");
        assert_eq!(
            resolve_style(Some("named"), Some("x"), None, lookup, "hard"),
            "described"
        );
        assert_eq!(
            resolve_style(None, Some("literal"), None, lookup, "hard"),
            "literal"
        );
        assert_eq!(
            resolve_style(None, None, Some("named"), lookup, "hard"),
            "described"
        );
        assert_eq!(
            resolve_style(None, None, Some("missing"), lookup, "hard"),
            "hard"
        );
    }
}
