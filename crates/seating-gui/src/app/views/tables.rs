use super::*;
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_tables_tab(&self) -> Element<'_, Msg> {
        let generated_instances = self.generated_table_summary();
        let actions = container(
            row![
                button(text("Import Tables CSV").size(13))
                    .on_press(Msg::ImportTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save Tables CSV").size(13))
                    .on_press(Msg::SaveTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export Tables CSV As...").size(13))
                    .on_press(Msg::ExportTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Table Type").size(13))
                    .on_press(Msg::AddTableConfig)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(crate::styles::toolbar_style);

        let mut content = column![actions, text("Generated table instances")].spacing(12);

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
                            .width(Length::Fixed(180.0)),
                        pick_list(
                            ShapeChoice::ALL.to_vec(),
                            Some(row_state.shape),
                            move |shape| Msg::UpdateTableShape(index, shape),
                        )
                        .width(Length::Fixed(150.0)),
                        button(text("Delete").size(13))
                            .on_press(Msg::DeleteTableConfig(index))
                            .padding([8, 12])
                            .style(theme::Button::custom(AppButtonStyle::danger())),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                    row![
                        text_input("max_people", &row_state.max_people_input)
                            .on_input(move |value| Msg::UpdateMaxPeople(index, value))
                            .width(Length::Fixed(120.0)),
                        text_input("min_people", &row_state.min_people_input)
                            .on_input(move |value| Msg::UpdateMinPeople(index, value))
                            .width(Length::Fixed(120.0)),
                        text_input("recommended_people", &row_state.recommended_people_input)
                            .on_input(move |value| Msg::UpdateRecommendedPeople(index, value))
                            .width(Length::Fixed(170.0)),
                        text_input("number_of_tables", &row_state.number_of_tables_input)
                            .on_input(move |value| Msg::UpdateNumberOfTables(index, value))
                            .width(Length::Fixed(160.0)),
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
