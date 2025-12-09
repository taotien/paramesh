#![feature(slice_split_once)]
// use pyo3::prelude::*;

pub mod microcad;

// /// Formats the sum of two numbers as string.
// #[pyfunction]
// fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
//     Ok((a + b).to_string())
// }

// /// A Python module implemented in Rust.
// #[pymodule]
// fn paramesh(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
//     Ok(())
// }

enum Primitive {
    Cube(u8, u8, u8),
    Sphere(u8),
    Cylinder(u8, u8),
}

enum Operation {
    Translate(u8, u8, u8),
    Rotate(u8, u8, u8),
}
