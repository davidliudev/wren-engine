use wren_core::*;
use std::ffi::{CStr, CString};

#[test]
fn test_validate_sql_syntax_only() {
    // Test valid SQL syntax without schema
    let sql = CString::new("SELECT * FROM users WHERE id = 1").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        std::ptr::null(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    assert!(json["message"].as_str().unwrap().contains("SQL syntax is valid"));
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_syntax_error() {
    // Test invalid SQL syntax
    let sql = CString::new("SELECT * FORM users").unwrap(); // typo: FORM instead of FROM
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        std::ptr::null(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], false);
    assert_eq!(json["error_type"], "syntax");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_with_schema_valid() {
    // Test valid SQL with schema
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"},
                {"name": "email", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT id, name FROM users WHERE id = 1").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_table_not_found() {
    // Test SQL with non-existent table
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT * FROM products").unwrap(); // products table doesn't exist
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], false);
    assert_eq!(json["error_type"], "table_not_found");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_column_not_found() {
    // Test SQL with non-existent column
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT id, name, email FROM users").unwrap(); // email column doesn't exist
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], false);
    assert_eq!(json["error_type"], "column_not_found");
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_with_schema_prefix() {
    // Test SQL with schema-prefixed table names
    let schema = CString::new(r#"[
        {
            "catalog": "db",
            "schema": "public",
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT * FROM public.users").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_complex_sql() {
    // Test complex SQL with joins
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"}
            ]
        },
        {
            "tableName": "orders",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "user_id", "type": "integer"},
                {"name": "total", "type": "decimal"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT u.name, SUM(o.total) as total_spent FROM users u JOIN orders o ON u.id = o.user_id GROUP BY u.name").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_with_cte() {
    // Test SQL with Common Table Expression
    let schema = CString::new(r#"[
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"},
                {"name": "department", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("WITH dept_users AS (SELECT * FROM users WHERE department = 'Engineering') SELECT name FROM dept_users").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    
    wren_free_string(result_ptr);
}

#[test]
fn test_validate_sql_with_subquery() {
    // Test SQL with subquery
    let schema = CString::new(r#"[
        {
            "tableName": "orders",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "user_id", "type": "integer"},
                {"name": "amount", "type": "decimal"}
            ]
        },
        {
            "tableName": "users",
            "columns": [
                {"name": "id", "type": "integer"},
                {"name": "name", "type": "varchar"}
            ]
        }
    ]"#).unwrap();
    
    let sql = CString::new("SELECT name FROM users WHERE id IN (SELECT user_id FROM orders WHERE amount > 100)").unwrap();
    let dialect = CString::new("postgres").unwrap();
    
    let result_ptr = wren_validate_sql(
        schema.as_ptr(),
        sql.as_ptr(),
        dialect.as_ptr()
    );
    
    assert!(!result_ptr.is_null());
    
    let result_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(result_str).unwrap();
    
    assert_eq!(json["valid"], true);
    
    wren_free_string(result_ptr);
}