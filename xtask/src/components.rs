use crate::{xfel::Xfel, AsmArg, Package, Target, XError, DIRS};
use command_ext::{CommandExt, Ext};
use std::{
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read},
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
            ans.spl.replace(Package::Spl.objcopy());
        }
        if self.see {
            ans.see.replace(Package::See.objcopy());
        }
        if let Some(kernel) = &self.kernel {
            if !kernel.is_file() {
                return Err(IoError::new(
                    IoErrorKind::NotFound,
                    format!("kernel file \"{}\" not exist", kernel.display()),
                )
                .into());
            }
            ans.kernel.replace(kernel.clone());
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
                    .join(dt.file_stem().unwrap_or_else(|| OsStr::new("nezha")))
                    .with_extension("dtb");
                Ext::new("dtc").arg("-o").arg(&dtb).arg(&dt).invoke();
                ans.dtb.replace(dtb);
            } else {
                ans.dtb.replace(dt.clone());
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
        let output = arg.output.unwrap_or_else(|| DIRS.workspace.join("target"));
        // 如果设置了要反汇编哪个模块
        if let [package] = packages.as_slice() {
            // 如果输出是个目录，就放在这个目录下，否则保存为输出指定的文件（可能会覆盖现有文件）
            let path = if output.is_dir() {
                output.join(package.name()).with_extension("asm")
            } else {
                output
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

    pub fn debug(&self) -> Result<(), Box<dyn Error>> {
        use common::memory::*;
        // FIXME 通过 xfel 初始化 ddr 之后 sram 就不能用了
        if self.spl && self.see {
            return Err(
                XError::InvalidProcedure("debuging spl + see is not supported now".into()).into(),
            );
        }
        if !self.see && self.kernel.is_some() {
            return Err(
                XError::InvalidProcedure("cannot debuging kernel without see".into()).into(),
            );
        }
        // 生成
        let target = self.make()?;
        let mut meta = Meta::DEFAULT;
        // 写入 see
        if let Some(see) = &target.see {
            Xfel::ddr("d1").invoke();
            meta.set_see(0);
            info!("write {} to {DRAM:#x}", see.display());
            Xfel::write(DRAM, see).invoke();
            // 写入 kernel
            if let Some(kernel) = &target.kernel {
                meta.set_see((KERNEL - DRAM) as _);
                info!("write {} to {KERNEL:#x}", kernel.display());
                Xfel::write(KERNEL, kernel).invoke();
            }
            // 写入 dtb
            if let Some(dtb) = &target.dtb {
                let len = dtb.metadata().unwrap().len() as usize;

                let mut file: File = File::open(dtb)?;
                let mut buf = vec![0u32; (len + 3) / 4];
                file.read_exact(unsafe {
                    std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), len)
                })?;
                let offset = dtb_offset(parse_memory_size(buf.as_ptr().cast()));
                let address = DRAM + offset as usize;
                meta.set_dtb(offset);
                info!("write {} to {address:#x}", dtb.display());
                Xfel::write(address, dtb).invoke();
            }
        }
        // 写入 spl 或执行外部初始化流程
        let entry = if let Some(spl) = &target.spl {
            info!("write {} to {SRAM:#x}", spl.display());
            Xfel::write(SRAM, spl).invoke();
            SRAM
        } else {
            DRAM
        };
        // 写入元数据
        let meta_bytes = meta.as_bytes();
        let meta = DIRS.target.join("meta.bin");
        fs::write(&meta, meta_bytes)?;
        info!("write {} to {META:#x}", meta.display());
        Xfel::write(META, meta).invoke();
        // 执行
        info!("exec from {entry:#x}");
        Xfel::exec(entry).invoke();
        Ok(())
    }
}
