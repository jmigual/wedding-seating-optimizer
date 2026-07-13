use super::*;
use crate::components::{danger_button, primary_button, secondary_button, spacer, toolbar};
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_tables_tab(&self) -> Element<'_, Msg> {
        let generated_instances = self.generated_table_summary();
        let actions = toolbar(
            row![
                secondary_button("Import Tables CSV", Msg::ImportTables),
                secondary_button("Save Tables CSV", Msg::SaveTables),
                secondary_button("Export Tables CSV As...", Msg::ExportTables),
                spacer(),
                primary_button("Add Table Type", Msg::AddTableConfig),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );

        if self.table_configs.is_empty() {
            return column![
                actions,
                self.empty_state_card("No table types yet — Add Table Type or Import Tables CSV.")
            ]
            .spacing(12)
            .into();
        }

        let mut content = column![actions, text("Generated table instances")].spacing(12);

        if generated_instances.is_empty() {
            content = content.push(text("No generated tables yet.").size(13));
        }
        for summary in generated_instances {
            content = content.push(text(summary));
        }

        for (index, row_state) in self.table_configs.iter().enumerate() {
            let errors = self.table_errors(row_state);
            let people_per_side = column![
                text("people_per_side (top|right|bottom|left)").size(13),
                text_input("1|1|1|1", &row_state.people_per_side_input)
                    .on_input(move |value| Msg::UpdatePeoplePerSide(index, value))
                    .width(Length::Fixed(280.0)),
            ]
            .spacing(6);

            content = content.push(
                container(column![
                    row![
                        text(format!("Table type {}", index + 1)).width(Length::Fixed(110.0)),
                        text_input("table_type_id", &row_state.table_type_id)
                            .on_input(move |value| Msg::UpdateTableTypeId(index, value))
                            .width(Length::FillPortion(2)),
                        pick_list(
                            ShapeChoice::ALL.to_vec(),
                            Some(row_state.shape),
                            move |shape| Msg::UpdateTableShape(index, shape),
                        )
                        .width(Length::FillPortion(2)),
                        danger_button("Delete", Msg::DeleteTableConfig(index)),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                    row![
                        text_input("max_people", &row_state.max_people_input)
                            .on_input(move |value| Msg::UpdateMaxPeople(index, value))
                            .width(Length::FillPortion(1)),
                        text_input("min_people", &row_state.min_people_input)
                            .on_input(move |value| Msg::UpdateMinPeople(index, value))
                            .width(Length::FillPortion(1)),
                        text_input("recommended_people", &row_state.recommended_people_input)
                            .on_input(move |value| Msg::UpdateRecommendedPeople(index, value))
                            .width(Length::FillPortion(1)),
                        text_input("number_of_tables", &row_state.number_of_tables_input)
                            .on_input(move |value| Msg::UpdateNumberOfTables(index, value))
                            .width(Length::FillPortion(1)),
                    ]
                    .spacing(8),
                    people_per_side,
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
