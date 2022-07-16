use crate::{AsmArg, Package, Target, DIRS};
use command_ext::{CommandExt, Ext};
use std::{
    error::Error,
    ffi::OsStr,
    fs,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::PathBuf,
};

#[derive(Args)]
pub(crate) struct Components {
    #[clap(long, global = true)]
    spl: bool,
    #[clap(long, global = true)]
    see: bool,
    #[clap(long, global = true)]
    kernel: Option<PathBuf>,
    #[clap(long, global = true)]
    dt: Option<PathBuf>,
}

impl Components {
    pub fn make(&self) -> Result<Target, Box<dyn Error>> {
        let mut ans = Target::default();
        if self.spl {
            ans.spl.replace(fs::File::open(Package::Spl.objcopy())?);
        }
        if self.see {
            ans.see.replace(fs::File::open(Package::See.objcopy())?);
        }
        if let Some(kernel) = &self.kernel {
            ans.kernel.replace(fs::File::open(kernel)?);
        }
        if let Some(dt) = &self.dt {
            if !dt.is_file() {
                return Err(IoError::new(
                    IoErrorKind::NotFound,
                    format!("dt file \"{}\" not exist", dt.display()),
                )
                .into());
            }
            if dt.extension() == Some(OsStr::new("dts")) {
                let dtb = DIRS
                    .target
                    .join(dt.file_stem().unwrap_or(OsStr::new("nezha")))
                    .with_extension("dtb");
                Ext::new("dtc").arg("-o").arg(&dtb).arg(&dt).invoke();
                ans.dtb.replace(fs::File::open(dtb)?);
            } else {
                ans.dtb.replace(fs::File::open(dt)?);
            }
        }
        Ok(ans)
    }

    pub fn asm(&self, arg: AsmArg) -> Result<(), Box<dyn Error>> {
        let mut packages = vec![];
        if self.spl {
            packages.push(Package::Spl);
        }
        if self.see {
            packages.push(Package::See);
        }
        let packages = if packages.is_empty() {
            vec![Package::Spl, Package::See]
        } else {
            packages
        };
        // 如果没有设置输出目录，就放在项目根目录
        let output = arg.output.clone().unwrap_or(DIRS.workspace.join("target"));
        // 如果设置了要反汇编哪个模块
        if let [package] = packages.as_slice() {
            // 如果输出是个目录，就放在这个目录下，否则保存为输出指定的文件（可能会覆盖现有文件）
            let path = if output.is_dir() {
                output.join(package.name()).with_extension("asm")
            } else {
                output.to_path_buf()
            };
            // 保存
            package.objdump(path)
        }
        // 如果没有设置要反汇编哪个模块
        else {
            if output.is_dir() {
                // 输出就是目录，什么也不用做
            } else if !output.exists() {
                // 输出不存在，创建为一个目录
                fs::create_dir_all(&output)?;
            } else {
                // 存在一个同名文件，不能用目录替换文件
                return Err(IoError::new(
                    IoErrorKind::AlreadyExists,
                    "multiple targets need a dir to save",
                )
                .into());
            }
            // 保存两个文件
            for package in packages {
                package.objdump(output.join(package.name()).with_extension("asm"))?;
            }
            Ok(())
        }
    }
}
