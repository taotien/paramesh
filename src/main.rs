use std::{path::PathBuf, process::exit};

use clap::Parser;
use glam::Vec3;
use paramesh::microcad::{generate, Microcad};
use rand::prelude::*;
use reedline::{DefaultPrompt, Reedline, Signal};
use rerun::{demo_util::grid, external::glam};

#[derive(Parser)]
struct Args {
    // input_path: PathBuf,
    // primitives: Vec<String>,
    // operations: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt {
        left_prompt: reedline::DefaultPromptSegment::Basic("paramesh".into()),
        right_prompt: reedline::DefaultPromptSegment::Empty,
    };
    println!("ctrl+c or ctrl+d to exit");
    println!("input amount (0-255) of objects to generate:");
    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(input)) => {
                if let Ok(count) = input.parse() {
                    let target = random_target(count)?;
                    rerun_target(target)?;
                    break;
                } else {
                    println!("not a u8, try again");
                }
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("exit");
                exit(0);
            }
            e => println!("bad input: {e:?}"),
        }
    }
    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(input)) => match input {
                _ => {}
            },
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("exit");
                exit(0);
            }
            // e => println!("dunno: {e:?}",),
            e => println!("bad input: {e:?}"),
        }
    }
}

fn rerun_target(mut target: Microcad) -> Result<(), anyhow::Error> {
    let triags = target.render_mesh()?;
    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer").spawn()?;
    let positions = triags.positions.iter().map(|v| glam::vec3(v.x, v.y, v.z));
    let points = rerun::Points3D::new(positions);
    println!("rendered target mesh");
    rec.log("mesh", &points.with_radii([0.1]))?;
    Ok(())
}

fn random_target(count: u8) -> Result<Microcad, anyhow::Error> {
    let mut rng = rand::rng();
    let mut target = Microcad::new();
    let tgt_ucad = generate::from(
        (0..count).map(|_| rng.random_range(0..=2)).collect(),
        (0..count)
            .flat_map(|_| {
                [
                    (0..9).map(|_| rng.random_range(1i8..=20i8)).collect(),
                    vec![rng.random_range(0..=1i8)],
                ]
                .into_iter()
                .flatten()
            })
            .collect(),
    )?;
    println!("{tgt_ucad}");
    target.set_root(&tgt_ucad);
    Ok(target)
}

pub fn chamfer_distance(a: &[Vec3], b: &[Vec3]) -> f32 {
    fn nearest_sum(from: &[Vec3], to: &[Vec3]) -> f32 {
        let mut accum = 0.0;
        for p in from {
            let mut best = f32::INFINITY;
            for q in to {
                let d = (*p - *q).length_squared();
                if d < best {
                    best = d;
                }
            }
            accum += best;
        }
        accum / from.len() as f32
    }

    nearest_sum(a, b) + nearest_sum(b, a)
}
