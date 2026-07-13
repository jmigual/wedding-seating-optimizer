use iced::widget::{button, container, row, svg, text, Button, Container};
use iced::{theme, Element, Length, Theme};

use crate::state::Msg;
use crate::styles::{toolbar_style, AppButtonStyle};

pub(crate) fn primary_button<'a>(label: &'a str, message: Msg) -> Button<'a, Msg, Theme> {
    button(text(label).size(13))
        .on_press(message)
        .padding([9, 14])
        .style(theme::Button::custom(AppButtonStyle::primary()))
}

pub(crate) fn secondary_button<'a>(label: &'a str, message: Msg) -> Button<'a, Msg, Theme> {
    button(text(label).size(13))
        .on_press(message)
        .padding([9, 14])
        .style(theme::Button::custom(AppButtonStyle::secondary()))
}

pub(crate) fn danger_button<'a>(label: &'a str, message: Msg) -> Button<'a, Msg, Theme> {
    button(text(label).size(13))
        .on_press(message)
        .padding([8, 12])
        .style(theme::Button::custom(AppButtonStyle::danger()))
}

pub(crate) fn chip_button<'a>(label: String, message: Msg) -> Button<'a, Msg, Theme> {
    button(text(label).size(12))
        .on_press(message)
        .padding([5, 9])
        .style(theme::Button::custom(AppButtonStyle::chip()))
}

pub(crate) fn spacer<'a>() -> Container<'a, Msg, Theme> {
    container(row![]).width(Length::Fill)
}

pub(crate) fn toolbar<'a>(content: impl Into<Element<'a, Msg>>) -> Container<'a, Msg, Theme> {
    container(content)
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style)
}

pub(crate) fn logo_mark<'a>() -> Container<'a, Msg, Theme> {
    container(
        svg(svg::Handle::from_memory(
            include_bytes!("../assets/app-icon.svg").to_vec(),
        ))
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0)),
    )
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
}
