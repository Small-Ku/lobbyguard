//! Theme system for LobbyGuard GUI.
//!
//! Provides light and dark theme color palettes.

use xilem::Color;

/// Theme selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
	#[default]
	Dark,
	Light,
}

impl ThemeMode {
	/// Toggle between light and dark.
	#[must_use]
	pub fn toggle(self) -> Self {
		match self {
			ThemeMode::Dark => ThemeMode::Light,
			ThemeMode::Light => ThemeMode::Dark,
		}
	}
}

/// Color palette for UI theming.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
	// Background colors
	pub bg_primary: Color,
	pub bg_secondary: Color,
	pub bg_tertiary: Color,
	pub bg_elevated: Color,

	// Border colors
	pub border_primary: Color,
	pub border_secondary: Color,

	// Text colors
	pub text_primary: Color,
	pub text_secondary: Color,
	pub text_muted: Color,

	// Accent colors (same for both themes)
	pub accent_emerald: Color,
	pub accent_emerald_light: Color,
	pub accent_cyan: Color,
	pub accent_rose: Color,
	pub accent_amber: Color,

	// Semantic colors
	pub success: Color,
	pub warning: Color,
	pub error: Color,
}

impl Theme {
	/// Dark theme (default).
	#[must_use]
	pub fn dark() -> Self {
		Self {
			// Slate palette - dark
			bg_primary: Color::from_rgb8(1, 4, 18),     // darker
			bg_secondary: Color::from_rgb8(10, 18, 38), // slightly lighter navy
			bg_tertiary: Color::from_rgb8(21, 32, 59),  // slate-800
			bg_elevated: Color::from_rgb8(31, 44, 73),  // slate-700

			border_primary: Color::from_rgb8(21, 32, 59), // slate-800
			border_secondary: Color::from_rgb8(31, 44, 73), // slate-700

			text_primary: Color::WHITE,
			text_secondary: Color::from_rgb8(226, 232, 240), // slate-200
			text_muted: Color::from_rgb8(148, 163, 184),     // slate-400

			// Accents
			accent_emerald: Color::from_rgb8(16, 185, 129), // emerald-500
			accent_emerald_light: Color::from_rgb8(52, 211, 153), // emerald-400
			accent_cyan: Color::from_rgb8(6, 182, 212),     // cyan-500
			accent_rose: Color::from_rgb8(244, 63, 94),     // rose-500
			accent_amber: Color::from_rgb8(245, 158, 11),   // amber-500

			// Semantic
			success: Color::from_rgb8(16, 185, 129), // emerald-500
			warning: Color::from_rgb8(245, 158, 11), // amber-500
			error: Color::from_rgb8(244, 63, 94),    // rose-500
		}
	}

	/// Light theme.
	#[must_use]
	pub fn light() -> Self {
		Self {
			// Slate palette - light
			bg_primary: Color::from_rgb8(248, 250, 252), // slate-50
			bg_secondary: Color::from_rgb8(241, 245, 249), // slate-100
			bg_tertiary: Color::from_rgb8(226, 232, 240), // slate-200
			bg_elevated: Color::WHITE,

			border_primary: Color::from_rgb8(203, 213, 225), // slate-300
			border_secondary: Color::from_rgb8(226, 232, 240), // slate-200

			text_primary: Color::from_rgb8(15, 23, 42), // slate-900
			text_secondary: Color::from_rgb8(51, 65, 85), // slate-700
			text_muted: Color::from_rgb8(100, 116, 139), // slate-500

			// Accents (slightly adjusted for light bg)
			accent_emerald: Color::from_rgb8(5, 150, 105), // emerald-600
			accent_emerald_light: Color::from_rgb8(16, 185, 129), // emerald-500
			accent_cyan: Color::from_rgb8(8, 145, 178),    // cyan-600
			accent_rose: Color::from_rgb8(225, 29, 72),    // rose-600
			accent_amber: Color::from_rgb8(217, 119, 6),   // amber-600

			// Semantic
			success: Color::from_rgb8(5, 150, 105), // emerald-600
			warning: Color::from_rgb8(217, 119, 6), // amber-600
			error: Color::from_rgb8(225, 29, 72),   // rose-600
		}
	}

	/// Get theme for the given mode.
	#[must_use]
	pub fn for_mode(mode: ThemeMode) -> Self {
		match mode {
			ThemeMode::Dark => Self::dark(),
			ThemeMode::Light => Self::light(),
		}
	}
}

impl Default for Theme {
	fn default() -> Self { Self::dark() }
}
