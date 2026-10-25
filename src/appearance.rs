//! Codmin's existing Soft Orbit palette and compact operator typography.
use glory::prelude::*;
use sapid::{ColorScheme, Palette, Size, Theme, ThemeTokens, TypographyMetrics, TypographyRole};
const fn rgb(value: u32) -> Color {
    Color((value >> 16) as u8, (value >> 8) as u8, value as u8, 255)
}
const LIGHT: Palette = Palette {
    canvas: rgb(0xfbf7f4),
    surface: rgb(0xffffff),
    input: rgb(0xffffff),
    foreground: rgb(0x2a1f33),
    muted: rgb(0x76677e),
    accent: rgb(0xcb3d2f),
    accent_hover: rgb(0xc53628),
    accent_soft: rgb(0xfdeae8),
    accent_text: rgb(0xa93023),
    on_accent: rgb(0xffffff),
    focus_ring: rgb(0xf1776a),
    input_focus: rgb(0xf1776a),
    border: rgb(0xe1dde3),
    disabled: rgb(0xf1edea),
    disabled_text: rgb(0x83738c),
    danger: rgb(0xd52035),
    danger_fill: rgb(0xd52035),
    danger_soft: rgb(0xfcedef),
    success: rgb(0x277c64),
    success_soft: rgb(0xecf9f5),
    warning: rgb(0x915e12),
    warning_soft: rgb(0xfcf4e8),
    ..sapid::BLUE.base
};
const DARK: Palette = Palette {
    canvas: rgb(0x1a141f),
    surface: rgb(0x211b27),
    input: rgb(0x211b27),
    foreground: rgb(0xf6f2ef),
    muted: rgb(0xc7c2cb),
    accent: rgb(0xef8b80),
    accent_hover: rgb(0xf3a49b),
    accent_soft: rgb(0x462d2a),
    accent_text: rgb(0xf3a49b),
    on_accent: rgb(0x1c1424),
    focus_ring: rgb(0xef8b80),
    input_focus: rgb(0xef8b80),
    border: rgb(0x38313f),
    disabled: rgb(0x2e2833),
    disabled_text: rgb(0x9991a1),
    danger: rgb(0xee909b),
    danger_fill: rgb(0xc92c3e),
    danger_soft: rgb(0x402629),
    success: rgb(0x7eceb7),
    success_soft: rgb(0x213630),
    warning: rgb(0xefc380),
    warning_soft: rgb(0x372c1b),
    ..sapid::BLUE.base
};
pub fn theme(dark: bool) -> Theme {
    let mut theme = Theme::from_palette(
        "orbit",
        if dark {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        },
        if dark { DARK } else { LIGHT },
    )
    .expect("valid application palette");
    theme.radius = 8.;
    let mut tokens = ThemeTokens::default();
    for size in [
        Size::ExtraSmall,
        Size::Small,
        Size::Medium,
        Size::Large,
        Size::ExtraLarge,
    ] {
        for (role, font_size, line_height, weight) in [
            (TypographyRole::Body, 14., 22., 400.),
            (TypographyRole::Small, 13., 20., 400.),
            (TypographyRole::Heading1, 24., 32., 700.),
            (TypographyRole::Heading2, 18., 28., 600.),
            (TypographyRole::Heading3, 16., 24., 600.),
            (TypographyRole::Heading4, 14., 22., 600.),
        ] {
            tokens = tokens
                .with_typography(
                    role,
                    size,
                    TypographyMetrics {
                        font_size,
                        line_height,
                        weight: Some(weight),
                    },
                )
                .expect("finite typography");
        }
    }
    theme.with_tokens(tokens)
}
pub fn icon(name: &str) -> impl Widget + use<> {
    text()
        .kind(TextKind::Inline)
        .class(format!("codmin-icon icon-{name}"))
        .content("")
}
#[cfg(target_arch = "wasm32")]
pub fn install_styles(
    document: &web_sys::Document,
) -> std::result::Result<(), wasm_bindgen::JsValue> {
    let style = document.create_element("style")?;
    style.set_text_content(Some(&format!(
        "{}\n{}",
        theme(false)
            .stylesheet()
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?,
        theme(true)
            .stylesheet()
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?
    )));
    document
        .document_element()
        .ok_or("missing document root")?
        .append_child(&style)?;
    Ok(())
}
