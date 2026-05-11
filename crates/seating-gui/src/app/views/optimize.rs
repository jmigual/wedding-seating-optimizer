use super::*;
use iced::widget::{column, row};

impl GuiApp {
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
        .style(crate::styles::toolbar_style);

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
}
