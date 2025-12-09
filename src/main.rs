use std::{path::PathBuf, process::exit, thread::sleep, time::Duration};

use clap::Parser;
use glam::Vec3;
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
    let mut size_range = (1f32..=20f32);
    let mut tran_range = (1f32..=5f32);
    let mut rota_range = (0f32..=360f32);
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
                        size_range = (1f32..=20f32);
                        tran_range = (1f32..=5f32);
                        rota_range = (0f32..=360f32);
                    }
                    _ => {}
                }
            }
            Signal::CtrlC | Signal::CtrlD => exit(1),
        }

        let mut candidates = vec![];
        for _ in 0..100 {
            let kind = {
                if let Some(n) = next {
                    n
                } else {
                    rng.random_range(0..=2u8)
                }
            };

            let mut ps = [0f32; 10];
            for p in ps.iter_mut().take(3) {
                *p = rng.random_range(size_range.clone())
            }
            for p in ps.iter_mut().skip(3).take(3) {
                *p = rng.random_range(tran_range.clone())
            }
            for p in ps.iter_mut().skip(6).take(3) {
                *p = rng.random_range(rota_range.clone())
            }
            ps[9] = rng.random_range(0f32..=1f32);

            let mut kinds = built_kinds.clone();
            kinds.push(kind);
            let mut params = built_params.clone();
            params.push(ps);
            let params = params.into_iter().flatten().collect::<Vec<_>>();
            let glam = params_to_glam(&kinds, &params);

            let score = chamfer_distance(&target_mesh, &glam);
            candidates.push((glam.clone(), (kind, ps, score)));
            visualize(glam, &rec);
        }
        candidates.sort_by(|(_, (_, _, s1)), (_, (_, _, s2))| s1.total_cmp(s2));
        let (glam, b) = candidates.last().unwrap();
        println!("{b:?}");
        best = *b;
        visualize(glam.clone(), &rec);
    }

    // let mut whole: Vec<(u8, [i8; 10])> = Vec::new();
    // let mut candidates: Vec<(u8, [i8; 10], f32)> = Vec::new();
    // for _ in 0..100 {
    //     let kind = rng.random_range(0..=2u8);
    //     let mut params: [i8; 10] = [0; 10];
    //     for p in params.iter_mut().take(9) {
    //         *p = rng.random_range(1..=20);
    //     }
    //     params[9] = rng.random_range(0..=1);

    //     let mut trial = whole.clone();
    //     trial.push((kind, params));
    //     let (kinds, paramss): (Vec<u8>, Vec<[i8; 10]>) = trial.into_iter().unzip();
    //     let paramss = paramss.into_iter().flatten().collect();
    //     let score = score_k_p(&rec, &target_mesh, kinds, paramss)?;
    //     candidates.push((kind, params, score));
    // }
    // candidates.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());
    // let (a, b, _) = candidates.first().unwrap();
    // whole.push((*a, *b));
    // loop {
    //     let mut candidates: Vec<(u8, [i8; 10], f32)> = Vec::new();

    //     for _ in 0..500 {
    //         let kind = rng.random_range(0..=2u8);
    //         let mut params: [i8; 10] = [0; 10];
    //         for p in params.iter_mut().take(9) {
    //             *p = rng.random_range(1..=20);
    //         }
    //         params[9] = rng.random_range(0..=1);

    //         let mut trial = whole.clone();
    //         trial.push((kind, params));
    //         let (kinds, paramss): (Vec<u8>, Vec<[i8; 10]>) = trial.into_iter().unzip();
    //         let paramss = paramss.into_iter().flatten().collect();
    //         let score = score_k_p(&rec, &target_mesh, kinds, paramss)?;
    //         candidates.push((kind, params, score));
    //     }

    //     let (k, p): (Vec<u8>, Vec<[i8; 10]>) = whole.clone().into_iter().unzip();
    //     let p = p.into_iter().flatten().collect();
    //     let curr_score = score_k_p(&rec, &target_mesh, k, p)?;
    //     println!("{curr_score}");
    //     let weights: Vec<f32> = candidates
    //         .iter()
    //         .map(|(_, _, s)| (curr_score - s).max(0.01))
    //         .collect();

    //     let dist = WeightedIndex::new(&weights).unwrap();
    //     let idx = dist.sample(&mut rng);

    //     let (kind, params, schore) = &candidates[idx];
    //     whole.push((*kind, *params));

    //     println!("whole: {whole:?}, diff: {schore}");
    // }

    // // Ok(())
}
