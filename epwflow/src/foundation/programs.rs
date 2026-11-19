//! 外部程序使用平铺路径和独立 argv；内部保留统一 Program 接口。
use super::{config::Program, paths};
use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

macro_rules! programs {
    ($($name:ident: ($path:ident, $args:ident)),* $(,)?) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(try_from = "Fields", into = "Fields")]
        pub struct Programs {
            pub launcher: Vec<String>,
            pub environment_script: Option<PathBuf>,
            pub pw: Program,
            pub ph: Program,
            pub epw: Program,
            $(pub $name: Option<Program>,)*
        }

        #[derive(Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            launcher: Vec<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            environment_script: Option<PathBuf>,
            pw_x: PathBuf,
            #[serde(default)]
            pw_args: Vec<String>,
            ph_x: PathBuf,
            #[serde(default)]
            ph_args: Vec<String>,
            epw_x: PathBuf,
            #[serde(default)]
            epw_args: Vec<String>,
            $(
                #[serde(default, skip_serializing_if = "Option::is_none")]
                $path: Option<PathBuf>,
                #[serde(default)]
                $args: Vec<String>,
            )*
        }

        impl TryFrom<Fields> for Programs {
            type Error = anyhow::Error;
            fn try_from(f: Fields) -> Result<Self> {
                Ok(Self {
                    launcher: f.launcher,
                    environment_script: f.environment_script,
                    pw: Program { path: f.pw_x, args: f.pw_args },
                    ph: Program { path: f.ph_x, args: f.ph_args },
                    epw: Program { path: f.epw_x, args: f.epw_args },
                    $($name: optional(f.$path, f.$args, stringify!($name))?,)*
                })
            }
        }

        impl From<Programs> for Fields {
            fn from(p: Programs) -> Self {
                Self {
                    launcher: p.launcher,
                    environment_script: p.environment_script,
                    pw_x: p.pw.path, pw_args: p.pw.args,
                    ph_x: p.ph.path, ph_args: p.ph.args,
                    epw_x: p.epw.path, epw_args: p.epw.args,
                    $(
                        $path: p.$name.as_ref().map(|p| p.path.clone()),
                        $args: p.$name.map(|p| p.args).unwrap_or_default(),
                    )*
                }
            }
        }

        impl Programs {
            pub fn get(&self, name: &str) -> Result<&Program> {
                let program = match name {
                    "pw" => Some(&self.pw),
                    "ph" => Some(&self.ph),
                    "epw" => Some(&self.epw),
                    $(stringify!($name) => self.$name.as_ref(),)*
                    _ => bail!("configuration: unknown executable {name}"),
                };
                program.ok_or_else(|| anyhow::anyhow!(
                    "configuration: programs.{name}_x is required for this calculation; configure the executable explicitly"
                ))
            }

            pub fn iter(&self) -> impl Iterator<Item = (&'static str, &Program)> {
                [
                    ("pw", Some(&self.pw)),
                    ("ph", Some(&self.ph)),
                    ("epw", Some(&self.epw)),
                    $((stringify!($name), self.$name.as_ref()),)*
                ].into_iter().filter_map(|(n, p)| p.map(|p| (n, p)))
            }

            pub fn resolve(&mut self, base: &Path) -> Result<()> {
                for (name, program) in [
                    ("pw", Some(&mut self.pw)),
                    ("ph", Some(&mut self.ph)),
                    ("epw", Some(&mut self.epw)),
                    $((stringify!($name), self.$name.as_mut()),)*
                ] {
                    if let Some(p) = program {
                        p.path = paths::existing(base, &p.path, &format!("programs.{name}_x"), true)?;
                    }
                }
                Ok(())
            }
        }
    };
}

fn optional(path: Option<PathBuf>, args: Vec<String>, name: &str) -> Result<Option<Program>> {
    ensure!(path.is_some() || args.is_empty(),
        "configuration: programs.{name}_args requires programs.{name}_x");
    Ok(path.map(|path| Program { path, args }))
}

programs! {
    bands: (bands_x, bands_args),
    dos: (dos_x, dos_args),
    projwfc: (projwfc_x, projwfc_args),
    q2r: (q2r_x, q2r_args),
    matdyn: (matdyn_x, matdyn_args),
    postw90: (postw90_x, postw90_args),
    wannier90: (wannier90_x, wannier90_args),
    pw2wannier90: (pw2wannier90_x, pw2wannier90_args),
}
