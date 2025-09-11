/*!
 * 查询构建器模块
 *
 * 提供类型安全的SQL查询构建功能，避免SQL注入风险
 */

use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde_json::Value;

use std::collections::HashMap;

/// 查询条件
#[derive(Debug, Clone)]
pub enum QueryCondition {
    Eq(String, Value),
    Ne(String, Value),
    Lt(String, Value),
    Le(String, Value),
    Gt(String, Value),
    Ge(String, Value),
    Like(String, String),
    In(String, Vec<Value>),
    IsNull(String),
    IsNotNull(String),
    Between(String, Value, Value),
    And(Vec<QueryCondition>),
    Or(Vec<QueryCondition>),
}

impl QueryCondition {
    /// 创建等于条件
    pub fn new(field: &str, operator: &str) -> Self {
        match operator {
            "=" => QueryCondition::Eq(field.to_string(), Value::Null),
            "!=" => QueryCondition::Ne(field.to_string(), Value::Null),
            "<" => QueryCondition::Lt(field.to_string(), Value::Null),
            "<=" => QueryCondition::Le(field.to_string(), Value::Null),
            ">" => QueryCondition::Gt(field.to_string(), Value::Null),
            ">=" => QueryCondition::Ge(field.to_string(), Value::Null),
            _ => QueryCondition::Eq(field.to_string(), Value::Null),
        }
    }
    
    /// 创建等于条件
    pub fn eq(field: &str, value: Value) -> Self {
        QueryCondition::Eq(field.to_string(), value)
    }
}

/// 排序方向
#[derive(Debug, Clone)]
pub enum QueryOrder {
    Asc(String),
    Desc(String),
}

/// 类型安全的查询构建器
#[derive(Debug)]
pub struct SafeQueryBuilder {
    table: String,
    select_fields: Vec<String>,
    conditions: Vec<QueryCondition>,
    orders: Vec<QueryOrder>,
    limit: Option<i64>,
    offset: Option<i64>,
    joins: Vec<String>,
}

impl SafeQueryBuilder {
    /// 创建新的查询构建器
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            select_fields: vec!["*".to_string()],
            conditions: Vec::new(),
            orders: Vec::new(),
            limit: None,
            offset: None,
            joins: Vec::new(),
        }
    }

    /// 选择字段
    pub fn select(mut self, fields: &[&str]) -> Self {
        self.select_fields = fields.iter().map(|f| f.to_string()).collect();
        self
    }

    /// 添加条件
    pub fn where_condition(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// 添加排序
    pub fn order_by(mut self, order: QueryOrder) -> Self {
        self.orders.push(order);
        self
    }

    /// 设置限制
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置偏移
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 添加JOIN
    pub fn join(mut self, join_clause: impl Into<String>) -> Self {
        self.joins.push(join_clause.into());
        self
    }

    /// 构建SELECT查询
    pub fn build_select_all(self) -> AppResult<(String, Vec<Value>)> {
        self.build()
    }

    /// 构建查询
    pub fn build(self) -> AppResult<(String, Vec<Value>)> {
        let mut query = format!(
            "SELECT {} FROM {}",
            self.select_fields.join(", "),
            self.table
        );

        // 添加JOIN
        for join in &self.joins {
            query.push(' ');
            query.push_str(join);
        }

        let mut params = Vec::new();

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            query.push_str(" WHERE ");
            let (condition_sql, condition_params) = self.build_conditions(&self.conditions)?;
            query.push_str(&condition_sql);
            params.extend(condition_params);
        }

        // 添加ORDER BY
        if !self.orders.is_empty() {
            query.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self
                .orders
                .iter()
                .map(|order| match order {
                    QueryOrder::Asc(field) => format!("{} ASC", field),
                    QueryOrder::Desc(field) => format!("{} DESC", field),
                })
                .collect();
            query.push_str(&order_clauses.join(", "));
        }

        // 添加LIMIT
        if let Some(limit) = self.limit {
            query.push_str(" LIMIT ?");
            params.push(Value::Number(limit.into()));
        }

        // 添加OFFSET
        if let Some(offset) = self.offset {
            query.push_str(" OFFSET ?");
            params.push(Value::Number(offset.into()));
        }

        Ok((query, params))
    }

    /// 构建条件SQL
    fn build_conditions(&self, conditions: &[QueryCondition]) -> AppResult<(String, Vec<Value>)> {
        if conditions.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut sql_parts = Vec::new();
        let mut params = Vec::new();

        for condition in conditions {
            let (condition_sql, condition_params) = self.build_single_condition(condition)?;
            sql_parts.push(condition_sql);
            params.extend(condition_params);
        }

        Ok((sql_parts.join(" AND "), params))
    }

    /// 构建单个条件
    fn build_single_condition(
        &self,
        condition: &QueryCondition,
    ) -> AppResult<(String, Vec<Value>)> {
        match condition {
            QueryCondition::Eq(field, value) => Ok((format!("{} = ?", field), vec![value.clone()])),
            QueryCondition::Ne(field, value) => {
                Ok((format!("{} != ?", field), vec![value.clone()]))
            }
            QueryCondition::Lt(field, value) => Ok((format!("{} < ?", field), vec![value.clone()])),
            QueryCondition::Le(field, value) => {
                Ok((format!("{} <= ?", field), vec![value.clone()]))
            }
            QueryCondition::Gt(field, value) => Ok((format!("{} > ?", field), vec![value.clone()])),
            QueryCondition::Ge(field, value) => {
                Ok((format!("{} >= ?", field), vec![value.clone()]))
            }
            QueryCondition::Like(field, pattern) => Ok((
                format!("{} LIKE ?", field),
                vec![Value::String(pattern.clone())],
            )),
            QueryCondition::In(field, values) => {
                let placeholders = vec!["?"; values.len()].join(", ");
                Ok((format!("{} IN ({})", field, placeholders), values.clone()))
            }
            QueryCondition::IsNull(field) => Ok((format!("{} IS NULL", field), Vec::new())),
            QueryCondition::IsNotNull(field) => Ok((format!("{} IS NOT NULL", field), Vec::new())),
            QueryCondition::Between(field, start, end) => Ok((
                format!("{} BETWEEN ? AND ?", field),
                vec![start.clone(), end.clone()],
            )),
            QueryCondition::And(conditions) => {
                let (sql, params) = self.build_conditions(conditions)?;
                Ok((format!("({})", sql), params))
            }
            QueryCondition::Or(conditions) => {
                let mut sql_parts = Vec::new();
                let mut params = Vec::new();

                for condition in conditions {
                    let (condition_sql, condition_params) =
                        self.build_single_condition(condition)?;
                    sql_parts.push(condition_sql);
                    params.extend(condition_params);
                }

                Ok((format!("({})", sql_parts.join(" OR ")), params))
            }
        }
    }
}

/// 插入构建器
#[derive(Debug)]
pub struct InsertBuilder {
    table: String,
    fields: HashMap<String, Value>,
    on_conflict: Option<String>,
}

impl InsertBuilder {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            fields: HashMap::new(),
            on_conflict: None,
        }
    }

    pub fn set(mut self, field: impl Into<String>, value: Value) -> Self {
        self.fields.insert(field.into(), value);
        self
    }

    pub fn on_conflict_replace(mut self) -> Self {
        self.on_conflict = Some("REPLACE".to_string());
        self
    }

    pub fn on_conflict_ignore(mut self) -> Self {
        self.on_conflict = Some("IGNORE".to_string());
        self
    }

    pub fn add_field(mut self, field: &str, value: &impl serde::Serialize) -> Self {
        let json_value = serde_json::to_value(value).unwrap_or(Value::Null);
        self.fields.insert(field.to_string(), json_value);
        self
    }

    pub fn add_field_opt<T: serde::Serialize>(mut self, field: &str, value: Option<&T>) -> Self {
        let json_value = match value {
            Some(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            None => Value::Null,
        };
        self.fields.insert(field.to_string(), json_value);
        self
    }

    pub fn build(self) -> AppResult<(String, Vec<Value>)> {
        if self.fields.is_empty() {
            return Err(anyhow!("No fields specified for insert"));
        }

        let fields: Vec<String> = self.fields.keys().cloned().collect();
        let placeholders = vec!["?"; fields.len()].join(", ");
        let values: Vec<Value> = fields.iter().map(|f| self.fields[f].clone()).collect();

        let query = match &self.on_conflict {
            Some(action) => format!(
                "INSERT OR {} INTO {} ({}) VALUES ({})",
                action,
                self.table,
                fields.join(", "),
                placeholders
            ),
            None => format!(
                "INSERT INTO {} ({}) VALUES ({})",
                self.table,
                fields.join(", "),
                placeholders
            ),
        };

        Ok((query, values))
    }
}

/// 更新构建器
#[derive(Debug)]
pub struct UpdateBuilder {
    table: String,
    fields: HashMap<String, Value>,
    conditions: Vec<QueryCondition>,
}

impl UpdateBuilder {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            fields: HashMap::new(),
            conditions: Vec::new(),
        }
    }

    pub fn set(mut self, field: impl Into<String>, value: Value) -> Self {
        self.fields.insert(field.into(), value);
        self
    }

    pub fn where_condition(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn build(self) -> AppResult<(String, Vec<Value>)> {
        if self.fields.is_empty() {
            return Err(anyhow!("No fields specified for update"));
        }

        let set_clauses: Vec<String> = self.fields.keys().map(|f| format!("{} = ?", f)).collect();
        let mut values: Vec<Value> = self.fields.values().cloned().collect();

        let mut query = format!("UPDATE {} SET {}", self.table, set_clauses.join(", "));

        if !self.conditions.is_empty() {
            let builder = SafeQueryBuilder::new(&self.table);
            let (condition_sql, condition_params) = builder.build_conditions(&self.conditions)?;
            query.push_str(" WHERE ");
            query.push_str(&condition_sql);
            values.extend(condition_params);
        }

        Ok((query, values))
    }
}
