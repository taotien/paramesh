use std::{path::PathBuf, rc::Rc};

use anyhow::anyhow;
use microcad_builtin::*;
use microcad_core::{RenderResolution, Transformed3D, TriangleMesh};
use microcad_lang::{
    diag::{Diag, DiagHandler},
    eval::{EvalContext, Stdout},
    rc::RcMut,
    render::{GeometryOutput, RenderCache, RenderContext, RenderWithContext},
    resolve::ResolveContext,
    syntax::SourceFile,
};

pub mod generate;

pub struct Microcad {
    lib_paths: Vec<PathBuf>,
    render_cache: RcMut<RenderCache>,
    root: Rc<SourceFile>,
}

impl Default for Microcad {
    fn default() -> Self {
        Self::new()
    }
}

const PRELUDE: &str = concat!(
    "use std::geo3d::*;\n",
    "use std::ops::*;\n",
    // "use std::math::*;\n"
);

impl Microcad {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir().map(|d| d.join("lib")).unwrap();
        let lib_paths = vec![config_dir];

        let render_cache = RcMut::new(RenderCache::default());

        let root = SourceFile::load_from_str(None, "tmp", "").unwrap();

        Self {
            lib_paths,
            render_cache,
            root,
        }
    }

    pub fn set_root(&mut self, new: &str) {
        let mut n = String::from(PRELUDE);
        n.push_str(new);
        self.root = SourceFile::load_from_str(None, "tmp", &n).unwrap();
    }

    pub fn render_mesh(&mut self) -> anyhow::Result<TriangleMesh> {
        let res_ctx = ResolveContext::create(
            self.root.clone(),
            &self.lib_paths,
            Some(builtin_module()),
            DiagHandler::default(),
        )?;
        let mut eval_ctx = EvalContext::new(
            res_ctx,
            Stdout::new(),
            builtin_exporters(),
            builtin_importers(),
        );

        let result = eval_ctx.eval();

        if eval_ctx.has_errors() {
            eprintln!("{}", eval_ctx.diagnosis())
        }

        let model = {
            match result {
                Ok(Some(model)) => model,
                // Err(e) => Err(anyhow!("model step fail: {e}"))?,
                _ => Err(anyhow!("model step fail"))?,
            }
        };

        let mut rdr_ctx = RenderContext::new(
            &model,
            RenderResolution { linear: 0.5 },
            Some(self.render_cache.clone()),
            None,
        )?;

        let model = &<microcad_lang::model::Model as RenderWithContext<
            microcad_lang::model::Model,
        >>::render_with_context(&model, &mut rdr_ctx)?;

        let model = model.borrow();
        let output = model.output();
        let geometry = &output.geometry;
        let matrix = output.world_matrix.unwrap();

        match geometry {
            Some(GeometryOutput::Geometry3D(geometry)) => {
                let geometry = geometry.transformed_3d(&matrix);
                match geometry.inner {
                    microcad_core::Geometry3D::Mesh(triangles) => Ok(triangles),
                    _ => Err(anyhow!("geometry inner step fail"))?,
                }
            }
            _ => Err(anyhow!("geometry step fail"))?,
        }
    }
}
