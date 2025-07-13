#[cfg(test)]
mod integration_tests {
    use crate::test_utils::test_utils::setup_test_app;
    use crate::handlers::accounts::{CreateAccountRequest, UpdateAccountRequest};
    use crate::schemas::ApiResponse;
    use axum_test::TestServer;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_health_check() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Send GET request to health endpoint
        let response = server.get("/health").await;

        // Verify response
        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create account request
        let create_request = CreateAccountRequest {
            name: "Test Account".to_string(),
            description: Some("Test account description".to_string()),
            currency_code: "USD".to_string(),
            owner_id: 1,
            include_in_statistics: Some(true),
            ledger_name: Some("test_ledger".to_string()),
        };

        // Send POST request to create account
        let response = server
            .post("/api/v1/accounts")
            .json(&create_request)
            .await;

        // Verify response
        if response.status_code() != StatusCode::CREATED {
            let error_body = response.text();
            println!("Error response: {}", error_body);
            panic!("Expected 201 Created, got {}", response.status_code());
        }
        let body: ApiResponse<serde_json::Value> = response.json();
        assert!(body.success);
        assert_eq!(body.message, "Account created successfully");

        // Verify account data
        let account_data = &body.data;
        assert_eq!(account_data["name"], "Test Account");
        assert_eq!(account_data["description"], "Test account description");
        assert_eq!(account_data["currency_code"], "USD");
        assert_eq!(account_data["owner_id"], 1);
        assert_eq!(account_data["include_in_statistics"], true);
        assert_eq!(account_data["ledger_name"], "test_ledger");
        assert!(account_data["id"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_get_accounts() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // First create an account
        let create_request = CreateAccountRequest {
            name: "Test Account".to_string(),
            description: Some("Test description".to_string()),
            currency_code: "EUR".to_string(),
            owner_id: 1,
            include_in_statistics: Some(false),
            ledger_name: None,
        };

        let create_response = server
            .post("/api/v1/accounts")
            .json(&create_request)
            .await;
        create_response.assert_status(StatusCode::CREATED);

        // Get all accounts
        let response = server.get("/api/v1/accounts").await;

        // Verify response
        response.assert_status(StatusCode::OK);
        let body: ApiResponse<Vec<serde_json::Value>> = response.json();
        assert!(body.success);
        assert_eq!(body.message, "Accounts retrieved successfully");
        assert_eq!(body.data.len(), 1);

        // Verify account data
        let account = &body.data[0];
        assert_eq!(account["name"], "Test Account");
        assert_eq!(account["currency_code"], "EUR");
        assert_eq!(account["include_in_statistics"], false);
    }

    #[tokio::test]
    async fn test_get_account_by_id() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create an account first
        let create_request = CreateAccountRequest {
            name: "Specific Account".to_string(),
            description: None,
            currency_code: "GBP".to_string(),
            owner_id: 2,
            include_in_statistics: Some(true),
            ledger_name: Some("specific_ledger".to_string()),
        };

        let create_response = server
            .post("/api/v1/accounts")
            .json(&create_request)
            .await;
        create_response.assert_status(StatusCode::CREATED);

        let create_body: ApiResponse<serde_json::Value> = create_response.json();
        let account_id = create_body.data["id"].as_i64().unwrap();

        // Get the specific account
        let response = server
            .get(&format!("/api/v1/accounts/{}", account_id))
            .await;

        // Verify response
        response.assert_status(StatusCode::OK);
        let body: ApiResponse<serde_json::Value> = response.json();
        assert!(body.success);
        assert_eq!(body.message, "Account retrieved successfully");

        // Verify account data
        let account = &body.data;
        assert_eq!(account["id"], account_id);
        assert_eq!(account["name"], "Specific Account");
        assert_eq!(account["currency_code"], "GBP");
        assert_eq!(account["owner_id"], 2);
        assert_eq!(account["ledger_name"], "specific_ledger");
    }

    #[tokio::test]
    async fn test_update_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create an account first
        let create_request = CreateAccountRequest {
            name: "Original Account".to_string(),
            description: Some("Original description".to_string()),
            currency_code: "USD".to_string(),
            owner_id: 1,
            include_in_statistics: Some(true),
            ledger_name: None,
        };

        let create_response = server
            .post("/api/v1/accounts")
            .json(&create_request)
            .await;
        create_response.assert_status(StatusCode::CREATED);

        let create_body: ApiResponse<serde_json::Value> = create_response.json();
        let account_id = create_body.data["id"].as_i64().unwrap();

        // Update the account
        let update_request = UpdateAccountRequest {
            name: Some("Updated Account".to_string()),
            description: Some("Updated description".to_string()),
            currency_code: Some("EUR".to_string()),
            include_in_statistics: Some(false),
            ledger_name: Some("updated_ledger".to_string()),
        };

        let response = server
            .put(&format!("/api/v1/accounts/{}", account_id))
            .json(&update_request)
            .await;

        // Verify response
        response.assert_status(StatusCode::OK);
        let body: ApiResponse<serde_json::Value> = response.json();
        assert!(body.success);
        assert_eq!(body.message, "Account updated successfully");

        // Verify updated data
        let account = &body.data;
        assert_eq!(account["id"], account_id);
        assert_eq!(account["name"], "Updated Account");
        assert_eq!(account["description"], "Updated description");
        assert_eq!(account["currency_code"], "EUR");
        assert_eq!(account["include_in_statistics"], false);
        assert_eq!(account["ledger_name"], "updated_ledger");

        // Verify the update persisted by getting the account again
        let get_response = server
            .get(&format!("/api/v1/accounts/{}", account_id))
            .await;
        get_response.assert_status(StatusCode::OK);

        let get_body: ApiResponse<serde_json::Value> = get_response.json();
        let retrieved_account = &get_body.data;
        assert_eq!(retrieved_account["name"], "Updated Account");
        assert_eq!(retrieved_account["currency_code"], "EUR");
        assert_eq!(retrieved_account["include_in_statistics"], false);
    }

    #[tokio::test]
    async fn test_delete_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create an account first
        let create_request = CreateAccountRequest {
            name: "Account to Delete".to_string(),
            description: Some("Will be deleted".to_string()),
            currency_code: "USD".to_string(),
            owner_id: 1,
            include_in_statistics: Some(true),
            ledger_name: None,
        };

        let create_response = server
            .post("/api/v1/accounts")
            .json(&create_request)
            .await;
        create_response.assert_status(StatusCode::CREATED);

        let create_body: ApiResponse<serde_json::Value> = create_response.json();
        let account_id = create_body.data["id"].as_i64().unwrap();

        // Delete the account
        let response = server
            .delete(&format!("/api/v1/accounts/{}", account_id))
            .await;

        // Verify delete response
        response.assert_status(StatusCode::OK);
        let body: ApiResponse<String> = response.json();
        assert!(body.success);
        assert_eq!(body.message, "Account deleted successfully");
        assert_eq!(body.data, format!("Account {} deleted", account_id));

        // Verify the account is actually deleted by trying to get it
        let get_response = server
            .get(&format!("/api/v1/accounts/{}", account_id))
            .await;
        get_response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_nonexistent_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Try to get a non-existent account
        let response = server.get("/api/v1/accounts/999").await;

        // Should return 404
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_nonexistent_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Try to update a non-existent account
        let update_request = UpdateAccountRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            currency_code: None,
            include_in_statistics: None,
            ledger_name: None,
        };

        let response = server
            .put("/api/v1/accounts/999")
            .json(&update_request)
            .await;

        // Should return 404
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_account() {
        // Setup test server
        let app = setup_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Try to delete a non-existent account
        let response = server.delete("/api/v1/accounts/999").await;

        // Should return 404
        response.assert_status(StatusCode::NOT_FOUND);
    }
}
