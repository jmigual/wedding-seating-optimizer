#![allow(dead_code)]

use iced::widget::{button, container, row, text, Button, Container};
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

pub(crate) fn spacer<'a>() -> Container<'a, Msg, Theme> {
    container(row![]).width(Length::Fill)
}

pub(crate) fn toolbar<'a>(content: impl Into<Element<'a, Msg>>) -> Container<'a, Msg, Theme> {
    container(content)
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style)
}
