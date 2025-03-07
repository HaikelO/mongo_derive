# mongo_derive

A Rust procedural macro crate that simplifies MongoDB operations by generating update builders for your structs.

## Features

- `MongoOperations` derive macro for generating update builders with typesafe methods
- Support for `$set`, `$push`, and `$pull` MongoDB operations
- Nested field handling with the `mongo_nested_fields` attribute
- Path-based updates for flexibility

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mongo_derive = "0.1.0"
```

## Usage

### Basic Usage

```rust
use mongo_derive::MongoOperations;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, MongoOperations)]
struct User {
    #[mongo_ops(set)]
    name: String,

    #[mongo_ops(set, push, pull)]
    tags: Vec<String>,

    #[mongo_ops(none)]
    password_hash: String, // Excluded from update builder
}

fn main() -> Result<(), mongodb::error::Error> {
    // Create an update document
    let update = User::update_builder()
        .set_name("John Doe".to_string())
        .push_tags("rust".to_string())
        .build()?;

    // Use with MongoDB driver
    // collection.update_one(query, update, None).await?;

    println!("{:?}", update);
    Ok(())
}
```

### Working with Nested Fields

```rust
use mongo_derive::{MongoOperations, mongo_nested_fields};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, MongoOperations)]
struct Address {
    #[mongo_ops(set)]
    city: String,

    #[mongo_ops(set)]
    street: String,
}

#[mongo_nested_fields(address: "Address", settings: "UserSettings")]
#[derive(Serialize, Deserialize, Clone, MongoOperations)]
struct User {
    #[mongo_ops(set)]
    name: String,

    address: Address,
}

#[derive(Serialize, Deserialize, Clone, MongoOperations)]
struct UserSettings {
    #[mongo_ops(set)]
    theme: String,
}

fn main() -> Result<(), mongodb::error::Error> {
    // Update nested fields
    let update = User::update_builder()
        .with_address(|builder| {
            builder.set_city("New York".to_string())
        })
        .address("zipcode", "10001".to_string())? // Direct path access
        .build()?;

    println!("{:?}", update);
    Ok(())
}
```

## How It Works

The crate generates update builder structs that create MongoDB update documents with the proper operators:

- `$set` for replacing field values
- `$push` for adding to arrays
- `$pull` for removing from arrays

The builder pattern ensures type safety while giving you the flexibility of MongoDB's update operators.

## License

This project is licensed under the [MIT License](LICENSE).
