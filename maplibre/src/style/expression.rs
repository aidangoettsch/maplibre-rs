use std::collections::HashMap;
use std::fmt;
use geozero::ColumnValue;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde::de::{SeqAccess, Visitor};

#[derive(Serialize, Debug, Clone)]
pub enum ExpressionComparisonOp {
    Eq,
    Neq,
    Gt,
    Geq,
    Lt,
    Leq,
}

impl ExpressionComparisonOp {
    fn compare(&self, a: ComparisonLiteral, b: ComparisonLiteral) -> bool {
        match self {
            ExpressionComparisonOp::Eq => a == b,
            ExpressionComparisonOp::Neq => a != b,
            ExpressionComparisonOp::Gt => {
                match (a, b) {
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Integer(b)) => a > b,
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Float(b)) => (a as f64) > b,
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Integer(b)) => a > (b as f64),
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Float(b)) => a > b,
                    (ComparisonLiteral::String(a), ComparisonLiteral::String(b)) => a > b,
                    _ => false,
                }
            }
            ExpressionComparisonOp::Geq => {
                match (a, b) {
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Integer(b)) => a >= b,
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Float(b)) => (a as f64) >= b,
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Integer(b)) => a >= (b as f64),
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Float(b)) => a >= b,
                    (ComparisonLiteral::String(a), ComparisonLiteral::String(b)) => a >= b,
                    _ => false,
                }
            }
            ExpressionComparisonOp::Lt => {
                match (a, b) {
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Integer(b)) => a < b,
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Float(b)) => (a as f64) < b,
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Integer(b)) => a < (b as f64),
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Float(b)) => a < b,
                    (ComparisonLiteral::String(a), ComparisonLiteral::String(b)) => a < b,
                    _ => false,
                }
            }
            ExpressionComparisonOp::Leq => {
                match (a, b) {
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Integer(b)) => a <= b,
                    (ComparisonLiteral::Integer(a), ComparisonLiteral::Float(b)) => (a as f64) <= b,
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Integer(b)) => a <= (b as f64),
                    (ComparisonLiteral::Float(a), ComparisonLiteral::Float(b)) => a <= b,
                    (ComparisonLiteral::String(a), ComparisonLiteral::String(b)) => a <= b,
                    _ => false,
                }
            }
        }
    }
}

impl TryFrom<String> for ExpressionComparisonOp {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "==" => Ok(Self::Eq),
            "!=" => Ok(Self::Neq),
            ">" => Ok(Self::Gt),
            ">=" => Ok(Self::Geq),
            "<" => Ok(Self::Lt),
            "<=" => Ok(Self::Leq),
            _ => Err(())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ComparisonLiteral {
    Float(f64),
    Integer(isize),
    Bool(bool),
    String(String),
}

impl From<&ColumnValue<'_>> for ComparisonLiteral {
    fn from(value: &ColumnValue) -> Self {
        match value {
            ColumnValue::Bool(v) => ComparisonLiteral::Bool(*v),
            ColumnValue::UByte(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::Byte(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::Short(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::UShort(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::Int(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::UInt(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::Long(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::ULong(v) => ComparisonLiteral::Integer(*v as isize),
            ColumnValue::Float(v) => ComparisonLiteral::Float(*v as f64),
            ColumnValue::Double(v) => ComparisonLiteral::Float(*v),
            ColumnValue::String(v) | ColumnValue::Json(v) => ComparisonLiteral::String(v.to_string()),
            ColumnValue::DateTime(_) => unimplemented!("Date property comparisons are not supported"),
            ColumnValue::Binary(_) => unimplemented!("Binary property comparisons are not supported"),
        }
    }
}

// https://maplibre.org/maplibre-style-spec/deprecations/#other-filter
// TODO(aidangoettsch): create custom serialization
#[derive(Serialize, Debug, Clone)]
pub enum LegacyFilterExpression {
    // Existential
    Has(String),
    NotHas(String),
    // Comparison
    Comparison(ExpressionComparisonOp, String, ComparisonLiteral),
    // Membership
    In(String, Vec<String>),
    NotIn(String, Vec<String>),
    // Combining
    All(Vec<LegacyFilterExpression>),
    Any(Vec<LegacyFilterExpression>),
    None(Vec<LegacyFilterExpression>),
}

impl<'de> Deserialize<'de> for LegacyFilterExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        struct LegacyFilterVisitor;

        impl<'de> Visitor<'de> for LegacyFilterVisitor {
            type Value = LegacyFilterExpression;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence conforming to the legacy filter specification")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<LegacyFilterExpression, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let kw = seq.next_element::<String>()?.ok_or_else(||
                    de::Error::custom("filter array was empty")
                )?;

                match kw.as_str() {
                    "has" => {
                        let property = seq.next_element::<String>()?.ok_or_else(||
                            de::Error::custom("has filter was missing property")
                        )?;

                        Ok(LegacyFilterExpression::Has(property))
                    },
                    "!has" => {
                        let property = seq.next_element::<String>()?.ok_or_else(||
                            de::Error::custom("!has filter was missing property")
                        )?;

                        Ok(LegacyFilterExpression::NotHas(property))},
                    x if ExpressionComparisonOp::try_from(x.to_string()).is_ok() => {
                        let op = ExpressionComparisonOp::try_from(x.to_string()).unwrap();
                        let property = seq.next_element::<String>()?.ok_or_else(||
                            de::Error::custom("comparison filter was missing property")
                        )?;
                        let literal = seq.next_element::<ComparisonLiteral>()?.ok_or_else(||
                            de::Error::custom("!has filter was missing literal")
                        )?;

                        Ok(LegacyFilterExpression::Comparison(op, property, literal))
                    },
                    "in" => {
                        let property = seq.next_element::<String>()?.ok_or_else(||
                            de::Error::custom("comparison filter was missing property")
                        )?;

                        let mut predicates = vec![];

                        while let Some(predicate) = seq.next_element::<String>()? {
                            predicates.push(predicate);
                        }

                        Ok(LegacyFilterExpression::In(property, predicates))
                    },
                    "!in" => {
                        let property = seq.next_element::<String>()?.ok_or_else(||
                            de::Error::custom("comparison filter was missing property")
                        )?;

                        let mut predicates = vec![];

                        while let Some(predicate) = seq.next_element::<String>()? {
                            predicates.push(predicate);
                        }

                        Ok(LegacyFilterExpression::NotIn(property, predicates))
                    },
                    "all" => {
                        let mut filters = vec![];

                        while let Some(filter) = seq.next_element::<LegacyFilterExpression>()? {
                            filters.push(filter);
                        }

                        Ok(LegacyFilterExpression::All(filters))
                    },
                    "any" => {
                        let mut filters = vec![];

                        while let Some(filter) = seq.next_element::<LegacyFilterExpression>()? {
                            filters.push(filter);
                        }

                        Ok(LegacyFilterExpression::Any(filters))
                    },
                    "none" => {
                        let mut filters = vec![];

                        while let Some(filter) = seq.next_element::<LegacyFilterExpression>()? {
                            filters.push(filter);
                        }

                        Ok(LegacyFilterExpression::None(filters))
                    },
                    _ => Err(de::Error::custom(format!("Invalid filter keyword {kw}"))),
                }
            }
        }

        deserializer.deserialize_seq(LegacyFilterVisitor)
    }
}

impl LegacyFilterExpression {
    pub fn evaluate(&self, properties: &HashMap<String, ComparisonLiteral>) -> bool {
        match self {
            LegacyFilterExpression::Has(key) => properties.contains_key(key),
            LegacyFilterExpression::NotHas(key) => !properties.contains_key(key),
            LegacyFilterExpression::Comparison(op, key, value) => {
                if let Some(v) = properties.get(key) {
                    op.compare(v.clone(), value.clone())
                } else {
                    false
                }
            },
            LegacyFilterExpression::In(key, predicates) => properties.get(key).is_some_and(|v| match v {
                ComparisonLiteral::String(s) => predicates.contains(s),
                _ => unimplemented!("In expression is not supported for non-string types"),
            }),
            LegacyFilterExpression::NotIn(key, predicates) => properties.get(key).is_some_and(|v| match v {
                ComparisonLiteral::String(s) => !predicates.contains(s),
                _ => unimplemented!("In expression is not supported for non-string types"),
            }),
            LegacyFilterExpression::All(children) => children.iter().all(|c| c.evaluate(properties)),
            LegacyFilterExpression::Any(children) => children.iter().any(|c| c.evaluate(properties)),
            LegacyFilterExpression::None(children) => children.iter().all(|c| !c.evaluate(properties)),
        }
    }
}