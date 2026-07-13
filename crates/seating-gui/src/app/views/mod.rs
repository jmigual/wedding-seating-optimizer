mod closeness;
mod diagnostics;
mod header;
mod optimize;
mod people;
mod seating_plan;
mod tables;

use super::*;
use crate::components::chip_button;
use crate::state::MessageKind;
use crate::styles::{error_message_style, error_text_color, success_message_style};
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_shell(&self) -> Element<'_, Msg> {
        let tabs = Tab::ALL.into_iter().fold(
            row![].spacing(8).align_items(Alignment::Center),
            |tabs, tab| tabs.push(self.tab_button(tab)),
        );

        let content = match self.active_tab {
            Tab::People => self.view_people_tab(),
            Tab::Closeness => self.view_closeness_tab(),
            Tab::Tables => self.view_tables_tab(),
            Tab::Optimize => self.view_optimize_tab(),
            Tab::SeatingPlan => self.view_seating_plan_tab(),
            Tab::Diagnostics => self.view_diagnostics_tab(),
        };

        let message_container_style = match self.message_kind {
            MessageKind::Info => message_style,
            MessageKind::Success => success_message_style,
            MessageKind::Error => error_message_style,
        };
        let message = container(text(&self.message).size(13))
            .padding([12, 18])
            .width(Length::Fill)
            .style(message_container_style);

        let shell = container(
            column![
                self.view_header(),
                container(tabs)
                    .padding([20, 28, 10, 28])
                    .width(Length::Fill),
                container(message)
                    .padding([0, 28, 10, 28])
                    .width(Length::Fill),
                container(content)
                    .padding([18, 28, 28, 28])
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .max_width(1440)
        .style(shell_style);

        container(shell)
            .padding(34)
            .center_x()
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background_style)
            .into()
    }

    pub(super) fn tab_button(&self, tab: Tab) -> Element<'_, Msg> {
        button(text(tab.label()).size(13))
            .on_press(Msg::SelectTab(tab))
            .padding([10, 14])
            .style(theme::Button::custom(AppButtonStyle::tab(
                tab == self.active_tab,
            )))
            .into()
    }

    /// Suggestion chips for a reference-id text input. Returns `None` (render
    /// nothing) when the query is empty, has no matches, or already exactly
    /// names an option — the common cases where the row is pure chrome.
    pub(super) fn suggestion_row<F>(
        &self,
        label: &str,
        query: &str,
        options: &[seating_core::ReferenceIdOption],
        on_press: F,
    ) -> Option<Element<'_, Msg>>
    where
        F: 'static + Clone + Fn(String) -> Msg,
    {
        let trimmed = query.trim();
        if trimmed.is_empty() || options.iter().any(|option| option.id == trimmed) {
            return None;
        }
        let suggestions = reference_matches(options, query);
        if suggestions.is_empty() {
            return None;
        }
        let row = suggestions.into_iter().fold(
            row![text(label)].spacing(6).align_items(Alignment::Center),
            |suggestion_row, suggestion| {
                suggestion_row.push(chip_button(
                    suggestion.label.clone(),
                    on_press.clone()(suggestion.id.clone()),
                ))
            },
        );
        Some(row.into())
    }

    pub(super) fn error_column(&self, errors: Vec<String>) -> Element<'_, Msg> {
        errors
            .into_iter()
            .fold(column![].spacing(4), |column, error| {
                column.push(
                    text(format!("⚠ {error}"))
                        .size(13)
                        .style(iced::theme::Text::Color(error_text_color())),
                )
            })
            .into()
    }

    /// Centered hint card shown in place of an empty list tab.
    pub(super) fn empty_state_card(&self, message: &str) -> Element<'_, Msg> {
        container(text(message).size(13))
            .padding(24)
            .width(Length::Fill)
            .center_x()
            .style(row_card_style)
            .into()
    }
}
