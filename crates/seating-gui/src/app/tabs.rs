use super::*;
use iced::widget::column;

impl GuiApp {
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
        .style(toolbar_style);

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
        .style(toolbar_style);

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
        .style(toolbar_style);

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

    pub(super) fn view_optimize_tab(&self) -> Element<'_, Msg> {
        let seating_preview = if self.seating_csv.is_empty() {
            "No seating CSV yet. Run Optimize to generate a plan.".to_string()
        } else {
            self.seating_csv.clone()
        };

        let actions = container(
            row![
                text("Seed").size(13),
                text_input("42", &self.seed)
                    .on_input(Msg::SeedChanged)
                    .width(Length::Fixed(100.0)),
                text("Proximity").size(13),
                text_input("1.0", &self.proximity_weight)
                    .on_input(Msg::ProximityWeightChanged)
                    .width(Length::Fixed(100.0)),
                text("Used table").size(13),
                text_input("0.0", &self.used_table_weight)
                    .on_input(Msg::UsedTableWeightChanged)
                    .width(Length::Fixed(100.0)),
                text("Table size").size(13),
                text_input("1.0", &self.optimal_table_size_weight)
                    .on_input(Msg::OptimalTableSizeWeightChanged)
                    .width(Length::Fixed(100.0)),
                container(row![]).width(Length::Fill),
                button(text("Run Optimize").size(13))
                    .on_press(Msg::Optimize)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
                button(text("Save Seating CSV").size(13))
                    .on_press(Msg::SaveSeating)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        scrollable(
            column![
                actions,
                text("Current seating assignments").size(15),
                container(text(seating_preview).size(13))
                    .padding(14)
                    .width(Length::Fill)
                    .style(row_card_style),
            ]
            .spacing(12),
        )
        .into()
    }

    pub(super) fn view_seating_plan_tab(&self) -> Element<'_, Msg> {
        let controls = container(
            row![
                button(text("Export seating plan as SVG").size(13))
                    .on_press(Msg::ExportPlanSvg)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export seating plan as PNG").size(13))
                    .on_press(Msg::ExportPlanPng)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Zoom -").size(13))
                    .on_press(Msg::ZoomOut)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Zoom +").size(13))
                    .on_press(Msg::ZoomIn)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                text(format!("Zoom: {:.1}x", self.zoom)).size(13),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        let body: Element<'_, Msg> = match (&self.layout, &self.layout_svg) {
            (Some(layout), Some(svg_markup)) => scrollable(
                container(
                    svg(svg::Handle::from_memory(svg_markup.as_bytes().to_vec()))
                        .width(Length::Fixed(layout.width * self.zoom))
                        .height(Length::Fixed(layout.height * self.zoom)),
                )
                .width(Length::Shrink)
                .height(Length::Shrink),
            )
            .into(),
            _ => text("No valid seating plan to render yet.").into(),
        };

        column![
            controls,
            container(body)
                .padding(18)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(canvas_style)
        ]
        .spacing(12)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn view_diagnostics_tab(&self) -> Element<'_, Msg> {
        let errors = if self.validation_errors.is_empty() {
            vec!["No validation issues.".to_string()]
        } else {
            self.validation_errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };

        let paths = [
            format!("People path: {}", display_path(&self.people_path)),
            format!("Closeness path: {}", display_path(&self.closeness_path)),
            format!("Tables path: {}", display_path(&self.tables_path)),
            format!("Seating path: {}", display_path(&self.seating_path)),
        ];

        let content = paths.into_iter().fold(
            column![text("Validation errors").size(15)].spacing(8),
            |column, entry| column.push(text(entry)),
        );
        let content = errors
            .into_iter()
            .fold(content, |column, error| column.push(text(error)));

        scrollable(
            column![container(content)
                .padding(14)
                .width(Length::Fill)
                .style(row_card_style)]
            .spacing(12),
        )
        .into()
    }
}
