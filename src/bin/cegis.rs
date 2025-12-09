use paramesh::{
    chamfer_distance, generate_random,
    microcad::{generate, Microcad},
};
use rand::prelude::*;
use rerun::{
    external::glam::{self, Vec3},
    RecordingStream,
};
use std::collections::VecDeque;

/// Compute centroid of a set of 3D points
fn compute_centroid(points: &[[f32; 3]]) -> [f32; 3] {
    let n = points.len().max(1) as f32;
    let mut centroid = [0.0; 3];
    for p in points {
        centroid[0] += p[0];
        centroid[1] += p[1];
        centroid[2] += p[2];
    }
    centroid[0] /= n;
    centroid[1] /= n;
    centroid[2] /= n;
    centroid
}

/// Compute axis-aligned bounding box sizes
fn compute_bbox_sizes(points: &[[f32; 3]]) -> [f32; 3] {
    if points.is_empty() {
        return [1.0, 1.0, 1.0];
    }
    let mut min_p = points[0];
    let mut max_p = points[0];
    for p in points.iter().skip(1) {
        for i in 0..3 {
            min_p[i] = min_p[i].min(p[i]);
            max_p[i] = max_p[i].max(p[i]);
        }
    }
    [
        max_p[0] - min_p[0],
        max_p[1] - min_p[1],
        max_p[2] - min_p[2],
    ]
}

/// Heuristic for choosing primitive type based on bounding box
fn shape_heuristic(bbox_sizes: [f32; 3]) -> u8 {
    let max_dim = bbox_sizes.iter().cloned().fold(f32::NAN, f32::max);
    let min_dim = bbox_sizes.iter().cloned().fold(f32::NAN, f32::min);

    if (max_dim - min_dim) < 2.0 {
        1 // roughly spherical
    } else if bbox_sizes[2] > bbox_sizes[0].max(bbox_sizes[1]) {
        2 // elongated → cylinder
    } else {
        0 // cube
    }
}

/// Convert f32 to f32 for your parameter array
fn f32_to_f32_clamped(x: f32) -> f32 {
    x.round().clamp(1.0, 20.0) as f32
}

#[derive(Clone, Debug, Copy)]
enum Elem {
    Filled(u8, [f32; 10]),
    Hole,
}

type Sketch = Vec<Elem>;

#[derive(Clone, Debug)]
struct Constraint {
    residual_points: Vec<[f32; 3]>,
}

type Program = Vec<(u8, [f32; 10])>;

struct Cegis {
    sketch: Sketch,
    constraints: Vec<Constraint>,
    target: Vec<Vec3>,
    rec: RecordingStream,
}

fn refine_once(
    kind: u8,
    mut params: [f32; 10],
    score_fn: &impl Fn(u8, [f32; 10]) -> f32,
) -> (u8, [f32; 10], f32) {
    let mut best = score_fn(kind, params);

    let steps = [8., 4., 2., 1.];

    for &step in &steps {
        let mut improved = true;

        while improved {
            improved = false;

            for i in 0..10 {
                let old = params[i];

                let plus = old + step;
                params[i] = plus;
                let score_plus = score_fn(kind, params);

                let minus = old + step;
                params[i] = minus;
                let score_minus = score_fn(kind, params);

                let (best_dir, new_score) = if score_plus < best && score_plus <= score_minus {
                    (1, score_plus)
                } else if score_minus < best {
                    (-1, score_minus)
                } else {
                    (0, best)
                };

                match best_dir {
                    1 => {
                        params[i] = plus;
                        best = new_score;
                        improved = true;
                    }
                    -1 => {
                        params[i] = minus;
                        best = new_score;
                        improved = true;
                    }
                    _ => params[i] = old,
                };
            }
        }
    }

    (kind, params, best)
}

pub fn local_search(
    mut kind: u8,
    mut params: [f32; 10],
    score_fn: &impl Fn(u8, [f32; 10]) -> f32,
    rng: &mut impl Rng,
    restarts: usize,
) -> (u8, [f32; 10], f32) {
    let (mut best_kind, mut best_params, mut best_score) = refine_once(kind, params, score_fn);

    for _ in 0..restarts {
        let k = best_kind;
        let mut p = best_params;

        match k {
            0 => {
                for i in 0..3 {
                    p[i] = p[i] + rng.random_range(-4.0..=4.0);
                }
            }
            1 => {
                p[0] = p[0] + rng.random_range(-4.0..=4.0);
            }
            2 => {
                p[0] = p[0] + rng.random_range(-4.0..=4.0);
                p[1] = p[1] + rng.random_range(-4.0..=4.0);
            }
            _ => {}
        }

        for i in 3..6 {
            p[i] = p[i] + rng.random_range(-4.0..=4.);
        }

        for i in 6..9 {
            p[i] = p[i] + rng.random_range(-3.0..=3.);
        }

        let (_, refined_params, refined_score) = refine_once(k, p, score_fn);

        if refined_score < best_score {
            best_params = refined_params;
            best_score = refined_score;
        }
    }

    (best_kind, best_params, best_score)
}

impl Cegis {
    fn new() -> Self {
        let mut rng = rand::rng();

        let (kinds, params): (Vec<u8>, Vec<[f32; 10]>) =
            (0..2).map(|_| generate_random(&mut rng)).collect();
        println!("target: {kinds:?}, {params:?}");
        let params = params.into_iter().flatten().collect::<Vec<_>>();
        let target = params_to_glam(&kinds, &params);
        let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer")
            .spawn()
            .unwrap();
        let points = rerun::Points3D::new(target.clone());
        rec.log("target", &points.with_radii([0.1])).unwrap();

        Self {
            sketch: Vec::new(),
            constraints: Vec::new(),
            target,
            rec,
        }
    }

    fn fill_holes(&mut self) -> Program {
        self.sketch
            .iter()
            .map(|elem| match elem {
                Elem::Filled(kind, params) => (*kind, *params),
                Elem::Hole => self.propose_candidate_for_hole(),
            })
            .collect()
    }

    fn propose_candidate_for_hole(&self) -> (u8, [f32; 10]) {
        let mut rng = rand::rng();

        let mut residual_points = Vec::new();
        for c in self.constraints.clone() {
            residual_points.extend(c.residual_points);
        }

        let centroid = compute_centroid(&residual_points);
        let bbox_sizes = compute_bbox_sizes(&residual_points);

        let kind = shape_heuristic(bbox_sizes);

        let mut params = [0f32; 10];

        for i in 0..3 {
            params[i] = f32_to_f32_clamped(bbox_sizes[i] + rng.random_range(-1.0..=1.0));
        }
        for i in 0..3 {
            params[i + 3] = f32_to_f32_clamped(centroid[i] + rng.random_range(-1.0..=1.0));
        }
        for i in 6..9 {
            params[i] = rng.random_range(0.0..=20.0);
        }

        params[9] = rng.random_range(0.0..=1.0);

        (kind, params)
    }

    fn compute_counterexamples(&self, program: &Program) -> Vec<Constraint> {
        // Placeholder: you should implement distance-based residuals or feature-based
        // For example: collect points in target_mesh not covered by program
        let mut residual_points = Vec::new();
        for v in &self.target {
            let mut covered = false;
            for &(_, p) in program {
                let pc = [p[3] as f32, p[4] as f32, p[5] as f32];
                let dist_sq = (v.x - pc[0]).powi(2) + (v.y - pc[1]).powi(2) + (v.z - pc[2]).powi(2);

                if dist_sq.sqrt() <= 5.0 {
                    covered = true;
                    break;
                }
            }

            if !covered {
                residual_points.push([v.x, v.y, v.z]);
            }
        }

        vec![Constraint { residual_points }]
    }

    fn score_program(&self, program: &Program) -> f32 {
        // Wrap your score_k_p function here
        let (kinds, params): (Vec<u8>, Vec<[f32; 10]>) = program.iter().cloned().unzip();
        let flat_params: Vec<f32> = params.into_iter().flatten().collect();
        let b = params_to_glam(&kinds, &flat_params);

        visualize(&b, &self.rec);

        chamfer_distance(&self.target, &b)
    }

    fn run(&mut self, max_primitives: usize, max_attempts_per_hole: usize) -> Program {
        while self.sketch.len() < max_primitives {
            self.sketch.push(Elem::Hole);

            let mut best_score = f32::MAX;
            let mut best_candidate: Option<(u8, [f32; 10])> = None;

            for _ in 0..max_attempts_per_hole {
                let program = self.fill_holes();
                let score = self.score_program(&program);
                if score < best_score {
                    best_score = score;
                    if let Some(Elem::Hole) = self.sketch.last() {
                        let new_candidate = self.propose_candidate_for_hole();
                        best_candidate = Some(new_candidate);
                    }
                }

                let counterexamples = self.compute_counterexamples(&program);
                self.constraints.extend(counterexamples);

                println!("{score}");
                if score <= 10.0 {
                    best_candidate = None; // force fallback fill
                    break;
                }
            }

            let var_index = self.sketch.len() - 1;

            match best_candidate {
                Some((kind, params)) => {
                    self.sketch[var_index] = Elem::Filled(kind, params);
                }
                None => {
                    let (kind, params) = self.propose_candidate_for_hole();
                    self.sketch[var_index] = Elem::Filled(kind, params);
                }
            }
        }

        self.sketch
            .iter()
            .map(|elem| match elem {
                Elem::Filled(k, p) => (*k, *p),
                Elem::Hole => panic!("Hole remaining at end!"),
            })
            .collect()
    }
}

fn main() {
    let mut cegis = Cegis::new();

    cegis.constraints = Vec::new();
    cegis.sketch = vec![];

    let final_program = cegis.run(10, 100);

    let (k, p): (Vec<u8>, Vec<[f32; 10]>) = final_program.into_iter().unzip();
    println!("result: {k:?}, {p:?}");
    // let p = p.into_iter().flatten().collect::<Vec<_>>();
    // let glam = params_to_glam(&k, &p);
    // let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer")
    //     .spawn()
    //     .unwrap();
    // let points = rerun::Points3D::new(glam.clone());
    // rec.log("result", &points.with_radii([0.1])).unwrap();
}

fn params_to_glam(kinds: &[u8], params: &[f32]) -> Vec<Vec3> {
    let ucad = generate::ucad(kinds, params).unwrap();
    let mut target = Microcad::new();
    target.set_root(&ucad);
    let triags = target.render_mesh().unwrap();
    let pos = triags
        .positions
        .iter()
        .map(|v| glam::vec3(v.x, v.y, v.z))
        .collect();
    pos
}
