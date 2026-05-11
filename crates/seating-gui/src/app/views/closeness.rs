use super::*;
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_closeness_tab(&self) -> Element<'_, Msg> {
        let options = reference_id_options(&self.people_data());
        let actions = container(
            row![
                button(text("Import Closeness CSV").size(13))
                    .on_press(Msg::ImportCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save Closeness CSV").size(13))
                    .on_press(Msg::SaveCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export Closeness CSV As...").size(13))
                    .on_press(Msg::ExportCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Rule").size(13))
                    .on_press(Msg::AddClosenessRule)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(crate::styles::toolbar_style);

        let mut content = column![actions].spacing(12);

        for (index, row_state) in self.closeness_rules.iter().enumerate() {
            let left_matches = self.reference_matches(&options, &row_state.rule.left_id);
            let right_matches = self.reference_matches(&options, &row_state.rule.right_id);
            let errors = self.closeness_errors(&row_state.rule, &row_state.score_input);

            content = content.push(
                container(column![
                    row![
                        text(format!("Rule {}", index + 1)).width(Length::Fixed(80.0)),
                        text_input("left id", &row_state.rule.left_id)
                            .on_input(move |value| Msg::UpdateClosenessLeft(index, value))
                            .width(Length::Fixed(180.0)),
                        text_input("right id", &row_state.rule.right_id)
                            .on_input(move |value| Msg::UpdateClosenessRight(index, value))
                            .width(Length::Fixed(180.0)),
                        text_input("score", &row_state.score_input)
                            .on_input(move |value| Msg::UpdateClosenessScore(index, value))
                            .width(Length::Fixed(120.0)),
                        button(text("Delete").size(13))
                            .on_press(Msg::DeleteClosenessRule(index))
                            .padding([8, 12])
                            .style(theme::Button::custom(AppButtonStyle::danger())),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                    row![
                        text(format!(
                            "Left: {}",
                            self.reference_label(&row_state.rule.left_id, &options)
                        ))
                        .width(Length::FillPortion(1)),
                        text(format!(
                            "Right: {}",
                            self.reference_label(&row_state.rule.right_id, &options)
                        ))
                        .width(Length::FillPortion(1)),
                    ]
                    .spacing(8),
                    self.suggestion_row("Left suggestions:", left_matches, move |value| {
                        Msg::SelectClosenessLeft(index, value)
                    },),
                    self.suggestion_row("Right suggestions:", right_matches, move |value| {
                        Msg::SelectClosenessRight(index, value)
                    },),
                    self.error_column(errors),
                ])
                .padding(12)
                .width(Length::Fill)
                .style(row_card_style),
            );
        }

        scrollable(content).into()
    }
}
