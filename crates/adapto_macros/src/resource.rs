use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, LitStr, Result};

// ---------------------------------------------------------------------------
// Parsed representations
// ---------------------------------------------------------------------------

/// Container-level config from `#[resource(...)]`.
struct ResourceAttr {
    collection: String,
}

/// Per-field config from `#[field(...)]`.
#[derive(Default)]
struct FieldAttr {
    unique: bool,
    one_of: bool,
}

/// A parsed struct field we care about.
struct ResourceField {
    ident: Ident,
    attr: FieldAttr,
}

// ---------------------------------------------------------------------------
// Attribute parsing (syn 2.x API)
// ---------------------------------------------------------------------------

fn parse_resource_attr(input: &DeriveInput) -> Result<ResourceAttr> {
    let mut collection: Option<String> = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("resource") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("collection") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                collection = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("unknown resource attribute"))
            }
        })?;
    }

    match collection {
        Some(c) => Ok(ResourceAttr { collection: c }),
        None => Err(Error::new_spanned(
            &input.ident,
            "Resource derive requires #[resource(collection = \"...\")]",
        )),
    }
}

fn parse_field_attr(field: &syn::Field) -> Result<FieldAttr> {
    let mut attr = FieldAttr::default();

    for a in &field.attrs {
        if !a.path().is_ident("field") {
            continue;
        }
        a.parse_nested_meta(|meta| {
            if meta.path.is_ident("unique") {
                attr.unique = true;
            } else if meta.path.is_ident("one_of") {
                // Consume the value so syn doesn't error on `= [...]`.
                if meta.input.peek(syn::Token![=]) {
                    let _value = meta.value()?;
                    // Parse and discard the bracketed list.
                    let _content;
                    syn::bracketed!(_content in _value);
                    // Drain inner tokens.
                    let _ = _content.parse::<proc_macro2::TokenStream>();
                }
                attr.one_of = true;
            } else if meta.path.is_ident("required")
                || meta.path.is_ident("format")
                || meta.path.is_ident("max_length")
                || meta.path.is_ident("default")
            {
                // Accept silently — consume value if present.
                if meta.input.peek(syn::Token![=]) {
                    let _value = meta.value()?;
                    let _ = _value.parse::<proc_macro2::TokenStream>();
                }
            } else {
                // Unknown attributes are accepted silently for forward compat.
                if meta.input.peek(syn::Token![=]) {
                    let _value = meta.value()?;
                    let _ = _value.parse::<proc_macro2::TokenStream>();
                }
            }
            Ok(())
        })?;
    }

    Ok(attr)
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

pub fn expand(input: DeriveInput) -> Result<TokenStream> {
    let resource_attr = parse_resource_attr(&input)?;
    let collection = &resource_attr.collection;

    let struct_name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => named,
            _ => {
                return Err(Error::new_spanned(
                    struct_name,
                    "Resource can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                struct_name,
                "Resource can only be derived for structs",
            ))
        }
    };

    // Collect field info.
    let mut resource_fields: Vec<ResourceField> = Vec::new();
    for f in &fields.named {
        let ident = f.ident.clone().unwrap();
        let attr = parse_field_attr(f)?;
        resource_fields.push(ResourceField { ident, attr });
    }

    // --- field_names ---------------------------------------------------------
    let field_name_strs: Vec<String> =
        resource_fields.iter().map(|f| f.ident.to_string()).collect();
    let field_name_literals: Vec<&str> =
        field_name_strs.iter().map(String::as_str).collect();

    // --- get_field arms ------------------------------------------------------
    let get_field_arms = resource_fields.iter().map(|f| {
        let ident = &f.ident;
        let name = ident.to_string();
        quote! {
            #name => Some(format!("{}", self.#ident)),
        }
    });

    // --- ensure_indexes body -------------------------------------------------
    let index_stmts: Vec<TokenStream> = resource_fields
        .iter()
        .filter_map(|f| {
            let field_str = f.ident.to_string();
            if f.attr.unique {
                Some(quote! {
                    let _ = col.create_index(#field_str, true);
                })
            } else if f.attr.one_of {
                Some(quote! {
                    let _ = col.create_index(#field_str, false);
                })
            } else {
                None
            }
        })
        .collect();

    // --- generated impl ------------------------------------------------------
    let expanded = quote! {
        impl #struct_name {
            /// The collection name for this resource.
            pub fn collection_name() -> &'static str {
                #collection
            }

            /// Convert from an adapto_store Document to this type.
            pub fn from_document(doc: &adapto_store::Document) -> Option<Self> {
                serde_json::from_value(doc.data.clone()).ok()
            }

            /// Convert this resource to a serde_json::Value for insertion.
            pub fn to_value(&self) -> serde_json::Value {
                serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
            }

            /// Get a typed collection handle.
            pub fn store_collection(store: &adapto_store::AdaptoStore) -> adapto_store::Collection<'_> {
                store.collection(#collection)
            }

            /// Ensure indexes exist for this resource.
            pub fn ensure_indexes(store: &adapto_store::AdaptoStore) {
                let col = store.collection(#collection);
                #(#index_stmts)*
            }

            /// Insert this resource into the store. Returns the document ID.
            pub fn insert_into(
                &self,
                store: &adapto_store::AdaptoStore,
            ) -> Result<String, adapto_store::StoreError> {
                let col = store.collection(#collection);
                col.insert(self.to_value())
            }

            /// Find a resource by document ID.
            pub fn find_by_id(
                store: &adapto_store::AdaptoStore,
                id: &str,
            ) -> Option<(String, Self)> {
                let col = store.collection(#collection);
                col.find_by_id(id).ok().flatten().and_then(|doc| {
                    let id = doc.id.clone();
                    Self::from_document(&doc).map(|r| (id, r))
                })
            }

            /// Find all resources matching a query.
            pub fn find_all(
                store: &adapto_store::AdaptoStore,
                query: adapto_store::Query,
            ) -> Vec<(String, Self)> {
                let col = store.collection(#collection);
                col.find(query)
                    .filter_map(|doc| {
                        let id = doc.id.clone();
                        Self::from_document(&doc).map(|r| (id, r))
                    })
                    .collect()
            }

            /// Count all resources in this collection.
            pub fn count(store: &adapto_store::AdaptoStore) -> u64 {
                store.collection(#collection).count_all()
            }

            /// Delete a resource by document ID.
            pub fn delete(store: &adapto_store::AdaptoStore, id: &str) -> bool {
                store.collection(#collection).delete_by_id(id).unwrap_or(false)
            }

            /// Get field names as a slice (useful for views / table columns).
            pub fn field_names() -> &'static [&'static str] {
                &[#(#field_name_literals),*]
            }

            /// Get a field value by name from this resource.
            pub fn get_field(&self, name: &str) -> Option<String> {
                match name {
                    #(#get_field_arms)*
                    _ => None,
                }
            }

            /// Update this resource in the store by document ID.
            pub fn update_in(
                &self,
                store: &adapto_store::AdaptoStore,
                id: &str,
            ) -> Result<bool, adapto_store::StoreError> {
                let col = store.collection(#collection);
                let val = self.to_value();
                let fields: Vec<(String, serde_json::Value)> = match val {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => vec![],
                };
                col.update_by_id(id, adapto_store::Update::Set(fields))
            }

            /// Find one resource by a field value (requires index for O(1)).
            pub fn find_one_by(
                store: &adapto_store::AdaptoStore,
                field: &str,
                value: impl Into<serde_json::Value>,
            ) -> Option<(String, Self)> {
                let col = store.collection(#collection);
                col.find_one(adapto_store::Query::eq(field, value))
                    .ok()
                    .flatten()
                    .and_then(|doc| {
                        let id = doc.id.clone();
                        Self::from_document(&doc).map(|r| (id, r))
                    })
            }

            /// Delete all resources matching a query. Returns count deleted.
            pub fn delete_where(
                store: &adapto_store::AdaptoStore,
                query: adapto_store::Query,
            ) -> Result<u64, adapto_store::StoreError> {
                let col = store.collection(#collection);
                col.delete(query)
            }

            /// Check if a resource exists by field value.
            pub fn exists(
                store: &adapto_store::AdaptoStore,
                field: &str,
                value: impl Into<serde_json::Value>,
            ) -> bool {
                let col = store.collection(#collection);
                col.find_one(adapto_store::Query::eq(field, value))
                    .ok()
                    .flatten()
                    .is_some()
            }
        }
    };

    Ok(expanded)
}
