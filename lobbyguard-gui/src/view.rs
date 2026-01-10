//! View module - page layouts and main view logic.

use xilem::WidgetView;
use xilem::core::one_of::OneOf4;
use xilem::masonry::properties::types::{AsUnit, CrossAxisAlignment, MainAxisAlignment};
use xilem::style::Style as _;
use xilem::view::{FlexExt as _, FlexSpacer, button, flex_col, flex_row, label, sized_box};

use crate::AppState;
use crate::actions::{Action, Page};
use crate::components::{mode_selector, stat_card, status_bar};
use crate::theme::Theme;

/// Main app view with sidebar and content.
pub fn app_view(state: &mut AppState) -> impl WidgetView<AppState, Action> + use<> {
	let theme = state.theme();

	flex_row((
		sidebar(state, &theme),
		sized_box(main_content(state, &theme)).flex(1.0),
	))
	.background_color(theme.bg_primary)
}

fn sidebar(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	let is_active = state.is_active;
	let theme_mode = state.theme_mode;

	sized_box(
		flex_col((
			// Logo
			flex_row((
				sized_box(label("🛡️").text_size(24.0))
					.padding(10.0)
					.background_color(theme.accent_emerald)
					.corner_radius(8.0),
				label("LobbyGuard")
					.text_size(22.0)
					.color(theme.text_primary),
			))
			.gap(16.px())
			.padding(32.0),
			// Navigation
			flex_col((
				nav_button("Dashboard", Page::Dashboard, state, theme),
				nav_button("Connections", Page::Connections, state, theme),
				nav_button("Whitelist", Page::Whitelist, state, theme),
				nav_button("Settings", Page::Settings, state, theme),
			))
			.gap(8.0.px())
			.padding(16.0),
			// Spacer
			FlexSpacer::Flex(1.0),
			// Status bar
			status_bar(is_active, theme, theme_mode),
		))
		.background_color(theme.bg_secondary)
		.border(theme.border_primary, 1.0),
	)
	.width(220.px())
}

fn nav_button(
	text: &'static str, page: Page, state: &AppState, theme: &Theme,
) -> impl WidgetView<AppState, Action> + use<> {
	let is_active = state.page == page;
	let fg = if is_active {
		theme.accent_emerald_light
	} else {
		theme.text_muted
	};
	let bg = if is_active {
		theme.bg_tertiary
	} else {
		xilem::Color::TRANSPARENT
	};

	sized_box(
		button(
			sized_box(label(text).text_size(16.0).color(fg)).padding(12.0),
			move |_: &mut AppState| Action::Navigate(page),
		)
		.background_color(bg)
		.corner_radius(8.0),
	)
}

fn main_content(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	let content = match state.page {
		Page::Dashboard => OneOf4::A(dashboard_page(state, theme)),
		Page::Connections => OneOf4::B(connections_page(state, theme)),
		Page::Whitelist => OneOf4::C(whitelist_page(state, theme)),
		Page::Settings => OneOf4::D(settings_page(state, theme)),
	};

	flex_col(content)
		.cross_axis_alignment(CrossAxisAlignment::Start)
		.main_axis_alignment(MainAxisAlignment::Start)
		.padding(20.0)
		.background_color(theme.bg_primary)
}

// =============================================================================
// Dashboard Page
// =============================================================================

fn dashboard_page(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	let blocked = state.blocked_count;
	let flows = state.flow_count;
	let processes = state.process_count;
	let uptime = state.get_uptime();

	flex_col((
		// Header area - stacked for responsiveness
		flex_col((
			label("Session Manager")
				.text_size(24.0)
				.color(theme.text_primary),
			flex_row((
				label("Mode:").text_size(12.0).color(theme.text_muted),
				label(state.session_mode.name())
					.text_size(12.0)
					.color(theme.accent_emerald_light),
			))
			.gap(4.px()),
		))
		.gap(8.px()),
		// Stats row - still flex_row but more compact
		flex_row((
			sized_box(stat_card(
				"BLOCKED",
				&format!("{}", blocked),
				theme.accent_emerald_light,
				theme,
			))
			.flex(1.0),
			sized_box(stat_card(
				"UPTIME",
				&format_time(uptime),
				theme.accent_cyan,
				theme,
			))
			.flex(1.0),
			sized_box(stat_card(
				"FLOWS",
				&format!("{}", flows),
				theme.accent_amber,
				theme,
			))
			.flex(1.0),
			sized_box(stat_card(
				"PROCESSES",
				&format!("{}", processes),
				theme.text_secondary,
				theme,
			))
			.flex(1.0),
		))
		.gap(8.px()),
		// Network activity panel
		sized_box(
			flex_col((
				flex_row((
					label("NETWORK ACTIVITY")
						.text_size(12.0)
						.color(theme.text_muted),
					FlexSpacer::Flex(1.0),
					sized_box(
						label(if state.is_active {
							"● Engine Running"
						} else {
							"○ Engine Stopped"
						})
						.text_size(10.0)
						.color(if state.is_active {
							theme.success
						} else {
							theme.text_muted
						}),
					)
					.padding(4.0)
					.background_color(theme.bg_tertiary)
					.corner_radius(4.0),
				)),
				// Activity bars (visualization placeholder)
				flex_row((
					activity_bar(40.0, theme),
					activity_bar(60.0, theme),
					activity_bar(30.0, theme),
					activity_bar(80.0, theme),
					activity_bar(50.0, theme),
					activity_bar(90.0, theme),
					activity_bar(20.0, theme),
					activity_bar(70.0, theme),
					activity_bar(45.0, theme),
					activity_bar(65.0, theme),
				))
				.gap(4.px())
				.cross_axis_alignment(CrossAxisAlignment::End),
			))
			.gap(16.px()),
		)
		.height(200.px())
		.padding(24.0)
		.background_color(theme.bg_secondary)
		.border(theme.border_primary, 1.0)
		.corner_radius(16.0),
		// Mode selector (priority #1 - must be workable)
		label("SELECT SESSION MODE")
			.text_size(12.0)
			.color(theme.text_muted),
		mode_selector(state.session_mode, theme),
	))
	.gap(32.px())
}

fn activity_bar(height_pct: f64, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	sized_box(label(""))
		.width(20.px())
		.height((height_pct as f32).px())
		.background_color(theme.accent_emerald.with_alpha(0.6))
		.corner_radius(2.0)
}

// =============================================================================
// Connections Page
// =============================================================================

fn connections_page(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	let flow_count = state.flow_count;

	flex_col((
		// Header
		sized_box(
			flex_col((
				label("Active Connections")
					.text_size(20.0)
					.color(theme.text_primary),
				label(format!(
					"Monitoring {} active GTA network flows",
					flow_count
				))
				.text_size(14.0)
				.color(theme.text_muted),
			))
			.gap(8.px()),
		)
		.padding(24.0)
		.background_color(theme.bg_secondary)
		.border(theme.border_primary, 1.0)
		.corner_radius(16.0),
		// Flow list placeholder
		sized_box(
			flex_col((
				label("Flow monitoring visualization coming soon...").color(theme.text_muted),
				label("Real-time flow data is being collected in the background.")
					.text_size(12.0)
					.color(theme.text_muted),
			))
			.gap(8.px())
			.main_axis_alignment(MainAxisAlignment::Center)
			.cross_axis_alignment(CrossAxisAlignment::Center),
		)
		.height(300.px())
		.padding(24.0)
		.background_color(theme.bg_secondary.with_alpha(0.3))
		.border(theme.border_primary, 1.0)
		.corner_radius(8.0),
	))
	.gap(24.px())
}

// =============================================================================
// Whitelist Page
// =============================================================================

fn whitelist_page(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	let whitelist = state.get_whitelist();
	let mut list = Vec::with_capacity(whitelist.len() + 2);

	// Header
	list.push(
		sized_box(
			flex_col((
				label("Trusted IP Whitelist")
					.text_size(20.0)
					.color(theme.text_primary),
				label("Players with these IP addresses will bypass Solo and Locked session rules.")
					.text_size(14.0)
					.color(theme.text_muted),
			))
			.gap(8.px()),
		)
		.padding(24.0)
		.background_color(theme.bg_secondary)
		.border(theme.border_primary, 1.0)
		.corner_radius(16.0)
		.boxed(),
	);

	// Entries
	for entry in whitelist {
		let id = entry.id;
		list.push(
			sized_box(
				flex_row((
					flex_row((
						sized_box(label("👤"))
							.width(40.px())
							.height(40.px())
							.corner_radius(20.0)
							.padding(8.0)
							.background_color(theme.bg_tertiary),
						flex_col((
							label(&*entry.name)
								.text_size(16.0)
								.color(theme.text_secondary),
							label(&*entry.ip).text_size(12.0).color(theme.text_muted),
						)),
					))
					.gap(16.px()),
					FlexSpacer::Flex(1.0),
					button(
						sized_box(label("🗑️")).padding(8.0),
						move |_: &mut AppState| Action::RemoveWhitelist(id),
					)
					.background_color(xilem::Color::TRANSPARENT)
					.corner_radius(8.0),
				))
				.main_axis_alignment(MainAxisAlignment::SpaceBetween),
			)
			.padding(16.0)
			.background_color(theme.bg_secondary.with_alpha(0.5))
			.border(theme.border_primary, 1.0)
			.corner_radius(8.0)
			.boxed(),
		);
	}

	// Add button
	list.push(
		sized_box(
			button(
				flex_row((
					label("+").text_size(18.0).color(theme.text_primary),
					label("Add Test Entry").color(theme.text_primary),
				))
				.gap(8.px())
				.padding(12.0),
				|_: &mut AppState| Action::AddWhitelist {
					name: "Test User".to_string(),
					ip: "127.0.0.1".to_string(),
				},
			)
			.background_color(theme.accent_emerald)
			.corner_radius(8.0),
		)
		.boxed(),
	);

	flex_col(list).gap(8.px())
}

// =============================================================================
// Settings Page
// =============================================================================

fn settings_page(state: &AppState, theme: &Theme) -> impl WidgetView<AppState, Action> + use<> {
	flex_col((
		sized_box(
			flex_col((
				label("Settings").text_size(20.0).color(theme.text_primary),
				label("Configure LobbyGuard behavior")
					.text_size(14.0)
					.color(theme.text_muted),
			))
			.gap(8.px()),
		)
		.padding(24.0)
		.background_color(theme.bg_secondary)
		.border(theme.border_primary, 1.0)
		.corner_radius(16.0),
		// Theme section
		sized_box(
			flex_col((
				label("Appearance")
					.text_size(16.0)
					.color(theme.text_secondary),
				flex_row((
					label("Theme:").color(theme.text_muted),
					button(
						label(match state.theme_mode {
							crate::ThemeMode::Dark => "🌙 Dark",
							crate::ThemeMode::Light => "☀️ Light",
						})
						.text_size(14.0)
						.color(theme.text_primary),
						|_: &mut AppState| Action::ToggleTheme,
					)
					.background_color(theme.bg_tertiary)
					.corner_radius(4.0),
				))
				.gap(8.px()),
			))
			.gap(12.px()),
		)
		.padding(24.0)
		.background_color(theme.bg_secondary.with_alpha(0.5))
		.border(theme.border_primary, 1.0)
		.corner_radius(8.0),
		// Info section
		sized_box(
			flex_col((
				label("About").text_size(16.0).color(theme.text_secondary),
				label("LobbyGuard v2.5.0")
					.text_size(12.0)
					.color(theme.text_muted),
				label("Network session manager for GTA Online")
					.text_size(12.0)
					.color(theme.text_muted),
			))
			.gap(8.px()),
		)
		.padding(24.0)
		.background_color(theme.bg_secondary.with_alpha(0.5))
		.border(theme.border_primary, 1.0)
		.corner_radius(8.0),
	))
	.gap(16.px())
}

// =============================================================================
// Helpers
// =============================================================================

fn format_time(secs: u64) -> String {
	let mins = secs / 60;
	let s = secs % 60;
	format!("{:02}:{:02}", mins, s)
}
