use super::*;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub severity: String,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub component: String,
    pub metric: String,
    pub operator: Operator,
    pub value: f64,
    pub next_logical: Option<LogicalOperator>,
}

#[derive(Debug, Clone, Copy)]
pub enum Operator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl FromStr for Operator {
    type Err = MetricError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" => Ok(Operator::GreaterThan),
            "<" => Ok(Operator::LessThan),
            ">=" => Ok(Operator::GreaterThanOrEqual),
            "<=" => Ok(Operator::LessThanOrEqual),
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::NotEqual),
            _ => Err(MetricError::InvalidValue(format!(
                "Invalid operator: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogicalOperator {
    And,
    Or,
}

impl FromStr for LogicalOperator {
    type Err = MetricError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AND" => Ok(LogicalOperator::And),
            "OR" => Ok(LogicalOperator::Or),
            _ => Err(MetricError::InvalidValue(format!(
                "Invalid logical operator: {}",
                s
            ))),
        }
    }
}

// Rule parser that handles the expression syntax
pub struct RuleParser;

impl RuleParser {
    pub fn parse_expression(expression: &str) -> Result<Vec<Condition>, MetricError> {
        use regex::Regex;

        lazy_static::lazy_static! {
            static ref COMPONENT_RE: Regex = Regex::new(
                r"^([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\s*([<>!=]+)\s*([a-zA-Z0-9_.]+)"
            ).unwrap();
            static ref LOGICAL_RE: Regex = Regex::new(r"\s+(AND|OR)\s+").unwrap();
        }

        let segments: Vec<&str> = LOGICAL_RE.split(expression).collect();
        let operators: Vec<&str> = LOGICAL_RE
            .find_iter(expression)
            .map(|m| m.as_str().trim())
            .collect();

        let mut conditions = Vec::new();

        for (i, segment) in segments.iter().enumerate() {
            if let Some(caps) = COMPONENT_RE.captures(segment) {
                let component = caps.get(1).unwrap().as_str().to_string();
                let metric = caps.get(2).unwrap().as_str().to_string();
                let operator = Operator::from_str(caps.get(3).unwrap().as_str())?;
                let value = caps.get(4).unwrap().as_str().parse::<f64>().map_err(|_| {
                    MetricError::InvalidValue(format!(
                        "Invalid numeric value: {}",
                        caps.get(4).unwrap().as_str()
                    ))
                })?;

                let next_logical = if i < operators.len() {
                    Some(LogicalOperator::from_str(operators[i])?)
                } else {
                    None
                };

                conditions.push(Condition {
                    component,
                    metric,
                    operator,
                    value,
                    next_logical,
                });
            }
        }

        Ok(conditions)
    }
}

// Rule evaluator that works with the MetricRegistry
pub struct RuleEvaluator<'a> {
    registry: &'a MetricRegistry,
}

impl<'a> RuleEvaluator<'a> {
    pub fn new(registry: &'a MetricRegistry) -> Self {
        Self { registry }
    }

    pub async fn evaluate_condition(&self, condition: &Condition) -> Result<bool, MetricError> {
        let metric_value = self
            .registry
            .get_metric_value(&condition.component, &condition.metric)
            .await?;

        Ok(match condition.operator {
            Operator::GreaterThan => metric_value > condition.value,
            Operator::LessThan => metric_value < condition.value,
            Operator::GreaterThanOrEqual => metric_value >= condition.value,
            Operator::LessThanOrEqual => metric_value <= condition.value,
            Operator::Equal => (metric_value - condition.value).abs() < f64::EPSILON,
            Operator::NotEqual => (metric_value - condition.value).abs() >= f64::EPSILON,
        })
    }

    pub async fn evaluate_rule(&self, rule: &Rule) -> Result<bool, MetricError> {
        let mut result = true;

        for condition in &rule.conditions {
            let condition_result = self.evaluate_condition(condition).await?;

            match (condition.next_logical, result, condition_result) {
                (Some(LogicalOperator::And), true, false) => return Ok(false),
                (Some(LogicalOperator::Or), false, true) => return Ok(true),
                (Some(LogicalOperator::And), _, _) => result &= condition_result,
                (Some(LogicalOperator::Or), _, _) => result |= condition_result,
                (None, _, _) => result = condition_result,
            }
        }

        Ok(result)
    }
}
