use super::*;
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_people_tab(&self) -> Element<'_, Msg> {
        let actions = container(
            row![
                button(text("Import People CSV").size(13))
                    .on_press(Msg::ImportPeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save People CSV").size(13))
                    .on_press(Msg::SavePeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export People CSV As...").size(13))
                    .on_press(Msg::ExportPeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Person").size(13))
                    .on_press(Msg::AddPerson)
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

        for (index, row_state) in self.people.iter().enumerate() {
            let table_type_options = self.person_table_type_options(&row_state.person.table_type);
            let locked_table_options = self.locked_table_options(row_state.person.locked_table);
            let locked_seat_options = self.locked_seat_options(&row_state.person);
            let groups = row_state.person.groups.iter().enumerate().fold(
                row![].spacing(6),
                |group_row, (group_index, group)| {
                    group_row.push(
                        button(text(format!("{group} x")).size(12))
                            .on_press(Msg::RemoveGroup(index, group_index))
                            .padding([5, 9])
                            .style(theme::Button::custom(AppButtonStyle::chip())),
                    )
                },
            );
            let errors = self.person_errors(&row_state.person);

            let card = column![
                row![
                    text(format!("Person {}", index + 1)).width(Length::Fixed(90.0)),
                    text_input("id", &row_state.person.id)
                        .on_input(move |value| Msg::UpdatePersonId(index, value))
                        .width(Length::Fixed(140.0)),
                    text_input("name", &row_state.person.name)
                        .on_input(move |value| Msg::UpdatePersonName(index, value))
                        .width(Length::Fixed(180.0)),
                    pick_list(
                        table_type_options.clone(),
                        selected_string_choice(&table_type_options, &row_state.person.table_type),
                        move |choice| Msg::UpdatePersonTableType(index, choice),
                    )
                    .placeholder("table type")
                    .width(Length::Fixed(220.0)),
                    pick_list(
                        locked_table_options.clone(),
                        selected_usize_choice(&locked_table_options, row_state.person.locked_table),
                        move |choice| Msg::UpdatePersonLockedTable(index, choice),
                    )
                    .placeholder("locked table")
                    .width(Length::Fixed(170.0)),
                    pick_list(
                        locked_seat_options.clone(),
                        selected_usize_choice(&locked_seat_options, row_state.person.locked_seat),
                        move |choice| Msg::UpdatePersonLockedSeat(index, choice),
                    )
                    .placeholder("locked seat")
                    .width(Length::Fixed(170.0)),
                    button(text("Delete").size(13))
                        .on_press(Msg::DeletePerson(index))
                        .padding([8, 12])
                        .style(theme::Button::custom(AppButtonStyle::danger())),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                row![
                    text("Groups:"),
                    groups,
                    text_input("new group", &row_state.new_group)
                        .on_input(move |value| Msg::UpdateNewGroup(index, value))
                        .width(Length::Fixed(180.0)),
                    button(text("Add Group").size(13))
                        .on_press(Msg::AddGroup(index))
                        .padding([8, 12])
                        .style(theme::Button::custom(AppButtonStyle::secondary())),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                self.error_column(errors),
            ]
            .spacing(8);

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
