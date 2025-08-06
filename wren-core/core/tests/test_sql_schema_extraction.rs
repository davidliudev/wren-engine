use wren_core::*;
use std::ffi::{CStr, CString};

#[test]
fn test_extract_schema_simple_select() {
    let sql = CString::new("SELECT id, name, age FROM users").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true},
                {"name": "age", "type": "integer", "nullable": true},
                {"name": "email", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["success"], true);
    assert_eq!(json["tableName"], "query_result");
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0]["name"], "id");
    assert_eq!(columns[0]["type"], "integer");
    assert_eq!(columns[1]["name"], "name");
    assert_eq!(columns[1]["type"], "varchar");
    assert_eq!(columns[2]["name"], "age");
    assert_eq!(columns[2]["type"], "integer");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_alias() {
    let sql = CString::new("SELECT * FROM (SELECT id, name FROM users) AS u").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("Test result: {}", result_str);
    
    assert_eq!(json["success"], true);
    // Should extract alias "u" from SQL
    assert_eq!(json["tableName"], "u");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_suggested_name() {
    let sql = CString::new("SELECT id, name FROM users").unwrap();
    let dialect = CString::new("postgres").unwrap();
    let suggested_name = CString::new("my_custom_table").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        suggested_name.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["success"], true);
    // Should use suggested name when no alias in SQL
    assert_eq!(json["tableName"], "my_custom_table");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_table_alias() {
    // Test regular table alias: FROM users u
    let sql = CString::new("SELECT u.id, u.name FROM users u").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("Table alias test result: {}", result_str);
    
    assert_eq!(json["success"], true);
    // Should extract table alias "u"
    assert_eq!(json["tableName"], "u");
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0]["name"], "id");
    assert_eq!(columns[1]["name"], "name");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_alias_precedence() {
    // Test that SQL alias takes precedence over suggested name
    let sql = CString::new("SELECT * FROM (SELECT id, name FROM users) AS user_subset").unwrap();
    let dialect = CString::new("postgres").unwrap();
    let suggested_name = CString::new("my_suggested_name").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        suggested_name.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["success"], true);
    // SQL alias should take precedence over suggested name
    assert_eq!(json["tableName"], "user_subset");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_join() {
    // With table aliases in JOIN
    let sql = CString::new("SELECT u.id, u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id").unwrap();
    let dialect = CString::new("postgres").unwrap();
    let suggested_name = CString::new("user_orders").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        },
        {
            "tableName": "orders",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "user_id", "type": "integer", "nullable": true},
                {"name": "total", "type": "decimal", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        suggested_name.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("JOIN test result: {}", result_str);
    
    assert_eq!(json["success"], true);
    assert_eq!(json["tableName"], "user_orders");
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0]["name"], "id");
    assert_eq!(columns[1]["name"], "name");
    assert_eq!(columns[2]["name"], "total");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_aggregation() {
    let sql = CString::new("SELECT user_id, COUNT(*) as order_count, SUM(amount) as total_amount FROM orders GROUP BY user_id").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "orders",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "user_id", "type": "integer", "nullable": true},
                {"name": "amount", "type": "decimal", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("Aggregation test result: {}", result_str);
    
    assert_eq!(json["success"], true);
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0]["name"], "user_id");
    assert_eq!(columns[1]["name"], "order_count");
    assert_eq!(columns[2]["name"], "total_amount");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_invalid_sql() {
    let sql = CString::new("SELECT * FROM nonexistent_table").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Should return error for non-existent table
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("nonexistent_table") || 
            json["error"].as_str().unwrap().contains("not found"));
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_without_source_schema() {
    let sql = CString::new("SELECT 1 as num, 'hello' as greeting, true as flag").unwrap();
    let dialect = CString::new("postgres").unwrap();
    let suggested_name = CString::new("constants").unwrap();
    
    // Test without providing source schema (null pointer)
    let result_ptr = wren_extract_sql_schema(
        std::ptr::null(),
        sql.as_ptr(),
        dialect.as_ptr(),
        suggested_name.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("Test result without schema: {}", result_str);
    
    assert_eq!(json["success"], true);
    assert_eq!(json["tableName"], "constants");
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0]["name"], "num");
    assert_eq!(columns[1]["name"], "greeting");
    assert_eq!(columns[2]["name"], "flag");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_cte() {
    let sql = CString::new("WITH user_stats AS (SELECT user_id, COUNT(*) as order_count FROM orders GROUP BY user_id) SELECT * FROM user_stats").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "orders",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "user_id", "type": "integer", "nullable": true},
                {"name": "amount", "type": "decimal", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["success"], true);
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0]["name"], "user_id");
    assert_eq!(columns[1]["name"], "order_count");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_extract_schema_with_type_conversion() {
    let sql = CString::new("SELECT CAST(id AS varchar) as id_str, CAST(name AS text) as name_text FROM users").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer", "nullable": false},
                {"name": "name", "type": "varchar", "nullable": true}
            ]
        }
    ]"#).unwrap();
    
    let result_ptr = wren_extract_sql_schema(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr(),
        std::ptr::null()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    // Debug: print the result
    println!("Type conversion test result: {}", result_str);
    
    assert_eq!(json["success"], true);
    
    let columns = json["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0]["name"], "id_str");
    assert_eq!(columns[0]["type"], "varchar");
    assert_eq!(columns[1]["name"], "name_text");
    assert_eq!(columns[1]["type"], "varchar");
    
    wren_free_string(result_ptr);
}