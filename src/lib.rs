//! # mongo-derive
//!
//! `mongo-derive` is a procedural macro crate that simplifies working with MongoDB
//! in Rust applications. It generates update builders for your structs that make
//! it easy to create MongoDB update operations while maintaining type safety.
//!
//! ## Usage examples
//!
//! ### Basic usage with the `MongoOperations` derive macro:
//!
//! ```rust
//! use mongo_derive::MongoOperations;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, MongoOperations)]
//! struct User {
//!     #[mongo_ops(set)]
//!     name: String,
//!     
//!     #[mongo_ops(set, push, pull)]
//!     tags: Vec<String>,
//!     
//!     #[mongo_ops(none)]
//!     password_hash: String, // Excluded from update builder
//! }
//!
//! # fn main() -> Result<(), mongodb::error::Error> {
//! // Create an update document
//! let update = User::update_builder()
//!     .set_name("John Doe".to_string())
//!     .push_tags("rust".to_string())
//!     .build()?;
//!     
//! // Use with MongoDB driver
//! // collection.update_one(query, update, None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Working with nested fields:
//!
//! ```rust
//! use mongo_derive::{MongoOperations, mongo_nested_fields};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, Clone, MongoOperations)]
//! struct Address {
//!     #[mongo_ops(set)]
//!     city: String,
//!     
//!     #[mongo_ops(set)]
//!     street: String,
//! }
//!
//! #[mongo_nested_fields(address: "Address", settings: "UserSettings")]
//! #[derive(Serialize, Deserialize,  Clone, MongoOperations)]
//! struct User {
//!     #[mongo_ops(set)]
//!     name: String,
//!     
//!     address: Address,
//! }
//!
//! #[derive(Serialize, Deserialize,  Clone, MongoOperations)]
//! struct UserSettings {
//!     #[mongo_ops(set)]
//!     theme: String,
//! }
//!
//! # fn main() -> Result<(), mongodb::error::Error> {
//! // Update nested fields
//! let update = User::update_builder()
//!     .with_address(|builder| {
//!         builder.set_city("New York".to_string())
//!     })
//!     .address("zipcode", "10001".to_string())? // Direct path access
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::Parse, parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Fields,
    GenericArgument, Ident, LitStr, PathArguments, Token, Type,
};

/// Represents MongoDB operations that can be applied to a field.
/// Used to parse the `#[mongo_ops(...)]` attribute.
struct MongoOps {
    operations: Vec<String>,
}

impl Parse for MongoOps {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let operations = Punctuated::<Ident, Token![,]>::parse_terminated(input)?
            .into_iter()
            .map(|ident| ident.to_string())
            .collect();
        Ok(MongoOps { operations })
    }
}

/// Arguments for the `mongo_nested_fields` attribute macro.
/// Parses a list of field:type pairs.
struct NestedFieldsArgs {
    pairs: Vec<(String, String)>,
}

impl Parse for NestedFieldsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut pairs = Vec::new();

        // Parse comma-separated list of field:type
        let fields_meta = Punctuated::<FieldTypePair, Token![,]>::parse_terminated(input)?;

        for field_type in fields_meta {
            pairs.push((field_type.field_name, field_type.type_name));
        }

        Ok(NestedFieldsArgs { pairs })
    }
}

/// Represents a field:type pair for nested field declarations.
struct FieldTypePair {
    field_name: String,
    type_name: String,
}

impl Parse for FieldTypePair {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let field_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let type_name: LitStr = input.parse()?;

        Ok(FieldTypePair {
            field_name: field_name.to_string(),
            type_name: type_name.value(),
        })
    }
}

/// Returns the inner type if the type is a Vec<T>.
/// Used to support operations on array fields.
fn get_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return Some(inner_type);
                    }
                }
            }
        }
    }
    None
}

/// A derive macro that generates an update builder for a struct.
///
/// The update builder provides methods for creating MongoDB update operations
/// based on the struct's fields and their annotations.
///
/// # Supported Operations
///
/// - `set`: Generate methods for setting field values (default if no operations specified)
/// - `push`: Generate methods for pushing to array fields (Vec types only)
/// - `pull`: Generate methods for pulling from array fields (Vec types only)
/// - `none`: Exclude the field from the update builder
///
/// # Example
///
/// ```rust
/// use mongo_derive::MongoOperations;
///
/// #[derive(MongoOperations)]
/// struct User {
///     #[mongo_ops(set)]
///     name: String,
///     
///     #[mongo_ops(set, push)]
///     tags: Vec<String>,
/// }
/// ```
#[proc_macro_derive(MongoOperations, attributes(mongo_ops))]
pub fn derive_mongo_update_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = format_ident!("{}UpdateBuilder", name);

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let mut builder_methods = Vec::new();
    let mut builder_fields = Vec::new();
    let mut set_conversions = Vec::new();
    let mut push_conversions = Vec::new();
    let mut pull_conversions = Vec::new();

    // Process all fields
    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        let mut ops = vec![];
        for attr in &field.attrs {
            if attr.path().is_ident("mongo_ops") {
                if let Ok(mongo_ops) = attr.parse_args::<MongoOps>() {
                    ops = mongo_ops.operations;
                }
            }
        }

        if ops.contains(&"none".to_string()) {
            continue;
        }

        let field_name_str = field_name.to_string();

        // Handle push operations for Vec types
        if ops.contains(&"push".to_string()) {
            if let Some(inner_type) = get_vec_inner_type(field_type) {
                let field_storage = format_ident!("push_{}", field_name);
                builder_fields.push(quote! {
                    #field_storage: Option<#inner_type>
                });

                let method_name = format_ident!("push_{}", field_name);
                builder_methods.push(quote! {
                    pub fn #method_name(mut self, value: #inner_type) -> Self {
                        self.#field_storage = Some(value);
                        self
                    }
                });

                push_conversions.push(quote! {
                    if let Some(value) = &self.#field_storage {
                        push_doc.insert(#field_name_str, doc! {
                            "$each": [bson::to_bson(value)?]
                        });
                    }
                });
            }
        }

        // Handle pull operations for Vec types
        if ops.contains(&"pull".to_string()) {
            if let Some(inner_type) = get_vec_inner_type(field_type) {
                let field_storage = format_ident!("pull_{}", field_name);
                builder_fields.push(quote! {
                    #field_storage: Option<#inner_type>
                });

                let method_name = format_ident!("pull_{}", field_name);
                builder_methods.push(quote! {
                    pub fn #method_name(mut self, value: #inner_type) -> Self {
                        self.#field_storage = Some(value);
                        self
                    }
                });

                pull_conversions.push(quote! {
                    if let Some(value) = &self.#field_storage {
                        pull_doc.insert(#field_name_str, doc! {
                            "$in": [bson::to_bson(value)?]
                        });
                    }
                });
            }
        }

        // Handle set operations
        if ops.contains(&"set".to_string()) || ops.is_empty() {
            // Generate set methods for all types, including Vec
            let field_storage = format_ident!("set_{}", field_name);
            builder_fields.push(quote! {
                #field_storage: Option<#field_type>
            });

            let method_name = format_ident!("set_{}", field_name);
            builder_methods.push(quote! {
                pub fn #method_name(mut self, value: #field_type) -> Self {
                    self.#field_storage = Some(value);
                    self
                }
            });

            set_conversions.push(quote! {
                if let Some(value) = &self.#field_storage {
                    set_doc.insert(#field_name_str, bson::to_bson(value)?);
                }
            });
        }
    }

    // Add field for direct path updates
    builder_fields.push(quote! {
        path_updates: std::collections::HashMap<String, bson::Bson>
    });

    // Add direct path updates to set document
    set_conversions.push(quote! {
        for (path, value) in &self.path_updates {
            set_doc.insert(path, value.clone());
        }
    });

    // Generate the UpdateBuilder struct
    let expanded = quote! {
        /// The update builder for the struct, generated by the `MongoOperations` derive macro.
        ///
        /// This struct provides methods for creating MongoDB update operations based on the
        /// struct's fields and their annotations.
        #[derive(Default, Clone)]
        pub struct #builder_name {
            #(#builder_fields,)*
        }

        impl #name {
            /// Creates a new update builder for this struct.
            pub fn update_builder() -> #builder_name {
                #builder_name {
                    path_updates: std::collections::HashMap::new(),
                    ..Default::default()
                }
            }
        }

        impl #builder_name {
            #(#builder_methods)*

            /// Generic method for updating any field by path.
            ///
            /// This method allows you to set fields that might not be directly accessible
            /// through the generated methods, such as nested fields or fields with special characters.
            ///
            /// # Arguments
            ///
            /// * `field_path` - The dot notation path to the field
            /// * `value` - The value to set for the field
            ///
            /// # Returns
            ///
            /// Result containing the builder instance or a MongoDB error
            pub fn set_field<T: serde::Serialize>(
                mut self,
                field_path: &str,
                value: T
            ) -> Result<Self, mongodb::error::Error> {
                self.path_updates.insert(field_path.to_string(), bson::to_bson(&value)?);
                Ok(self)
            }

            /// Builds the MongoDB update document based on the configured operations.
            ///
            /// # Returns
            ///
            /// Result containing the update document or a MongoDB error
            pub fn build(self) -> Result<bson::Document, mongodb::error::Error> {
                use bson::{doc, Document};
                let mut update = Document::new();
                let mut set_doc = Document::new();
                let mut push_doc = Document::new();
                let mut pull_doc = Document::new();

                #(#set_conversions)*
                #(#push_conversions)*
                #(#pull_conversions)*

                if !set_doc.is_empty() {
                    update.insert("$set", set_doc);
                }
                if !push_doc.is_empty() {
                    update.insert("$push", push_doc);
                }
                if !pull_doc.is_empty() {
                    update.insert("$pull", pull_doc);
                }

                Ok(update)
            }
        }
    };

    TokenStream::from(expanded)
}

/// An attribute macro that generates methods for working with nested fields.
///
/// This macro allows you to easily update nested documents in MongoDB by
/// generating helper methods for your update builder.
///
/// # Arguments
///
/// A comma-separated list of `field: "Type"` pairs, where:
/// - `field` is the name of the nested field in the parent struct
/// - `"Type"` is the type of the nested field (must implement `MongoOperations`)
///
/// # Example
///
/// ```rust
/// use mongo_derive::{MongoOperations, mongo_nested_fields};
/// use serde::Serialize;
///
/// #[derive(Serialize, Clone, MongoOperations)]
/// struct Address {
///     #[mongo_ops(set)]
///     city: String,
/// }
///
/// #[mongo_nested_fields(address: "Address")]
/// #[derive(Serialize, MongoOperations)]
/// struct User {
///     #[mongo_ops(set)]
///     name: String,
///     
///     address: Address,
/// }
/// ```
#[proc_macro_attribute]
pub fn mongo_nested_fields(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let parent_name = &input.ident;
    let builder_name = format_ident!("{}UpdateBuilder", parent_name);

    // Parse nested field declarations
    let nested_fields = parse_macro_input!(args as NestedFieldsArgs);
    let mut nested_methods = Vec::new();

    for (field_name, type_name) in nested_fields.pairs {
        let field_name_ident = format_ident!("{}", field_name);
        let type_ident = format_ident!("{}", type_name);
        let nested_builder = format_ident!("{}UpdateBuilder", type_name);

        // Generate method to work with the nested builder
        let with_method_name = format_ident!("with_{}", field_name);
        nested_methods.push(quote! {
            impl #builder_name {
                /// Method to work with a nested update builder.
                ///
                /// This method allows you to use the update builder of a nested field
                /// to create updates for nested documents.
                ///
                /// # Arguments
                ///
                /// * `f` - A function that configures the nested builder
                ///
                /// # Returns
                ///
                /// The parent builder instance
                pub fn #with_method_name<F>(mut self, f: F) -> Self
                where
                    F: FnOnce(#nested_builder) -> #nested_builder,
                {
                    let builder = #type_ident::update_builder();
                    let updated_builder = f(builder);

                    // Clone the builder and call build to get the document
                    if let Ok(doc) = updated_builder.clone().build() {
                        // Insert each field from the nested document with the correct path
                        for (key, value) in doc.iter() {
                            if key == "$set" {
                                if let bson::Bson::Document(set_doc) = value {
                                    for (nested_key, nested_value) in set_doc.iter() {
                                        let path = format!("{}.{}", #field_name, nested_key);
                                        self.path_updates.insert(path, nested_value.clone());
                                    }
                                }
                            }
                        }
                    }
                    self
                }

                /// Direct access to update a nested field by path.
                ///
                /// # Arguments
                ///
                /// * `nested_field` - The field name within the nested document
                /// * `value` - The value to set for the nested field
                ///
                /// # Returns
                ///
                /// Result containing the parent builder instance or a MongoDB error
                pub fn #field_name_ident<T: serde::Serialize>(
                    mut self,
                    nested_field: &str,
                    value: T
                ) -> Result<Self, mongodb::error::Error> {
                    let path = format!("{}.{}", #field_name, nested_field);
                    self.path_updates.insert(path, bson::to_bson(&value)?);
                    Ok(self)
                }
            }
        });
    }

    // Combine the input with the new methods
    let result = quote! {
        #input

        #(#nested_methods)*
    };

    TokenStream::from(result)
}
