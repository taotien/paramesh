use std::{collections::BTreeMap, path::PathBuf, process::exit, thread::sleep, time::Duration};

use clap::Parser;
use glam::Vec3;
use itertools::iproduct;
use paramesh::{
    chamfer_distance, generate_random,
    microcad::{generate, Microcad},
    params_to_glam, visualize,
};
use rand::{
    distr::{weighted::WeightedIndex, Uniform},
    prelude::*,
};
use reedline::{DefaultPrompt, Reedline, Signal};
use rerun::{external::glam, RecordingStream};

enum E {
    Filled(u8, [i8; 10]),
    Hole,
}

fn main() -> anyhow::Result<()> {
    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer").spawn()?;

    let mut rng = rand::rng();

    let count = 5;
    let mut target = Microcad::new();
    let (kinds, params): (Vec<u8>, Vec<[f32; 10]>) =
        (0..2).map(|_| generate_random(&mut rng)).collect();
    println!("target: {kinds:?}, {params:?}");
    let params = params.into_iter().flatten().collect::<Vec<_>>();
    let tgt_ucad = generate::ucad(&kinds, &params)?;
    println!("{tgt_ucad}");
    target.set_root(&tgt_ucad);
    let triags = target.render_mesh()?;
    let target_positions = triags.positions.iter().map(|v| glam::vec3(v.x, v.y, v.z));
    let target_mesh: Vec<Vec3> = target_positions.collect();
    let points = rerun::Points3D::new(target_mesh.clone());
    rec.log("mesh", &points.with_radii([0.1]))?;

    println!("initial");
    // sleep(Duration::from_secs(10));

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt {
        left_prompt: reedline::DefaultPromptSegment::Basic("paramesh".into()),
        right_prompt: reedline::DefaultPromptSegment::Empty,
    };

    let mut built_kinds = vec![];
    let mut built_params = vec![];
    let mut best = (0u8, [0.0; 10], 0.0);
    let mut next = None;
    let mut size_range = 1u16..=20u16;
    let mut tran_range = 1u16..=5u16;
    let mut rota_range = 0u16..=360u16;
    loop {
        let input = line_editor.read_line(&prompt)?;
        match input {
            Signal::Success(input) => {
                let mut it = input.chars();
                match (it.next(), it.next()) {
                    (Some('k'), Some(k)) => match k {
                        '0' => next = Some(0),
                        '1' => next = Some(1),
                        '2' => next = Some(2),
                        _ => {}
                    },
                    (Some('s'), _) => {
                        let collect = it.collect::<String>();
                        let (low, high) = collect.split_once('-').unwrap();
                        let low = low.parse().unwrap();
                        let high = high.parse().unwrap();
                        size_range = low..=high;
                    }
                    (Some('t'), _) => {
                        let collect = it.collect::<String>();
                        let (low, high) = collect.split_once('-').unwrap();
                        let low = low.parse().unwrap();
                        let high = high.parse().unwrap();
                        tran_range = low..=high;
                    }
                    (Some('r'), _) => {
                        let collect = it.collect::<String>();
                        let (low, high) = collect.split_once('-').unwrap();
                        let low = low.parse().unwrap();
                        let high = high.parse().unwrap();
                        rota_range = low..=high;
                    }
                    (Some('c'), _) => {
                        built_kinds.push(best.0);
                        built_params.push(best.1);
                    }
                    (Some('a'), _) => {
                        next = None;
                        size_range = 1u16..=20u16;
                        tran_range = 1u16..=5u16;
                        rota_range = 0u16..=360u16;
                    }
                    _ => {}
                }
            }
            Signal::CtrlC | Signal::CtrlD => exit(1),
        }

        let mut best_candi = (f32::MAX, (Vec::new(), (0, [0f32; 10])));

        for (kind, sx, sy, sz, tx, ty, tz, rx, ry, rz) in iproduct!(
            0..=2,
            size_range.clone().step_by(10),
            size_range.clone().step_by(10),
            size_range.clone().step_by(10),
            tran_range.clone().step_by(2),
            tran_range.clone().step_by(2),
            tran_range.clone().step_by(2),
            rota_range.clone().step_by(90),
            rota_range.clone().step_by(90),
            rota_range.clone().step_by(90),
        ) {
            let kind = {
                if let Some(n) = next {
                    n
                } else {
                    kind
                }
            };

            let ps = [sx, sy, sz, tx, ty, tz, rx, ry, rz, 0].map(|p| p as f32);
            println!("{ps:?}");

            let mut kinds = built_kinds.clone();
            kinds.push(kind);
            let mut params = built_params.clone();
            params.push(ps);
            let params = params.into_iter().flatten().collect::<Vec<_>>();

            let glam = params_to_glam(&kinds, &params);

            let score = chamfer_distance(&target_mesh, &glam);
            visualize(glam.clone(), &rec);

            if score <= best_candi.0 {
                best_candi = (score, (glam, (kind, ps)));
            }
        }
        let (score, (glam, (kind, ps))) = best_candi;
        best = (kind, ps, score);
        visualize(glam.clone(), &rec);
    }
}
