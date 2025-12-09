use std::io::Write as _;

use anyhow::anyhow;
use rand::prelude::*;

use crate::microcad::PRELUDE;

pub fn from(tokens: Vec<u8>, params: Vec<i8>) -> anyhow::Result<String> {
    // assert_eq!(token.len(), params.len());
    if tokens.len() * 10 != params.len() {
        Err(anyhow!(format!(
            "tokens and parameters length do not match!: {}, {}",
            tokens.len() * 10,
            params.len(),
        )))?
    }

    let mut rng = rand::rngs::SmallRng::from_os_rng();

    // println!("{tokens:?}, {params:?}");
    let mut objs = vec![];
    let mut ucad = vec![];

    writeln!(ucad, "{}", PRELUDE)?;

    for (p, params) in tokens.into_iter().zip(params.chunks_exact(10)) {
        if let [sx, sy, sz, px, py, pz, rx, ry, rz, sig] = params {
            let name: String = (&mut rng)
                .sample_iter(rand::distr::Alphabetic)
                .take(10)
                .map(char::from)
                .collect();
            objs.push((name.clone(), *sig <= 1));

            match p {
                0 => {
                    writeln!(
                        ucad,
                        "{name} = Cube(size_x = {sx}mm, size_y = {sy}mm, size_z = {sz}mm)",
                    )?;
                }
                1 => {
                    writeln!(ucad, "{name} = Sphere({sx}mm)")?;
                }
                2 => {
                    writeln!(ucad, "{name} = Cylinder(d = {sx}mm, h = {sy}mm)")?;
                }
                3 => {
                    if let Some((want, _discard)) = ucad.rsplit_once(|c| *c == b'\n') {
                        ucad = want.into();
                    }
                    continue;
                }
                4 => break,
                _ => Err(anyhow!(format!("invalid token: {p}")))?,
            }
            writeln!(ucad, "\t.translate(x = {px}mm, y = {py}mm, z = {pz}mm)")?;
            writeln!(ucad, "\t.rotate(x = {rx}deg, y = {ry}deg, z = {rz}deg);")?;
        } else {
            Err(anyhow!("incorrect number of parameters"))?
        }
    }

    write!(ucad, "{}", objs[0].0)?;
    for obj in objs.iter().skip(1) {
        if obj.1 {
            write!(ucad, " | ")?;
        } else {
            write!(ucad, " & ")?;
        }
        write!(ucad, "{}", obj.0)?;
    }
    writeln!(ucad, ";")?;

    let var_name = String::from_utf8(ucad)?;
    // println!("{var_name}");
    Ok(var_name)
}
