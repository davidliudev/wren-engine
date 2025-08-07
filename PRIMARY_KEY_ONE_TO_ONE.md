# Primary Key Behavior with One-to-One Relationships

## How Primary Keys Work with Different Relationship Types

### Core Concept
The primary key requirement for calculated columns is **independent of the relationship type** (one-to-one, one-to-many, many-to-one, many-to-many). The primary key serves the same purpose regardless: it acts as a correlation key to match calculated results back to their parent rows.

### One-to-One Relationships

For one-to-one relationships, the primary key works exactly the same as with other relationship types:

1. **Primary Key is Still Required**: Even with 1-1 relationships, if you have a calculated column that references another model, you need a primary key.

2. **No Special Optimization**: The code doesn't have special handling for 1-1 relationships. It uses the same CalculationPlanNode logic.

3. **Why Not Optimized?**: While theoretically a 1-1 relationship could be optimized to avoid the subquery pattern, Wren treats all relationships uniformly for consistency and simplicity.

### Example: User and Profile (1-1)

```json
{
  "models": [
    {
      "name": "users",
      "primaryKey": "user_id",  // Required for profile_bio calculated column
      "columns": [
        {"name": "user_id", "type": "integer"},
        {"name": "username", "type": "varchar"},
        {
          "name": "profile_bio",
          "type": "varchar",
          "isCalculated": true,
          "expression": "profiles.bio"  // References another model
        }
      ]
    },
    {
      "name": "profiles",
      "primaryKey": "user_id",
      "columns": [
        {"name": "user_id", "type": "integer"},
        {"name": "bio", "type": "text"},
        {"name": "avatar_url", "type": "varchar"}
      ]
    }
  ],
  "relationships": [
    {
      "name": "users_profiles",
      "models": ["users", "profiles"],
      "joinType": "one_to_one",
      "condition": "users.user_id = profiles.user_id"
    }
  ]
}
```

### Query Transformation (Same as Other Relationships)

When querying `SELECT username, profile_bio FROM users`, Wren generates:

1. **Calculation Subquery** (includes primary key):
```sql
SELECT 
  users.user_id,        -- Primary key for correlation
  profiles.bio AS profile_bio
FROM users
LEFT JOIN profiles ON users.user_id = profiles.user_id
-- No GROUP BY needed for 1-1, but Wren doesn't optimize this
```

2. **Final Query**:
```sql
SELECT 
  main.username,
  calc.profile_bio
FROM users AS main
LEFT JOIN (calculation_subquery) AS calc 
  ON main.user_id = calc.user_id  -- Primary key correlation
```

### Key Observations

1. **No GROUP BY Optimization**: Even though 1-1 relationships don't need GROUP BY (no aggregation), Wren doesn't detect this and optimize away the subquery pattern.

2. **Uniform Treatment**: The code in `CalculationPlanNode::new()` doesn't check `join_type` to optimize for 1-1 relationships:
   ```rust
   // From plan.rs:990-993
   let Some(pk_column) = model.primary_key().and_then(|pk| model.get_column(pk))
   else {
       return plan_err!("Primary key not found");
   };
   ```

3. **Same Correlation Pattern**: The primary key is used as a correlation key regardless of whether it's 1-1, 1-many, or many-many.

### Why This Design?

1. **Simplicity**: One code path for all relationship types reduces complexity.

2. **Consistency**: All calculated columns behave the same way, making the system more predictable.

3. **Future Flexibility**: The subquery pattern allows for future enhancements like adding filters or transformations.

4. **Correctness Over Performance**: The current approach guarantees correctness for all relationship types, even if it's not optimal for 1-1.

### Performance Implications

For 1-1 relationships, this means:
- **Extra Join**: An unnecessary subquery join when a simple join would suffice
- **No Aggregation Overhead**: Since there's no GROUP BY for 1-1, the performance impact is mainly the extra join
- **Potential Optimization**: A future enhancement could detect 1-1 relationships and use a simpler join pattern

### Conclusion

The primary key requirement and behavior for calculated columns is **uniform across all relationship types**, including one-to-one. While this may seem inefficient for 1-1 relationships, it provides a consistent, predictable model that works correctly for all cases. The primary key always serves as the correlation key to reassemble calculated results with their parent rows, regardless of the relationship cardinality.