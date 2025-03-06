#[cfg(test)]
mod tests {
    use bson::{doc, Bson, Document};
    use mongo_derive::{mongo_nested_fields, MongoOperations};
    use serde::{Deserialize, Serialize};
    // Test Models

    #[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
    struct Address {
        #[mongo_ops(set)]
        street: String,

        #[mongo_ops(set)]
        city: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
    struct Preferences {
        #[mongo_ops(set)]
        theme: String,

        #[mongo_ops(set)]
        language: String,
    }

    #[mongo_nested_fields(address: "Address", preferences: "Preferences")]
    #[derive(Debug, Serialize, Deserialize, Clone, MongoOperations)]
    struct User {
        #[mongo_ops(set)]
        name: String,

        #[mongo_ops(set)]
        email: String,

        #[mongo_ops(set, push, pull)]
        tags: Vec<String>,

        #[mongo_ops(none)]
        password_hash: String,

        address: Address,

        preferences: Preferences,
    }

    // Helper function to extract a document from a specific MongoDB operator
    fn get_operator_doc<'a>(doc: &'a Document, operator: &'a str) -> Option<&'a Document> {
        match doc.get(operator) {
            Some(Bson::Document(operator_doc)) => Some(operator_doc),
            _ => None,
        }
    }

    #[test]
    fn test_basic_set_operations() {
        // Create a simple update
        let update = User::update_builder()
            .set_name("John Doe".to_string())
            .set_email("john@example.com".to_string())
            .build()
            .unwrap();

        // Extract $set document
        let set_doc = get_operator_doc(&update, "$set").expect("$set operator should exist");

        // Verify values
        assert_eq!(set_doc.get("name").unwrap().as_str().unwrap(), "John Doe");
        assert_eq!(
            set_doc.get("email").unwrap().as_str().unwrap(),
            "john@example.com"
        );
    }

    #[test]
    fn test_array_operations() {
        // Create an update with array operations
        let update = User::update_builder()
            .push_tags("mongodb".to_string())
            .pull_tags("rust".to_string())
            .build()
            .unwrap();

        // Verify $push operation
        let push_doc = get_operator_doc(&update, "$push").expect("$push operator should exist");
        let push_tags = push_doc.get("tags").unwrap().as_document().unwrap();
        let each_array = push_tags.get("$each").unwrap().as_array().unwrap();
        assert_eq!(each_array[0].as_str().unwrap(), "mongodb");

        // Verify $pull operation
        let pull_doc = get_operator_doc(&update, "$pull").expect("$pull operator should exist");
        let pull_tags = pull_doc.get("tags").unwrap().as_document().unwrap();
        let in_array = pull_tags.get("$in").unwrap().as_array().unwrap();
        assert_eq!(in_array[0].as_str().unwrap(), "rust");
    }

    #[test]
    fn test_excluded_fields() {
        // Create an update attempting to set a field with mongo_ops(none)
        let update = User::update_builder()
            .set_name("John Doe".to_string())
            .build()
            .unwrap();

        // Extract $set document
        let set_doc = get_operator_doc(&update, "$set").expect("$set operator should exist");

        // Verify the password_hash field doesn't exist in the update
        assert!(set_doc.get("password_hash").is_none());
    }

    #[test]
    fn test_nested_fields() {
        // Create an update with nested fields
        let update = User::update_builder()
            .with_address(|builder| {
                builder
                    .set_city("New York".to_string())
                    .set_street("123 Broadway".to_string())
            })
            .with_preferences(|builder| {
                builder
                    .set_theme("dark".to_string())
                    .set_language("en".to_string())
            })
            .build()
            .unwrap();

        // Extract $set document
        let set_doc = get_operator_doc(&update, "$set").expect("$set operator should exist");

        // Verify nested fields
        assert_eq!(
            set_doc.get("address.city").unwrap().as_str().unwrap(),
            "New York"
        );
        assert_eq!(
            set_doc.get("address.street").unwrap().as_str().unwrap(),
            "123 Broadway"
        );
        assert_eq!(
            set_doc.get("preferences.theme").unwrap().as_str().unwrap(),
            "dark"
        );
        assert_eq!(
            set_doc
                .get("preferences.language")
                .unwrap()
                .as_str()
                .unwrap(),
            "en"
        );
    }

    #[test]
    fn test_direct_path_access() {
        // Create an update with direct path access
        let update = User::update_builder()
            .address("zip_code", "10001")
            .unwrap()
            .preferences("font_size", 14)
            .unwrap()
            .set_field("metadata.created_at", "2025-03-06")
            .unwrap()
            .build()
            .unwrap();

        // Extract $set document
        let set_doc = get_operator_doc(&update, "$set").expect("$set operator should exist");

        // Verify direct path fields
        assert_eq!(
            set_doc.get("address.zip_code").unwrap().as_str().unwrap(),
            "10001"
        );
        assert_eq!(
            set_doc
                .get("preferences.font_size")
                .unwrap()
                .as_i32()
                .unwrap(),
            14
        );
        assert_eq!(
            set_doc
                .get("metadata.created_at")
                .unwrap()
                .as_str()
                .unwrap(),
            "2025-03-06"
        );
    }
}
