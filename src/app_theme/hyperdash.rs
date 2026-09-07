use crate::app_state::TradingTerminal;
use iced::Color;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn palette_matches_hyperdash_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x19, 0x16, 0x13])
            && super::rgba8_eq(palette.text, [0xD5, 0xD1, 0xCD])
            && super::rgba8_eq(palette.primary, [0xFD, 0x46, 0x12])
            && super::rgba8_eq(palette.success, [0x5C, 0xC0, 0x9B])
            && super::rgba8_eq(palette.warning, [0xFA, 0xCC, 0x15])
            && super::rgba8_eq(palette.danger, [0xE2, 0x3D, 0x59])
    }

    pub(crate) fn hyperdash_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        // Warm surfaces and orange actions sampled from the Hyperdash reference.
        let bg = color(0x19, 0x16, 0x13);
        let text = color(0xD5, 0xD1, 0xCD);
        let text_muted = color(0x92, 0x8D, 0x86);
        let text_dim = color(0x77, 0x74, 0x6D);
        let white = Color::WHITE;
        let orange = color(0xFD, 0x46, 0x12);
        let orange_bright = color(0xFE, 0x81, 0x5E);
        let green = color(0x5C, 0xC0, 0x9B);
        let yellow = color(0xFA, 0xCC, 0x15);
        let red = color(0xE2, 0x3D, 0x59);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(color(0x11, 0x0F, 0x0D), text_dim),
                weaker: super::pair(color(0x14, 0x12, 0x10), text_dim),
                weak: super::pair(color(0x1D, 0x1A, 0x18), text_muted),
                neutral: super::pair(color(0x22, 0x1F, 0x1C), text),
                // Pane bodies use `strong`, including the chart's flat background.
                strong: super::pair(bg, white),
                stronger: super::pair(color(0x26, 0x24, 0x22), white),
                strongest: super::pair(color(0x33, 0x31, 0x2E), white),
            },
            primary: Primary {
                base: super::pair(orange, white),
                weak: super::pair(color(0x31, 0x1C, 0x13), orange_bright),
                strong: super::pair(orange_bright, bg),
            },
            secondary: Secondary {
                base: super::pair(text_muted, bg),
                weak: super::pair(text_dim, bg),
                strong: super::pair(text, bg),
            },
            success: Success {
                base: super::pair(green, bg),
                weak: super::pair(color(0x15, 0x33, 0x28), green),
                strong: super::pair(color(0x7B, 0xD4, 0xB1), bg),
            },
            warning: Warning {
                base: super::pair(yellow, bg),
                weak: super::pair(color(0x33, 0x2B, 0x16), yellow),
                strong: super::pair(color(0xFC, 0xDD, 0x65), bg),
            },
            danger: Danger {
                base: super::pair(red, white),
                weak: super::pair(color(0x33, 0x18, 0x1A), red),
                strong: super::pair(color(0xF0, 0x65, 0x7C), bg),
            },
            is_dark: true,
        }
    }
}
