//! Canon, references, versification and the `.grtb` module format.

pub mod assemble;
pub mod canon;
pub mod catalog;
pub mod integrity;
pub mod module;
pub mod refs;
pub mod source;
pub mod sword;
pub mod text;
pub mod usfm;
pub mod userdata;
pub mod versification;
pub mod votd;

pub use canon::{book, book_by_osis, books, Book};
pub use refs::Ref;
pub use versification::Scheme;
