use std::fmt::Display;

use serde::{de::Visitor, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    And,
    Or,
    Implies,
    Equal,
    Other(String),
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Operation::And => "∧",
            Operation::Or => "∨",
            Operation::Implies => "⇒",
            Operation::Equal => "=",
            Operation::Other(s) => s,
        })
    }
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(OpVisitor)
    }
}

struct OpVisitor;
impl<'de> Visitor<'de> for OpVisitor {
    type Value = Operation;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Expecting a string: either one of the special operations (∧,∨,⇒,=), or a variable name.")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v {
            "∧" => Operation::And,
            "∨" => Operation::Or,
            "⇒" => Operation::Implies,
            "=" => Operation::Equal,
            _ => Operation::Other(v.into()),
        })
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v.as_str() {
            "∧" => Operation::And,
            "∨" => Operation::Or,
            "⇒" => Operation::Implies,
            "=" => Operation::Equal,
            _ => Operation::Other(v),
        })
    }
}
