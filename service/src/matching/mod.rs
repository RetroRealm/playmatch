//! Name-matching building blocks and match-writing flows. `name_parse` splits
//! a DAT file name into a base title and tags, `util` normalizes titles, and
//! `scoring` gates and ranks candidates; the provider matchers compose these
//! three on every rung. `clone` and `content_anchor` run as post-import
//! passes from the ingestion pipeline, while `manual` applies operator
//! matches and `suggestions` stores proposed matches that approval replays
//! through `manual`.

pub mod clone;
pub mod content_anchor;
pub mod manual;
pub(crate) mod name_parse;
pub(crate) mod scoring;
pub mod suggestions;
pub(crate) mod util;
