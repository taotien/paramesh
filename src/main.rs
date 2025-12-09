use std::path::PathBuf;

use clap::Parser;
use paramesh::microcad::{generate, Microcad};
use rand::prelude::*;
use rerun::{demo_util::grid, external::glam};

#[derive(Parser)]
struct Args {
    // input_path: PathBuf,
    // primitives: Vec<String>,
    // operations: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rng = rand::rng();

    let mut target = Microcad::new();

    let tgt_ucad = generate::from(
        (0..10).map(|_| rng.random_range(0..=2)).collect(),
        (0..10)
            .flat_map(|_| {
                [
                    (0..9).map(|_| rng.random_range(1i8..=20i8)).collect(),
                    vec![rng.random_range(-1i8..0i8)],
                ]
                .into_iter()
                .flatten()
            })
            .collect(),
    )?;

    println!("{tgt_ucad}");
    target.set_root(&tgt_ucad);

    let triags = target.render_mesh()?;

    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer").spawn()?;
    let points = rerun::Points3D::new(triags.positions.iter().map(|v| glam::vec3(v.x, v.y, v.z)));

    rec.log("mesh", &points.with_radii([0.1]))?;

    Ok(())
}
