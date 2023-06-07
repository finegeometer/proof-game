use crate::level::case::{Case, ValidityReason, Wire};
use crate::level::expression::Expression;

use super::*;
use ::serde::Deserialize;
use anyhow::*;
use serde::Deserializer;
use smallvec::SmallVec;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct GameJson<'a>(#[serde(borrow)] Vec<LevelJson<'a>>);

impl<'a> TryFrom<GameJson<'a>> for GameData {
    type Error = Error;

    fn try_from(json: GameJson<'a>) -> Result<Self> {
        Ok(GameData {
            levels: json
                .0
                .into_iter()
                .map(Level::try_from)
                .collect::<Result<_, _>>()?,
        })
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
    prereqs: Vec<usize>,
    #[serde(default, deserialize_with = "deserialize_some")]
    next_level: Option<Option<usize>>,
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

impl<'a> TryFrom<LevelJson<'a>> for Level {
    type Error = Error;

    fn try_from(level: LevelJson) -> Result<Self> {
        let mut x_min = f64::INFINITY;
        let mut y_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        let mut case = Case::new();
        let mut wires = Vec::with_capacity(level.nodes.len());
        for (op, inputs, position) in &level.nodes {
            x_min = x_min.min(position[0]);
            y_min = y_min.min(position[1]);
            x_max = x_max.max(position[0]);
            y_max = y_max.max(position[1]);

            let inputs = inputs.iter().map(|idx| {
                wires.get(*idx).copied().ok_or(anyhow!(
                    "Node {} depends on later node {}",
                    wires.len(),
                    idx
                ))
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
        for idx in level.hypotheses.iter() {
            case.set_proven(
                *wires.get(*idx).ok_or(anyhow!(
                    "Hypothesis index too large. ({} >= {})",
                    idx,
                    wires.len()
                ))?,
                ValidityReason::new("By assumption."),
            )
        }
        case.set_goal(*wires.get(level.conclusion).ok_or(anyhow!(
            "Conclusion index too large. ({} >= {})",
            level.conclusion,
            wires.len()
        ))?);
        Ok(Level {
            case,
            pan_zoom: crate::render::PanZoom {
                svg_corners: ([x_min - 1., y_min - 1.], [x_max + 1., y_max + 3.]),
            },
            text_box: level.text_box.map(|s| s.to_owned()),
            map_position: level.map_position,
            bezier_vector: level.bezier_vector,
            prereqs: level.prereqs,
            next_level: level
                .next_level
                .ok_or(anyhow!("Missing next_level field."))?,
        })
    }
}
