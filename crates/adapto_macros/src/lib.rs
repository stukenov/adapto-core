//! # adapto_macros
//!
//! Proc-macro crate for the Adapto framework.
//!
//! Provides `#[derive(Resource)]` which generates typed wrappers around
//! `adapto_store` operations — turning a plain struct into a full
//! document-backed resource with CRUD, indexing, and field introspection.
//!
//! ```ignore
//! #[derive(Resource, Serialize, Deserialize)]
//! #[resource(collection = "customers")]
//! pub struct Customer {
//!     #[field(required, unique, format = "email")]
//!     pub email: String,
//!
//!     #[field(required, max_length = 120)]
//!     pub name: String,
//! }
//! ```

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod resource;

/// Derive macro that generates typed `adapto_store` operations for a struct.
///
/// # Container attributes
///
/// - `#[resource(collection = "name")]` — **required**. The store collection name.
///
/// # Field attributes
///
/// All field attributes are optional. If `#[field(...)]` is absent the field
/// is treated as a plain data field with no special handling.
///
/// - `required` — marks the field as required in form schemas.
/// - `unique` — creates a unique index on this field.
/// - `format = "..."` — validation format hint (e.g. `"email"`).
/// - `max_length = N` — maximum string length.
/// - `default = "..."` — default value for form schemas.
/// - `one_of = ["a", "b"]` — allowed values; also creates a non-unique index.
#[proc_macro_derive(Resource, attributes(resource, field))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    resource::expand(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
