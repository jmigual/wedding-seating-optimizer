use super::*;
use crate::components::{
    chip_button, danger_button, primary_button, secondary_button, spacer, toolbar,
};
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_people_tab(&self) -> Element<'_, Msg> {
        let actions = toolbar(
            row![
                secondary_button("Import People CSV", Msg::ImportPeople),
                secondary_button("Save People CSV", Msg::SavePeople),
                secondary_button("Export People CSV As...", Msg::ExportPeople),
                spacer(),
                primary_button("Add Person", Msg::AddPerson),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );

        if self.people.is_empty() {
            return column![
                actions,
                self.empty_state_card("No people yet — Add Person or Import People CSV.")
            ]
            .spacing(12)
            .into();
        }

        let mut content = column![actions].spacing(12);

        for (index, row_state) in self.people.iter().enumerate() {
            let table_type_options = self.person_table_type_options(&row_state.person.table_type);
            let locked_table_options = self.locked_table_options(row_state.person.locked_table);
            let locked_seat_options = self.locked_seat_options(&row_state.person);
            let groups = row_state.person.groups.iter().enumerate().fold(
                row![].spacing(6),
                |group_row, (group_index, group)| {
                    group_row.push(chip_button(
                        format!("{group} x"),
                        Msg::RemoveGroup(index, group_index),
                    ))
                },
            );
            let errors = self.person_errors(&row_state.person);

            let card = column![
                row![
                    text(format!("Person {}", index + 1)).width(Length::Fixed(90.0)),
                    text_input("id", &row_state.person.id)
                        .on_input(move |value| Msg::UpdatePersonId(index, value))
                        .width(Length::FillPortion(2)),
                    text_input("name", &row_state.person.name)
                        .on_input(move |value| Msg::UpdatePersonName(index, value))
                        .width(Length::FillPortion(3)),
                    pick_list(
                        table_type_options.clone(),
                        selected_string_choice(&table_type_options, &row_state.person.table_type),
                        move |choice| Msg::UpdatePersonTableType(index, choice),
                    )
                    .placeholder("table type")
                    .width(Length::FillPortion(3)),
                    pick_list(
                        locked_table_options.clone(),
                        selected_usize_choice(&locked_table_options, row_state.person.locked_table),
                        move |choice| Msg::UpdatePersonLockedTable(index, choice),
                    )
                    .placeholder("locked table")
                    .width(Length::FillPortion(2)),
                    pick_list(
                        locked_seat_options.clone(),
                        selected_usize_choice(&locked_seat_options, row_state.person.locked_seat),
                        move |choice| Msg::UpdatePersonLockedSeat(index, choice),
                    )
                    .placeholder("locked seat")
                    .width(Length::FillPortion(2)),
                    danger_button("Delete", Msg::DeletePerson(index)),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                row![
                    text("Groups:"),
                    groups,
                    text_input("new group", &row_state.new_group)
                        .on_input(move |value| Msg::UpdateNewGroup(index, value))
                        .width(Length::FillPortion(1)),
                    secondary_button("Add Group", Msg::AddGroup(index)),
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
