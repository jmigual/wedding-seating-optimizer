use super::*;
use crate::components::{secondary_button, spacer, toolbar};
use iced::widget::scrollable::{Direction, Properties};
use iced::widget::{column, row};

impl GuiApp {
    pub(super) fn view_seating_plan_tab(&self) -> Element<'_, Msg> {
        let controls = toolbar(
            row![
                secondary_button("Export seating plan as SVG", Msg::ExportPlanSvg),
                secondary_button("Export seating plan as PNG", Msg::ExportPlanPng),
                spacer(),
                button(text("Zoom -").size(13))
                    .on_press(Msg::ZoomOut)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Zoom +").size(13))
                    .on_press(Msg::ZoomIn)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                secondary_button("Reset zoom", Msg::ZoomReset),
                text(format!("Zoom: {:.1}x", self.zoom)).size(13),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        );

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
            .direction(Direction::Both {
                vertical: Properties::default(),
                horizontal: Properties::default(),
            })
            .into(),
            _ => match &self.layout_error {
                Some(reason) => text(format!("Seating plan render failed: {reason}")).into(),
                None if !self.validation_errors.is_empty() => {
                    text("Project data is invalid — see Diagnostics tab.").into()
                }
                None if self.assignments.is_empty() => {
                    text("Not optimized yet — run Optimize to generate a plan.").into()
                }
                None => text("No valid seating plan to render yet.").into(),
            },
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
