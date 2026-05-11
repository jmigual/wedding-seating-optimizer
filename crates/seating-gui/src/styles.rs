use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

#[derive(Debug, Clone, Copy)]
enum AppButtonKind {
    Tab { active: bool },
    Primary,
    Secondary,
    Danger,
    Chip,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AppButtonStyle {
    kind: AppButtonKind,
}

impl AppButtonStyle {
    pub(crate) fn tab(active: bool) -> Self {
        Self {
            kind: AppButtonKind::Tab { active },
        }
    }

    pub(crate) fn primary() -> Self {
        Self {
            kind: AppButtonKind::Primary,
        }
    }

    pub(crate) fn secondary() -> Self {
        Self {
            kind: AppButtonKind::Secondary,
        }
    }

    pub(crate) fn danger() -> Self {
        Self {
            kind: AppButtonKind::Danger,
        }
    }

    pub(crate) fn chip() -> Self {
        Self {
            kind: AppButtonKind::Chip,
        }
    }
}

impl button::StyleSheet for AppButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let (background, text_color, border_color) = match self.kind {
            AppButtonKind::Tab { active: true } => {
                (rgb(58, 83, 116), rgb(246, 249, 252), rgb(81, 114, 154))
            }
            AppButtonKind::Tab { active: false } => {
                (rgb(31, 48, 66), rgb(223, 231, 239), rgb(31, 48, 66))
            }
            AppButtonKind::Primary => (rgb(73, 112, 154), rgb(249, 252, 255), rgb(88, 134, 184)),
            AppButtonKind::Secondary => (rgb(32, 49, 67), rgb(230, 236, 243), rgb(42, 62, 82)),
            AppButtonKind::Danger => (rgb(116, 55, 62), rgb(255, 244, 244), rgb(144, 72, 82)),
            AppButtonKind::Chip => (rgb(40, 59, 78), rgb(230, 238, 247), rgb(62, 83, 105)),
        };

        button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(match self.kind {
            AppButtonKind::Tab { active: true } => rgb(67, 96, 132),
            AppButtonKind::Tab { active: false } => rgb(39, 58, 78),
            AppButtonKind::Primary => rgb(84, 127, 173),
            AppButtonKind::Secondary => rgb(41, 60, 80),
            AppButtonKind::Danger => rgb(133, 63, 72),
            AppButtonKind::Chip => rgb(48, 70, 92),
        }));
        appearance
    }
}

pub(crate) fn app_background_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(12, 20, 28))),
        text_color: Some(rgb(232, 238, 245)),
        ..container::Appearance::default()
    }
}

pub(crate) fn shell_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(20, 32, 43))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Appearance::default()
    }
}

pub(crate) fn header_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(11, 17, 24))),
        text_color: Some(rgb(238, 243, 248)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: [8.0, 8.0, 8.0, 8.0].into(),
        },
        ..container::Appearance::default()
    }
}

pub(crate) fn message_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(32, 49, 67))),
        text_color: Some(rgb(229, 237, 246)),
        border: Border {
            color: rgb(32, 49, 67),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

pub(crate) fn toolbar_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(18, 29, 39))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(42, 63, 84),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

pub(crate) fn row_card_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(17, 28, 38))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(44, 66, 88),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

pub(crate) fn canvas_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(10, 17, 24))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}
