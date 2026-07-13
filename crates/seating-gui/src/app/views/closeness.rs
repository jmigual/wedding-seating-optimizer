use super::*;
use crate::components::{danger_button, primary_button, secondary_button, spacer, toolbar};
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_closeness_tab(&self) -> Element<'_, Msg> {
        let options = reference_id_options(&self.people_data());
        let actions = toolbar(
            row![
                secondary_button("Import Closeness CSV", Msg::ImportCloseness),
                secondary_button("Save Closeness CSV", Msg::SaveCloseness),
                secondary_button("Export Closeness CSV As...", Msg::ExportCloseness),
                spacer(),
                primary_button("Add Rule", Msg::AddClosenessRule),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );

        if self.closeness_rules.is_empty() {
            return column![
                actions,
                self.empty_state_card("No closeness rules yet — Add Rule or Import Closeness CSV.")
            ]
            .spacing(12)
            .into();
        }

        let mut content = column![actions].spacing(12);

        for (index, row_state) in self.closeness_rules.iter().enumerate() {
            let errors = self.closeness_errors(
                &row_state.left_id,
                &row_state.right_id,
                &row_state.score_input,
            );
            let left_suggestions = self.suggestion_row(
                "Left suggestions:",
                &row_state.left_id,
                &options,
                move |value| Msg::SelectClosenessLeft(index, value),
            );
            let right_suggestions = self.suggestion_row(
                "Right suggestions:",
                &row_state.right_id,
                &options,
                move |value| Msg::SelectClosenessRight(index, value),
            );

            let mut card = column![
                row![
                    text(format!("Rule {}", index + 1)).width(Length::Fixed(80.0)),
                    text_input("left id", &row_state.left_id)
                        .on_input(move |value| Msg::UpdateClosenessLeft(index, value))
                        .width(Length::FillPortion(2)),
                    text_input("right id", &row_state.right_id)
                        .on_input(move |value| Msg::UpdateClosenessRight(index, value))
                        .width(Length::FillPortion(2)),
                    text_input("score", &row_state.score_input)
                        .on_input(move |value| Msg::UpdateClosenessScore(index, value))
                        .width(Length::FillPortion(1)),
                    danger_button("Delete", Msg::DeleteClosenessRule(index)),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                row![
                    text(format!(
                        "Left: {}",
                        reference_label(&row_state.left_id, &options)
                    ))
                    .width(Length::FillPortion(1)),
                    text(format!(
                        "Right: {}",
                        reference_label(&row_state.right_id, &options)
                    ))
                    .width(Length::FillPortion(1)),
                ]
                .spacing(8),
            ]
            .spacing(8);
            if let Some(suggestions) = left_suggestions {
                card = card.push(suggestions);
            }
            if let Some(suggestions) = right_suggestions {
                card = card.push(suggestions);
            }
            card = card.push(self.error_column(errors));

            content = content.push(
                container(card)
                    .padding(12)
                    .width(Length::Fill)
                    .style(row_card_style),
            );
        }

        scrollable(content).into()
    }
}
