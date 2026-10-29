//! Compose typed report surfaces from parsed Markdown and source records.
use super::*;

mod evidence;
mod field;
mod learning;
mod visuals;

pub(crate) use evidence::*;
pub(crate) use field::*;
pub(crate) use learning::*;
pub(crate) use visuals::*;
