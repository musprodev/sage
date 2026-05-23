use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginPreset {
    Compact,
    Normal,
    Wide,
}

impl MarginPreset {
    pub fn next(self) -> Self {
        match self {
            Self::Compact => Self::Normal,
            Self::Normal => Self::Wide,
            Self::Wide => Self::Compact,
        }
    }

    pub fn to_padding(self) -> ratatui::widgets::Padding {
        match self {
            Self::Compact => ratatui::widgets::Padding::new(1, 1, 0, 0),
            Self::Normal => ratatui::widgets::Padding::new(4, 4, 1, 1),
            Self::Wide => ratatui::widgets::Padding::new(8, 8, 2, 2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSpacing {
    Single,
    Relaxed,
    Double,
}

impl LineSpacing {
    pub fn next(self) -> Self {
        match self {
            Self::Single => Self::Relaxed,
            Self::Relaxed => Self::Double,
            Self::Double => Self::Single,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextWidth {
    Narrow,
    Medium,
    Wide,
    Full,
}

impl TextWidth {
    pub fn next(self) -> Self {
        match self {
            Self::Narrow => Self::Medium,
            Self::Medium => Self::Wide,
            Self::Wide => Self::Full,
            Self::Full => Self::Narrow,
        }
    }

    pub fn to_columns(self) -> u16 {
        match self {
            Self::Narrow => 60,
            Self::Medium => 80,
            Self::Wide => 100,
            Self::Full => u16::MAX, // effectively full width
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderColorScheme {
    Default,
    Sepia,
    Paper,
    SoftDark,
}

impl ReaderColorScheme {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Sepia,
            Self::Sepia => Self::Paper,
            Self::Paper => Self::SoftDark,
            Self::SoftDark => Self::Default,
        }
    }

    /// Returns (fg, bg)
    pub fn colors(self, default_fg: Color, default_bg: Color) -> (Color, Color) {
        match self {
            Self::Default => (default_fg, default_bg),
            Self::Sepia => (Color::Rgb(0x5B, 0x46, 0x36), Color::Rgb(0xF4, 0xEE, 0xD8)),
            Self::Paper => (Color::Rgb(0x2E, 0x2E, 0x2E), Color::Rgb(0xFA, 0xFA, 0xFA)),
            Self::SoftDark => (Color::Rgb(0xB0, 0xB0, 0xB0), Color::Rgb(0x1E, 0x1E, 0x1E)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
}

impl TextAlignment {
    pub fn next(self) -> Self {
        match self {
            Self::Left => Self::Center,
            Self::Center => Self::Left,
        }
    }

    pub fn to_ratatui(self) -> ratatui::layout::Alignment {
        match self {
            Self::Left => ratatui::layout::Alignment::Left,
            Self::Center => ratatui::layout::Alignment::Center,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphSpacing {
    Compact,
    Normal,
    Relaxed,
}

impl ParagraphSpacing {
    pub fn next(self) -> Self {
        match self {
            Self::Compact => Self::Normal,
            Self::Normal => Self::Relaxed,
            Self::Relaxed => Self::Compact,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReaderSettings {
    pub margin_preset: MarginPreset,
    pub line_spacing: LineSpacing,
    pub text_width: TextWidth,
    pub color_scheme: ReaderColorScheme,
    pub alignment: TextAlignment,
    pub paragraph_spacing: ParagraphSpacing,
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self {
            margin_preset: MarginPreset::Normal,
            line_spacing: LineSpacing::Relaxed,
            text_width: TextWidth::Medium,
            color_scheme: ReaderColorScheme::Default,
            alignment: TextAlignment::Left,
            paragraph_spacing: ParagraphSpacing::Normal,
        }
    }
}
