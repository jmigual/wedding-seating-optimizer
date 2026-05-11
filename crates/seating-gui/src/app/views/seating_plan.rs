use super::*;
use iced::widget::{column, row};

impl GuiApp {
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
        .style(crate::styles::toolbar_style);

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
}
