//! Primer — presentation primitives for the ecumene.
//!
//! Cells, pages, feeds, typed content, and the imprint mechanism.
//! Every presentation surface in the ecosystem speaks primer.
//!
//! The primer crate provides the types and traits. Matter compilers,
//! ractors, and mediatronics are built on top of these primitives
//! in their respective crates or binaries.

pub mod cell;
pub mod content;
pub mod content_display;
pub mod feed;
pub mod imprint;
pub mod imprints;
pub mod page;

pub use cell::{Cell, CellLink, LinkKind, Role};
pub use content::TypedContent;
pub use feed::{CellNavigator, Feed, NavResult};
pub use imprint::{CompileTrace, Imprint, ImprintRegistry, ImprintTrace};
pub use page::{Page, PageIndex};
