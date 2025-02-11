//! This module contains all logic for printing or displaying structured information about the
//! daemon.
//!
//! This includes formatting of task tables, group info, log inspection and log following.
pub mod group;
pub mod helper;
pub mod style;
pub mod table_builder;

use crossterm::style::Color;

// Re-exports
pub use self::style::OutputStyle;

/// Used to style any generic success message from the daemon.
pub fn print_success(_style: &OutputStyle, message: &str) {
    println!("{message}");
}

/// Used to style any generic failure message from the daemon.
pub fn print_error(style: &OutputStyle, message: &str) {
    let styled = style.style_text(message, Some(Color::Red), None);
    eprintln!("{styled}");
}
