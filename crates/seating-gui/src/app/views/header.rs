use super::*;
use crate::components::{logo_mark, secondary_button, spacer};
use crate::styles::header_style;
use iced::widget::row;

impl GuiApp {
    pub(super) fn view_header(&self) -> Element<'_, Msg> {
        container(
            row![
                logo_mark(),
                text("Wedding Seating").size(16),
                spacer(),
                secondary_button("New", Msg::NewProject),
                secondary_button("Open", Msg::OpenProject),
                secondary_button("Save", Msg::SaveProject),
                secondary_button("Save As", Msg::SaveProjectAs),
                secondary_button("Export CSVs", Msg::ExportProjectCsv),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .padding([14, 28])
        .width(Length::Fill)
        .style(header_style)
        .into()
    }
}
