mod closeness;
mod diagnostics;
mod header;
mod optimize;
mod people;
mod seating_plan;
mod tables;

use super::*;
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

        let message = container(text(&self.message).size(13))
            .padding([12, 18])
            .width(Length::Fill)
            .style(message_style);

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
            .width(Length::Fixed(tab.width()))
            .style(theme::Button::custom(AppButtonStyle::tab(
                tab == self.active_tab,
            )))
            .into()
    }

    pub(super) fn suggestion_row<F>(
        &self,
        label: &str,
        suggestions: Vec<seating_core::ReferenceIdOption>,
        on_press: F,
    ) -> Element<'_, Msg>
    where
        F: 'static + Clone + Fn(String) -> Msg,
    {
        let row = suggestions.into_iter().fold(
            row![text(label)].spacing(6).align_items(Alignment::Center),
            |suggestion_row, suggestion| {
                suggestion_row.push(
                    button(text(suggestion.label.clone()).size(12))
                        .on_press(on_press.clone()(suggestion.id.clone()))
                        .padding([5, 9])
                        .style(theme::Button::custom(AppButtonStyle::chip())),
                )
            },
        );
        row.into()
    }

    pub(super) fn error_column(&self, errors: Vec<String>) -> Element<'_, Msg> {
        errors
            .into_iter()
            .fold(column![].spacing(4), |column, error| {
                column.push(text(format!("⚠ {error}")))
            })
            .into()
    }
}
