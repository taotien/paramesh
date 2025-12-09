use paramesh::microcad::{generate, Microcad};
use rand::prelude::*;
use rerun::external::{glam, glam::Vec3};
use std::collections::VecDeque;

#[derive(Clone, Debug, Copy)]
enum Elem {
    Filled(u8, [i8; 10]),
    Hole,
}

type Sketch = Vec<Elem>;

#[derive(Clone, Debug)]
struct Constraint {
    residual_points: Vec<[f32; 3]>,
}

type Program = Vec<(u8, [i8; 10])>;

struct Cegis {
    sketch: Sketch,
    constraints: Vec<Constraint>,
    target: Vec<Vec3>,
}

impl Cegis {
    fn new() -> Self {
        let mut rng = rand::rng();
        let mut target = Microcad::new();
        let r = generate_random(&mut rng);
        let kinds = vec![r.0];
        let params = r.1;
        let target = params_to_glam(&kinds, &params);

        Self {
            sketch: Vec::new(),
            constraints: Vec::new(),
            target,
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

    fn propose_candidate_for_hole(&self) -> (u8, [i8; 10]) {
        let mut rng = rand::rng();
        let kind = rng.random_range(0..=2u8);
        let mut params = [0i8; 10];
        for i in 0..9 {
            params[i] = rng.random_range(1..=20);
        }
        params[9] = rng.random_range(0..=1);
        (kind, params)
    }

    fn compute_counterexamples(&self, program: &Program) -> Vec<Constraint> {
        // Placeholder: you should implement distance-based residuals or feature-based
        // For example: collect points in target_mesh not covered by program
        let mut residuals = Vec::new();
        // pseudo-code:
        // for vertex in target_mesh:
        //     if vertex not covered by program:
        //         residuals.push(vertex)
        residuals
    }

    /// Score a full program
    fn score_program(&self, program: &Program) -> f32 {
        // Wrap your score_k_p function here
        let (kinds, params): (Vec<u8>, Vec<[i8; 10]>) = program.iter().cloned().unzip();
        let flat_params: Vec<i8> = params.into_iter().flatten().collect();
        let b = params_to_glam(&kinds, &flat_params);

        chamfer_distance(&self.target, &b)
    }

    /// Main CEGIS loop
    fn run(&mut self, max_primitives: usize, max_attempts_per_hole: usize) -> Program {
        while self.sketch.len() < max_primitives {
            self.sketch.push(Elem::Hole);

            let mut solved = false;
            for _ in 0..max_attempts_per_hole {
                let program = self.fill_holes();
                let score = self.score_program(&program);
                let counterexamples = self.compute_counterexamples(&program);

                if counterexamples.is_empty() {
                    // hole is satisfied
                    solved = true;
                    break;
                } else {
                    // store counterexamples as constraints
                    self.constraints.extend(counterexamples);
                    // refine the last hole
                    if let Some(Elem::Hole) = self.sketch.last() {
                        let new_candidate = self.propose_candidate_for_hole();
                        let var_name = self.sketch.len() - 1;
                        self.sketch[var_name] = Elem::Filled(new_candidate.0, new_candidate.1);
                    }
                }
            }

            // fallback: commit last candidate even if constraints not fully satisfied
            if !solved {
                if let Some(Elem::Hole) = self.sketch.last() {
                    let candidate = self.propose_candidate_for_hole();
                    let var_name = self.sketch.len() - 1;
                    self.sketch[var_name] = Elem::Filled(candidate.0, candidate.1);
                }
            }
        }

        // return final filled program
        self.sketch
            .iter()
            .map(|elem| match elem {
                Elem::Filled(kind, params) => (*kind, *params),
                Elem::Hole => panic!("Hole remaining at end!"),
            })
            .collect()
    }
}

// Dummy scoring function placeholder
// fn score_k_p(target_mesh: &MeshType, kinds: Vec<u8>, flat_params: Vec<i8>) -> Option<f32> {
//     // Replace with your actual scoring
//     Some(0.0)
// }

fn main() {
    let mut cegis = Cegis::new();
    let mut rng = rand::rng();

    cegis.constraints = Vec::new();
    cegis.sketch = vec![];

    let final_program = cegis.run(5, 100);

    println!("Synthesized program: {:?}", final_program);
}

fn params_to_glam(kinds: &[u8], params: &[i8]) -> Vec<Vec3> {
    let ucad = generate::ucad(kinds, params).unwrap();
    let mut target = Microcad::new();
    target.set_root(&ucad);
    let triags = target.render_mesh().unwrap();
    let pos = triags
        .positions
        .iter()
        .map(|v| glam::vec3(v.x, v.y, v.z))
        .collect();
    // let triags = target.render_mesh()?;
    // let target_positions = triags.positions.iter().map(|v| glam::vec3(v.x, v.y, v.z));
    // let target_mesh: Vec<Vec3> = target_positions.collect();
    // let points = rerun::Points3D::new(target_mesh.clone());
    // rec.log("mesh", &points.with_radii([0.1]))?;
    pos
}

fn chamfer_distance(a: &[Vec3], b: &[Vec3]) -> f32 {
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

fn generate_random(rng: &mut ThreadRng) -> (u8, [i8; 10]) {
    let kind = rng.random_range(0..=2);
    let mut params = [0i8; 10];
    for p in params.iter_mut().take(9) {
        *p = rng.random_range(1i8..=20i8)
    }
    params[9] = 0i8;

    (kind, params)
}
