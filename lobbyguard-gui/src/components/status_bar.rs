//! Status bar component showing engine state and version.

use xilem::WidgetView;
use xilem::masonry::properties::types::AsUnit;
use xilem::style::Style as _;
use xilem::view::{FlexSpacer, button, flex_col, flex_row, label, sized_box};

use crate::actions::Action;
use crate::theme::{Theme, ThemeMode};

/// Render the status bar at the bottom of the sidebar.
pub fn status_bar<State: 'static>(
	is_active: bool, theme: &Theme, theme_mode: ThemeMode,
) -> impl WidgetView<State, Action> + use<State> {
	flex_col((
		flex_row((
			// Status indicator dot
			sized_box(label(""))
				.width(8.px())
				.height(8.px())
				.corner_radius(4.0)
				.background_color(if is_active {
					theme.success
				} else {
					theme.text_muted
				}),
			label(if is_active {
				"Engine Active"
			} else {
				"Engine Idle"
			})
			.text_size(12.0)
			.color(theme.text_muted),
		))
		.gap(8.px()),
		flex_row((
			label("v2.5.0").text_size(10.0).color(theme.text_muted),
			FlexSpacer::Flex(1.0),
			// Theme toggle button
			button(
				label(if theme_mode == ThemeMode::Dark {
					"☀️"
				} else {
					"🌙"
				})
				.text_size(12.0),
				move |_: &mut State| Action::ToggleTheme,
			)
			.background_color(xilem::Color::TRANSPARENT)
			.corner_radius(4.0),
		)),
	))
	.gap(8.px())
	.padding(16.0)
	.background_color(theme.bg_secondary)
	.border(theme.border_primary, 1.0)
	.corner_radius(8.0)
}
