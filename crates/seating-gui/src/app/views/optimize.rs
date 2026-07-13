use super::*;
use crate::components::{secondary_button, spacer, toolbar};
use crate::styles::muted_text_color;
use iced::widget::{column, row};
use std::collections::BTreeMap;

impl GuiApp {
    pub(super) fn view_optimize_tab(&self) -> Element<'_, Msg> {
        let run_optimize_button = button(
            text(if self.is_optimizing {
                "Optimizing…"
            } else {
                "Run Optimize"
            })
            .size(13),
        )
        .padding([9, 14])
        .style(theme::Button::custom(AppButtonStyle::primary()));
        let run_optimize_button = if self.is_optimizing {
            run_optimize_button
        } else {
            run_optimize_button.on_press(Msg::Optimize)
        };

        let actions = toolbar(
            row![
                text("Seed").size(13),
                text_input("42", &self.seed)
                    .on_input(Msg::SeedChanged)
                    .width(Length::Fixed(100.0)),
                text("Attempts").size(13),
                text_input("10", &self.attempts)
                    .on_input(Msg::AttemptsChanged)
                    .width(Length::Fixed(80.0)),
                text("Iterations").size(13),
                text_input("200", &self.iterations)
                    .on_input(Msg::IterationsChanged)
                    .width(Length::Fixed(80.0)),
                text("Solutions").size(13),
                text_input("1", &self.solutions)
                    .on_input(Msg::SolutionsChanged)
                    .width(Length::Fixed(80.0)),
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
                spacer(),
                run_optimize_button,
                secondary_button("Save Seating CSV", Msg::SaveSeating),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );

        let weights_hint = text(
            "Proximity rewards seating close guests together; Used table penalizes spreading \
             guests across more tables than needed; Table size penalizes drifting from a \
             table's recommended headcount.",
        )
        .size(12)
        .style(iced::theme::Text::Color(muted_text_color()));

        let assignments_body: Element<'_, Msg> = if self.assignments.is_empty() {
            text("No seating assignments yet — run Optimize to generate a plan.")
                .size(13)
                .into()
        } else {
            let mut by_table: BTreeMap<usize, Vec<&SeatingAssignment>> = BTreeMap::new();
            for assignment in &self.assignments {
                by_table
                    .entry(assignment.table_number)
                    .or_default()
                    .push(assignment);
            }

            let mut tables = column![].spacing(10);
            for (table_number, mut seats) in by_table {
                seats.sort_by_key(|assignment| assignment.seat_index);
                let table_type = seats
                    .first()
                    .map(|assignment| assignment.table_type.as_str())
                    .unwrap_or("");
                let mut table_card =
                    column![text(format!("Table {table_number} — {table_type}")).size(14)]
                        .spacing(4);
                for assignment in seats {
                    table_card = table_card.push(
                        text(format!(
                            "Seat {}: {}",
                            assignment.seat_index, assignment.person_name
                        ))
                        .size(13),
                    );
                }
                tables = tables.push(
                    container(table_card)
                        .padding(12)
                        .width(Length::Fill)
                        .style(row_card_style),
                );
            }
            tables.into()
        };

        scrollable(
            column![
                actions,
                weights_hint,
                text("Current seating assignments").size(15),
                assignments_body,
            ]
            .spacing(12),
        )
        .into()
    }
}
