#![feature(slice_split_once)]

use pyo3::prelude::*;
use rand::prelude::*;
use rerun::{
    external::glam::{self, Vec3},
    RecordingStream,
};

use crate::microcad::{generate, Microcad};

pub mod microcad;

#[pyfunction]
fn pyvisualize(kinds: Vec<u8>, params: Vec<f32>) -> PyResult<()> {
    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer")
        .spawn()
        .unwrap();
    let ucad = generate::ucad(&kinds, &params).unwrap();
    let mut tgt = Microcad::new();
    tgt.set_root(&ucad);
    let mesh: Vec<Vec3> = tgt
        .render_mesh()
        .unwrap()
        .positions
        .iter()
        .map(|v| glam::vec3(v.x, v.y, v.z))
        .collect();
    let points = rerun::Points3D::new(mesh.clone());
    rec.log("mesh", &points.with_radii([0.1])).unwrap();

    Ok(())
}

#[pymodule]
fn paramesh(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(pyvisualize, m)?)?;
    Ok(())
}

enum Primitive {
    Cube(u8, u8, u8),
    Sphere(u8),
    Cylinder(u8, u8),
}

enum Operation {
    Translate(u8, u8, u8),
    Rotate(u8, u8, u8),
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

pub fn generate_random(rng: &mut ThreadRng) -> (u8, [f32; 10]) {
    let kind = rng.random_range(0..=2);
    let mut params = [0f32; 10];
    for p in params.iter_mut().take(3) {
        *p = rng.random_range(1f32..=20f32)
    }
    for p in params.iter_mut().skip(3).take(3) {
        *p = rng.random_range(0f32..=5f32)
    }
    for p in params.iter_mut().skip(6).take(3) {
        *p = rng.random_range(0f32..=360f32)
    }
    params[9] = 0f32;

    (kind, params)
}

pub fn visualize(target: Vec<Vec3>, rec: &RecordingStream) {
    let points = rerun::Points3D::new(target);
    rec.log("candidate", &points.with_radii([0.1])).unwrap();
}

pub fn params_to_glam(kinds: &[u8], params: &[f32]) -> Vec<Vec3> {
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
