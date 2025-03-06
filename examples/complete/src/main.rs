use bson::doc;
use mongo_derive::{mongo_nested_fields, MongoOperations};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};

// Define models with MongoDB operations

#[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
struct Address {
    #[mongo_ops(set)]
    street: String,

    #[mongo_ops(set)]
    city: String,

    #[mongo_ops(set)]
    country: String,

    #[mongo_ops(set)]
    zip_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
struct UserSettings {
    #[mongo_ops(set)]
    theme: String,

    #[mongo_ops(set)]
    notifications_enabled: bool,

    #[mongo_ops(set)]
    language: String,
}

#[mongo_nested_fields(address: "Address", settings: "UserSettings")]
#[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
struct User {
    #[mongo_ops(set)]
    name: String,

    #[mongo_ops(set)]
    email: String,

    #[mongo_ops(set, push, pull)]
    tags: Vec<String>,

    #[mongo_ops(none)]
    password_hash: String, // Excluded from update builder

    address: Address,

    settings: UserSettings,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MongoDB
    let client = Client::with_uri_str("mongodb://localhost:27017").await?;
    let db = client.database("mongo_derive_example");
    let users: Collection<User> = db.collection("users");

    // Create a user document
    let user = User {
        name: "Jane Doe".to_string(),
        email: "jane@example.com".to_string(),
        tags: vec!["developer".to_string(), "rust".to_string()],
        password_hash: "hashed_password".to_string(),
        address: Address {
            street: "123 Main St".to_string(),
            city: "New York".to_string(),
            country: "USA".to_string(),
            zip_code: "10001".to_string(),
        },
        settings: UserSettings {
            theme: "dark".to_string(),
            notifications_enabled: true,
            language: "en".to_string(),
        },
    };

    // Insert the user
    let user_id = users.insert_one(user, None).await?.inserted_id;
    println!("Inserted user with ID: {}", user_id);

    // Example 1: Basic field updates
    let update1 = User::update_builder()
        .set_name("Jane Smith".to_string())
        .set_email("jane.smith@example.com".to_string())
        .build()?;

    println!("Example 1 - Basic updates:");
    println!("{:#?}", update1);

    // Example 2: Array operations
    let update2 = User::update_builder()
        .push_tags("mongodb".to_string())
        .pull_tags("developer".to_string())
        .build()?;

    println!("\nExample 2 - Array operations:");
    println!("{:#?}", update2);

    // Example 3: Nested document updates using with_* method
    let update3 = User::update_builder()
        .with_address(|builder| {
            builder
                .set_city("San Francisco".to_string())
                .set_zip_code("94105".to_string())
        })
        .with_settings(|builder| {
            builder
                .set_theme("light".to_string())
                .set_notifications_enabled(false)
        })
        .build()?;

    println!("\nExample 3 - Nested document updates:");
    println!("{:#?}", update3);

    // Example 4: Direct path updates
    let update4 = User::update_builder()
        .address("zip_code", "94106".to_string())?
        .settings("language", "fr".to_string())?
        .set_field("custom_field", "custom value".to_string())?
        .build()?;

    println!("\nExample 4 - Direct path updates:");
    println!("{:#?}", update4);

    // Apply the updates to MongoDB
    users
        .update_one(doc! { "_id": user_id }, update1, None)
        .await?;

    println!("\nUpdates applied successfully!");

    Ok(())
}
