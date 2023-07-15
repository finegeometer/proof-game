use super::*;
use ::serde::Deserialize;
use anyhow::*;

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExpressionJson<'a, T> {
    Variable(&'a str),
    Other(&'a str, SmallVec<[T; 2]>),
}

impl<'a, T> TryFrom<ExpressionJson<'a, T>> for Expression<T> {
    type Error = Error;

    fn try_from(json: ExpressionJson<'a, T>) -> Result<Self> {
        Ok(match json {
            ExpressionJson::Variable(v) => {
                Expression::Variable(Var(v.to_owned(), Type::TruthValue))
            }
            ExpressionJson::Other("∧", inputs) => Expression::And(inputs),
            ExpressionJson::Other("∨", inputs) => Expression::Or(inputs),
            ExpressionJson::Other("⇒", inputs) => {
                Expression::Implies(inputs.into_inner().map_err(|inputs| {
                    anyhow!(
                        "Wrong number of inputs to `⇒`: expected 2, found {}.",
                        inputs.len()
                    )
                })?)
            }
            ExpressionJson::Other("=", inputs) => {
                Expression::Equal(inputs.into_inner().map_err(|inputs| {
                    anyhow!(
                        "Wrong number of inputs to `=`: expected 2, found {}.",
                        inputs.len()
                    )
                })?)
            }
            ExpressionJson::Other(f, inputs) => {
                Expression::Function(f.to_owned(), Type::TruthValue, inputs)
            }
        })
    }
}
