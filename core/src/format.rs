//! Text formatting and ANSI color support.
//!
//! - [`Color`] — 16 standard ANSI colors plus terminal default
//! - [`Modifier`] — bitmask of text modifiers (bold, italic, blink, ...)
//! - [`RichText`] / [`Segment`] — styled text builder with ANSI rendering
//! - [`parse_tags`] — inline `{color}` tag parser for content files
//! - [`conventions`] — color conventions for common MUD display elements

mod color;
pub mod conventions;
pub mod preview;
mod rich_text;
pub mod social;
mod tag;

pub use color::Color;
pub use rich_text::{Modifier, RichText, Segment};
pub use tag::parse_tags;
