use crate::level::{
    expression::{Expression, Type, Var},
    LevelSpec,
};

use super::*;
use ::serde::Deserialize;
use anyhow::*;
use serde::{Deserializer, Serialize};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
#[serde(transparent)]
pub(super) struct GameJson<'a>(#[serde(borrow)] HashMap<&'a str, LevelJson<'a>>);

impl<'a> TryFrom<GameJson<'a>> for GameData {
    type Error = Error;

    fn try_from(json: GameJson<'a>) -> Result<Self> {
        let indices: HashMap<&'a str, usize> = json.0.keys().copied().zip(0..).collect();

        let levels = json
            .0
            .into_iter()
            .map(|(name, json)| {
                json.parse(&indices, name.to_owned())
                    .with_context(|| format!("Failed to parse level {name}"))
            })
            .collect::<Result<_, _>>()?;

        Ok(GameData { levels })
    }
}

#[derive(Deserialize)]
struct LevelJson<'a> {
    #[serde(borrow)]
    variables: HashMap<&'a str, Type>,
    nodes: Vec<(ExpressionJson<'a, usize>, [f64; 2])>,
    hypotheses: Vec<usize>,
    conclusion: usize,
    #[serde(borrow)]
    text_box: Option<(&'a str, &'a str)>,
    map_position: [f64; 2],
    bezier_vector: [f64; 2],
    prereqs: Vec<&'a str>,
    #[serde(default, deserialize_with = "deserialize_some")]
    next_level: Option<Option<&'a str>>,
    #[serde(default)]
    unlocks: Unlocks,
    #[serde(default)]
    axiom: bool,
}

// https://github.com/serde-rs/serde/issues/984
// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

impl<'a> LevelJson<'a> {
    fn parse(self, indices: &HashMap<&'a str, usize>, name: String) -> Result<Level> {
        let Self {
            variables,
            nodes,
            hypotheses,
            conclusion,
            text_box,
            map_position,
            bezier_vector,
            prereqs,
            next_level,
            unlocks,
            axiom,
        } = self;

        let mut x_min = f64::INFINITY;
        let mut y_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for &(_, [x, y]) in &nodes {
            x_min = x_min.min(x);
            y_min = y_min.min(y);
            x_max = x_max.max(x);
            y_max = y_max.max(y);
        }

        Ok(Level {
            name,
            spec: LevelSpec::new(
                nodes
                    .into_iter()
                    .map(|(expr, pos)| Ok((expr.parse(&variables)?, pos)))
                    .collect::<Result<_>>()?,
                hypotheses,
                conclusion,
            )?,
            panzoom: crate::render::PanZoom {
                svg_corners: ([x_min - 1., y_min - 1.], [x_max + 1., y_max + 3.]),
            },
            text_box: text_box.map(|(msg, link)| (msg.to_owned(), link.to_owned())),
            map_position,
            bezier_vector,
            prereqs: prereqs
                .into_iter()
                .map(|x| {
                    indices
                        .get(x)
                        .copied()
                        .ok_or_else(|| anyhow!("Unknown level {} in prereqs.", x))
                })
                .collect::<Result<_, _>>()?,
            next_level: match next_level.ok_or_else(|| anyhow!("Missing next_level field."))? {
                Some(x) => Some(
                    *indices
                        .get(x)
                        .ok_or_else(|| anyhow!("Unknown level {} in next_level.", x))?,
                ),
                None => None,
            },
            unlocks,
            axiom,
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExpressionJson<'a, T> {
    Variable(&'a str),
    Other(&'a str, SmallVec<[T; 2]>),
}

impl<'a, T> ExpressionJson<'a, T> {
    fn parse(self, variables: &HashMap<&'a str, Type>) -> Result<Expression<T>> {
        Ok(match self {
            ExpressionJson::Variable(v) => Expression::Variable(Var(
                v.to_owned(),
                *variables
                    .get(v)
                    .ok_or(anyhow!("Variable {}'s type is not stated.", v))?,
            )),
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

#[derive(Serialize, Deserialize)]
pub(super) struct SaveJson<'a> {
    #[serde(borrow)]
    completed: HashSet<&'a str>,
    unlocks: Unlocks,
}

impl<'a> SaveJson<'a> {
    pub(super) fn to_data(&self, game_data: &GameData) -> SaveData {
        SaveData {
            unlocks: self.unlocks,
            completed: (0..game_data.num_levels())
                .map(|level| {
                    self.completed
                        .contains(&game_data.levels[level].name.as_str())
                })
                .collect(),
        }
    }
}

impl SaveData {
    pub(super) fn to_json<'a>(&self, game_data: &'a GameData) -> SaveJson<'a> {
        SaveJson {
            completed: self
                .completed
                .iter()
                .enumerate()
                .filter_map(|(level, completed)| {
                    if *completed {
                        Some(game_data.levels[level].name.as_str())
                    } else {
                        None
                    }
                })
                .collect(),
            unlocks: self.unlocks,
        }
    }
}
