use crate::level::{expression::Expression, LevelSpec};

use super::*;
use ::serde::Deserialize;
use anyhow::*;
use serde::{Deserializer, Serialize};
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
    nodes: Vec<(&'a str, Vec<usize>, [f64; 2])>,
    hypotheses: Vec<usize>,
    conclusion: usize,
    text_box: Option<&'a str>,
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

        let nodes = nodes
            .into_iter()
            .map(|(op, inputs, position)| {
                x_min = x_min.min(position[0]);
                y_min = y_min.min(position[1]);
                x_max = x_max.max(position[0]);
                y_max = y_max.max(position[1]);

                let expression = match op {
                    "∧" => Expression::And(inputs.into()),
                    "∨" => Expression::Or(inputs.into()),
                    "⇒" => {
                        Expression::Implies(<[usize; 2]>::try_from(inputs).map_err(|inputs| {
                            anyhow!(
                                "Wrong number of inputs to `⇒`: expected 2, found {}.",
                                inputs.len()
                            )
                        })?)
                    }
                    "=" => Expression::Equal(<[usize; 2]>::try_from(inputs).map_err(|inputs| {
                        anyhow!(
                            "Wrong number of inputs to `=`: expected 2, found {}.",
                            inputs.len()
                        )
                    })?),
                    _ => Expression::Other(op.to_owned()),
                };

                Ok((expression, position))
            })
            .collect::<Result<_>>()?;

        Ok(Level {
            name,
            spec: LevelSpec::new(nodes, hypotheses, conclusion)?,
            panzoom: crate::render::PanZoom {
                svg_corners: ([x_min - 1., y_min - 1.], [x_max + 1., y_max + 3.]),
            },
            text_box: text_box.map(|s| s.to_owned()),
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
