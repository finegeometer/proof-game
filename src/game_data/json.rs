use crate::level::case::{Case, ValidityReason, Wire};
use crate::level::expression::Expression;

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
    unlocks: Vec<&'a str>,
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
    fn parse(&self, indices: &HashMap<&'a str, usize>, name: String) -> Result<Level> {
        let mut x_min = f64::INFINITY;
        let mut y_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        let mut case = Case::new();
        let mut wires = Vec::with_capacity(self.nodes.len());
        for (op, inputs, position) in &self.nodes {
            x_min = x_min.min(position[0]);
            y_min = y_min.min(position[1]);
            x_max = x_max.max(position[0]);
            y_max = y_max.max(position[1]);

            let inputs = inputs.iter().map(|idx| {
                wires
                    .get(*idx)
                    .copied()
                    .ok_or_else(|| anyhow!("Node {} depends on later node {}", wires.len(), idx))
            });
            let expression = {
                match *op {
                    "∧" => Expression::And(inputs.collect::<Result<_, _>>()?),
                    "∨" => Expression::Or(inputs.collect::<Result<_, _>>()?),
                    "⇒" => Expression::Implies({
                        inputs
                            .collect::<Result<SmallVec<[Wire; 2]>, _>>()?
                            .into_inner()
                            .map_err(|inputs| {
                                anyhow!(
                                    "Incorrect number of inputs to `⇒`: expected 2, found {}",
                                    inputs.len()
                                )
                            })?
                    }),
                    "=" => Expression::Equal({
                        inputs
                            .collect::<Result<SmallVec<[Wire; 2]>, _>>()?
                            .into_inner()
                            .map_err(|inputs| {
                                anyhow!(
                                    "Incorrect number of inputs to `=`: expected 2, found {}",
                                    inputs.len()
                                )
                            })?
                    }),
                    _ => Expression::Other((*op).into()),
                }
            };
            let node = case.make_node(expression, *position);
            wires.push(case.node_output(node));
        }
        for idx in self.hypotheses.iter() {
            case.set_proven(
                *wires.get(*idx).ok_or_else(|| {
                    anyhow!("Hypothesis index too large. ({} >= {})", idx, wires.len())
                })?,
                ValidityReason::new("By assumption."),
            )
        }
        case.set_goal(*wires.get(self.conclusion).ok_or_else(|| {
            anyhow!(
                "Conclusion index too large. ({} >= {})",
                self.conclusion,
                wires.len()
            )
        })?);
        Ok(Level {
            name,
            case,
            pan_zoom: crate::render::PanZoom {
                svg_corners: ([x_min - 1., y_min - 1.], [x_max + 1., y_max + 3.]),
            },
            text_box: self.text_box.map(|s| s.to_owned()),
            map_position: self.map_position,
            bezier_vector: self.bezier_vector,
            prereqs: self
                .prereqs
                .iter()
                .map(|x| {
                    indices
                        .get(x)
                        .copied()
                        .ok_or_else(|| anyhow!("Unknown level {} in prereqs.", x))
                })
                .collect::<Result<_, _>>()?,
            next_level: match self
                .next_level
                .ok_or_else(|| anyhow!("Missing next_level field."))?
            {
                Some(x) => Some(
                    *indices
                        .get(x)
                        .ok_or_else(|| anyhow!("Unknown level {} in next_level.", x))?,
                ),
                None => None,
            },
            unlocks: {
                let mut out = crate::UnlockState::None;
                for &unlock in &self.unlocks {
                    out = out.max(match unlock {
                        "cases" => crate::UnlockState::CaseTree,
                        "lemmas" => crate::UnlockState::Lemmas,
                        _ => crate::UnlockState::None,
                    })
                }
                out
            },
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct SaveJson<'a> {
    #[serde(borrow)]
    completed: HashSet<&'a str>,
    #[serde(borrow)]
    unlocks: Vec<&'a str>,
}

impl<'a> SaveJson<'a> {
    pub(super) fn to_data(&self, game_data: &GameData) -> SaveData {
        SaveData {
            unlocks: {
                let mut out = crate::UnlockState::None;
                for &unlock in &self.unlocks {
                    out = out.max(match unlock {
                        "cases" => crate::UnlockState::CaseTree,
                        "lemmas" => crate::UnlockState::Lemmas,
                        _ => crate::UnlockState::None,
                    })
                }
                out
            },
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
            unlocks: match self.unlocks {
                crate::UnlockState::None => vec![],
                crate::UnlockState::CaseTree => vec!["cases"],
                crate::UnlockState::Lemmas => vec!["cases", "lemmas"],
            },
        }
    }
}
