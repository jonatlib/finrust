#[cfg(test)]
mod tests {
    use crate::schemas::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_schema_generation() {
        // Test that the OpenAPI schema can be generated without errors
        let openapi = ApiDoc::openapi();

        // Verify that the schema contains the expected components
        assert!(openapi.components.is_some());
        let components = openapi.components.as_ref().unwrap();

        // Check that ErrorResponse schema is properly defined
        assert!(components.schemas.contains_key("ErrorResponse"));

        // Check that HealthResponse schema is properly defined
        assert!(components.schemas.contains_key("HealthResponse"));

        // Verify that the schema can be serialized to JSON without errors
        let json_result = serde_json::to_string(&openapi);
        assert!(json_result.is_ok());

        println!("OpenAPI schema generated successfully");
    }

    #[test]
    fn test_error_response_schema_structure() {
        let openapi = ApiDoc::openapi();
        let components = openapi.components.as_ref().unwrap();
        let error_response_schema = components.schemas.get("ErrorResponse").unwrap();

        // Verify ErrorResponse has the expected structure
        if let utoipa::openapi::RefOr::T(utoipa::openapi::schema::Schema::Object(obj)) = error_response_schema {
            let properties = &obj.properties;
            assert!(properties.contains_key("error"));
            assert!(properties.contains_key("code"));
            assert!(properties.contains_key("success"));
        } else {
            panic!("ErrorResponse should be an object schema");
        }
    }

    #[test]
    fn test_health_response_schema_structure() {
        let openapi = ApiDoc::openapi();
        let components = openapi.components.as_ref().unwrap();
        let health_response_schema = components.schemas.get("HealthResponse").unwrap();

        // Verify HealthResponse has the expected structure
        if let utoipa::openapi::RefOr::T(utoipa::openapi::schema::Schema::Object(obj)) = health_response_schema {
            let properties = &obj.properties;
            assert!(properties.contains_key("status"));
            assert!(properties.contains_key("version"));
            assert!(properties.contains_key("database"));
        } else {
            panic!("HealthResponse should be an object schema");
        }
    }

    #[test]
    fn test_openapi_paths_contain_health_endpoint() {
        let openapi = ApiDoc::openapi();

        // Verify that the /health endpoint is properly defined
        assert!(openapi.paths.paths.contains_key("/health"));

        let health_path = openapi.paths.paths.get("/health").unwrap();
        let health_get = health_path.operations.get(&utoipa::openapi::PathItemType::Get);
        assert!(health_get.is_some());

        let health_get_op = health_get.unwrap();

        let responses = &health_get_op.responses;
        // Check that both 200 and 500 responses are defined
        assert!(responses.responses.contains_key("200"));
        assert!(responses.responses.contains_key("500"));
    }

    #[test]
    fn test_all_error_responses_reference_correct_schema() {
        let openapi = ApiDoc::openapi();
        let openapi_json = serde_json::to_string(&openapi).unwrap();

        // Ensure no references to crate.schemas.ErrorResponse exist
        assert!(!openapi_json.contains("crate.schemas.ErrorResponse"));
        assert!(!openapi_json.contains("crate::schemas::ErrorResponse"));

        // Ensure proper ErrorResponse references exist
        assert!(openapi_json.contains("ErrorResponse"));

        println!("All error response references are correctly formatted");
    }
}
