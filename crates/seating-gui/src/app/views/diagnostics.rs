use super::*;
use iced::widget::column;

impl GuiApp {
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
            format!("Project path: {}", display_path(&self.project_path)),
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
