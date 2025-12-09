use std::{path::PathBuf, process::exit, thread::sleep, time::Duration};

use clap::Parser;
use glam::Vec3;
use paramesh::microcad::{generate, Microcad};
use rand::{
    distr::{weighted::WeightedIndex, Uniform},
    prelude::*,
};
use reedline::{DefaultPrompt, Reedline, Signal};
use rerun::{external::glam, RecordingStream};

#[derive(Parser)]
struct Args {
    // input_path: PathBuf,
    // primitives: Vec<String>,
    // operations: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer").spawn()?;

    let mut rng = rand::rng();

    let count = 5;
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
    let triags = target.render_mesh()?;
    let target_positions = triags.positions.iter().map(|v| glam::vec3(v.x, v.y, v.z));
    let target_mesh: Vec<Vec3> = target_positions.collect();
    let points = rerun::Points3D::new(target_mesh.clone());
    rec.log("mesh", &points.with_radii([0.1]))?;

    println!("initial");
    sleep(Duration::from_secs(10));

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt {
        left_prompt: reedline::DefaultPromptSegment::Basic("paramesh".into()),
        right_prompt: reedline::DefaultPromptSegment::Empty,
    };

    let mut whole: Vec<(u8, [i8; 10])> = Vec::new();
    let mut candidates: Vec<(u8, [i8; 10], f32)> = Vec::new();
    for _ in 0..100 {
        let kind = rng.random_range(0..=2u8);
        let mut params: [i8; 10] = [0; 10];
        for p in params.iter_mut().take(9) {
            *p = rng.random_range(1..=20);
        }
        params[9] = rng.random_range(0..=1);

        let mut trial = whole.clone();
        trial.push((kind, params));
        let (kinds, paramss): (Vec<u8>, Vec<[i8; 10]>) = trial.into_iter().unzip();
        let paramss = paramss.into_iter().flatten().collect();
        let score = score_k_p(&rec, &target_mesh, kinds, paramss)?;
        candidates.push((kind, params, score));
    }
    candidates.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());
    let (a, b, _) = candidates.first().unwrap();
    whole.push((*a, *b));
    loop {
        let mut candidates: Vec<(u8, [i8; 10], f32)> = Vec::new();

        for _ in 0..500 {
            let kind = rng.random_range(0..=2u8);
            let mut params: [i8; 10] = [0; 10];
            for p in params.iter_mut().take(9) {
                *p = rng.random_range(1..=20);
            }
            params[9] = rng.random_range(0..=1);

            let mut trial = whole.clone();
            trial.push((kind, params));
            let (kinds, paramss): (Vec<u8>, Vec<[i8; 10]>) = trial.into_iter().unzip();
            let paramss = paramss.into_iter().flatten().collect();
            let score = score_k_p(&rec, &target_mesh, kinds, paramss)?;
            candidates.push((kind, params, score));
        }

        let (k, p): (Vec<u8>, Vec<[i8; 10]>) = whole.clone().into_iter().unzip();
        let p = p.into_iter().flatten().collect();
        let curr_score = score_k_p(&rec, &target_mesh, k, p)?;
        println!("{curr_score}");
        let weights: Vec<f32> = candidates
            .iter()
            .map(|(_, _, s)| (curr_score - s).max(0.01))
            .collect();

        let dist = WeightedIndex::new(&weights).unwrap();
        let idx = dist.sample(&mut rng);

        let (kind, params, schore) = &candidates[idx];
        whole.push((*kind, *params));

        println!("whole: {whole:?}, diff: {schore}");
    }

    // Ok(())
}

fn score_k_p(
    rec: &RecordingStream,
    target_mesh: &Vec<Vec3>,
    kinds: Vec<u8>,
    params: Vec<i8>,
) -> Result<f32, anyhow::Error> {
    let ucad = generate::from(kinds.clone(), params.clone())?;
    let mut tgt = Microcad::new();
    tgt.set_root(&ucad);
    let mesh: Vec<Vec3> = tgt
        .render_mesh()?
        .positions
        .iter()
        .map(|v| glam::vec3(v.x, v.y, v.z))
        .collect();
    let points = rerun::Points3D::new(mesh.clone());
    rec.log("mesh", &points.with_radii([0.1]))?;
    let score = chamfer_distance(target_mesh, &mesh);
    Ok(score)
}

// #[derive(Default)]
// struct State {
//     objs: Vec<(u8, [i8; 10])>,
//     dist: f32,
// }

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
