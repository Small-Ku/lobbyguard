//! Mode selector component - allows switching between session filter modes.

use xilem::masonry::properties::types::{AsUnit, CrossAxisAlignment, MainAxisAlignment};
use xilem::style::Style as _;
use xilem::view::{FlexExt as _, FlexSpacer, button, flex_col, flex_row, label, sized_box};
use xilem::{Color, WidgetView};

use crate::actions::{Action, SessionMode};
use crate::theme::Theme;

/// Render the mode selector with four mode cards.
pub fn mode_selector<State: 'static>(
	current_mode: SessionMode, theme: &Theme,
) -> impl WidgetView<State, Action> + use<State> {
	flex_row((
		sized_box(mode_card(
			SessionMode::Standard,
			current_mode,
			theme,
			theme.text_secondary,
		))
		.flex(1.0),
		sized_box(mode_card(
			SessionMode::Solo,
			current_mode,
			theme,
			theme.accent_emerald_light,
		))
		.flex(1.0),
		sized_box(mode_card(
			SessionMode::Locked,
			current_mode,
			theme,
			theme.accent_cyan,
		))
		.flex(1.0),
		sized_box(mode_card(
			SessionMode::Disconnect,
			current_mode,
			theme,
			theme.accent_rose,
		))
		.flex(1.0),
	))
	.gap(16.px())
}

/// Individual mode card.
fn mode_card<State: 'static>(
	mode: SessionMode, current_mode: SessionMode, theme: &Theme, accent_color: Color,
) -> impl WidgetView<State, Action> + use<State> {
	let is_active = current_mode == mode;
	let bg = if is_active {
		theme.bg_tertiary
	} else {
		theme.bg_secondary
	};
	let border_color = if is_active {
		accent_color
	} else {
		theme.border_primary
	};

	sized_box(
		button(
			sized_box(
				flex_col((
					flex_row((
						// Status indicator
						sized_box(label(""))
							.width(24.px())
							.height(24.px())
							.corner_radius(12.0)
							.background_color(if is_active {
								accent_color.with_alpha(0.3)
							} else {
								theme.bg_tertiary
							}),
						FlexSpacer::Flex(1.0),
						// Active badge
						if is_active {
							sized_box(label("ACTIVE").text_size(10.0).color(theme.text_primary))
								.padding(4.0)
								.background_color(theme.bg_primary.with_alpha(0.5))
								.corner_radius(12.0)
								.boxed()
						} else {
							sized_box(label("")).width(0.px()).boxed()
						},
					))
					.main_axis_alignment(MainAxisAlignment::Start),
					// Mode name
					label(mode.name()).text_size(18.0).color(if is_active {
						theme.text_primary
					} else {
						theme.text_secondary
					}),
					// Description
					label(mode.description())
						.text_size(12.0)
						.color(theme.text_muted),
				))
				.gap(12.px())
				.cross_axis_alignment(CrossAxisAlignment::Start),
			)
			.padding(16.0),
			move |_: &mut State| Action::SetMode(mode),
		)
		.background_color(bg)
		.border(border_color, if is_active { 2.0 } else { 1.0 })
		.corner_radius(12.0),
	)
	.height(140.px())
}
