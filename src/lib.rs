#![feature(slice_split_once)]

use pyo3::prelude::*;
use rerun::external::glam::{self, Vec3};

use crate::microcad::{generate, Microcad};

pub mod microcad;

#[pyfunction]
fn visualize(kinds: Vec<u8>, params: Vec<f32>) -> PyResult<()> {
    let rec = rerun::RecordingStreamBuilder::new("microcad synthesizer")
        .spawn()
        .unwrap();
    let ucad = generate::from(kinds, params.into_iter().map(|i| i as i8).collect()).unwrap();
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
    m.add_function(wrap_pyfunction!(visualize, m)?)?;
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
