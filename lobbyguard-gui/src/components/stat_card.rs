//! Stat card component for displaying metrics.

use xilem::masonry::properties::types::AsUnit;
use xilem::style::Style as _;
use xilem::view::{flex_col, label, sized_box};
use xilem::{Color, WidgetView};

use crate::theme::Theme;

/// Render a stat card with title and value.
pub fn stat_card<State: 'static, Action: 'static>(
	title: &'static str, value: &str, accent_color: Color, theme: &Theme,
) -> impl WidgetView<State, Action> + use<State, Action> {
	sized_box(
		flex_col((
			label(title).text_size(8.0).color(theme.text_muted),
			label(value).text_size(16.0).color(accent_color),
		))
		.gap(2.px()),
	)
	.padding(8.0)
	.background_color(theme.bg_secondary)
	.border(theme.border_primary, 1.0)
	.corner_radius(6.0)
}
