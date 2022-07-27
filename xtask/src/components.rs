use crate::{xfel::Xfel, AsmArg, FlashArgs, Package, Target, XError, DIRS};
use command_ext::{dir, CommandExt, Ext};
use common::uninit;
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Seek, SeekFrom},
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
    pub fn make(&self) -> Result<Target, XError> {
        let mut ans = Target::default();
        // 生成 spl
        if self.spl {
            use common::{AsBinary, EgonHead};
            let mut file = File::open(ans.spl.insert(Package::Spl.objcopy()))?;
            // 校验 stamp
            let mut egonhead = unsafe { uninit::<EgonHead>() };
            file.seek(SeekFrom::Start(4))?;
            file.read_exact(egonhead.as_buf())?;
            if egonhead.checksum != EgonHead::DEFAULT.checksum {
                error!(
                    "wrong stamp value {:#x}; check your generated blob and try again",
                    egonhead.checksum
                );
                return Err(XError::InvalidStamp);
            }
        }
        // 生成 see
        if self.see {
            ans.see.replace(Package::See.objcopy());
        }
        // 生成 kernel
        if let Some(kernel) = &self.kernel {
            if kernel.as_os_str() == OsStr::new("::test") {
                ans.kernel.replace(Package::TestKernel.objcopy());
            } else {
                // 检查 kernel 文件是否存在
                if !kernel.is_file() {
                    return Err(IoError::new(
                        IoErrorKind::NotFound,
                        format!("kernel file \"{}\" not exist", kernel.display()),
                    )
                    .into());
                }
                ans.kernel.replace(kernel.clone());
            }
        }
        // 生成 dtb
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
                dir::create_parent(&dtb).unwrap();
                Ext::new("dtc").arg("-o").arg(&dtb).arg(&dt).invoke();
                ans.dtb.replace(dtb);
            } else {
                ans.dtb.replace(dt.clone());
            }
        }
        Ok(ans)
    }

    pub fn asm(&self, arg: AsmArg) -> Result<(), XError> {
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
                dir::create_parent(&output).unwrap();
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

    pub fn debug(&self) -> Result<(), XError> {
        use common::{memory::*, AsBinary};
        // FIXME 通过 xfel 初始化 ddr 之后 sram 就不能用了
        if self.spl && self.see {
            return Err(XError::InvalidProcedure(
                "debuging spl + see is not supported now".into(),
            ));
        }
        if !self.see && self.kernel.is_some() {
            return Err(XError::InvalidProcedure(
                "cannot debuging kernel without see".into(),
            ));
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
                meta.set_kernel((KERNEL - DRAM) as _);
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
        let meta = DIRS.target.join("meta_ram.bin");
        fs::write(&meta, meta_bytes)?;
        info!("write {} to {META:#x}", meta.display());
        Xfel::write(META, meta).invoke();
        // 执行
        info!("exec from {entry:#x}");
        Xfel::exec(entry).invoke();
        Ok(())
    }

    pub fn flash(&self, args: FlashArgs) -> Result<(), XError> {
        use common::{flash::*, memory, AsBinary};

        let target = self.make()?;

        if let Some(spl) = target.spl {
            use common::EgonHead;
            // 必须对齐到 16 KiB，实际只有 16 KiB 和 32 KiB 两种可能性，干脆直接 32 KiB
            let mut file = [0u8; EgonHead::DEFAULT.length as _];
            let _ = File::open(&spl).unwrap().read(&mut file)?;
            // 设定 flash 启动
            unsafe { &mut *(file[0x68..].as_mut_ptr() as *mut memory::Meta) }.from_flash = true;
            // 计算并填写校验和
            let checksum =
                unsafe { core::slice::from_raw_parts(file.as_ptr() as *const u32, file.len() / 4) }
                    .iter()
                    .copied()
                    .reduce(|a, b| a.wrapping_add(b))
                    .unwrap();
            unsafe { &mut *(file[4..].as_mut_ptr() as *mut EgonHead) }.checksum = checksum;
            // 保存文件
            let checked = spl.with_file_name("spl.checked.bin");
            fs::write(&checked, file).unwrap();
            Xfel::spinand_write(0, checked).invoke();
        }

        let meta_path = DIRS.target.join("meta_flash.bin");
        let mut meta = Meta::DEFAULT;
        // 如果不需要重置文件系统，则从 Flash 读取现有的元数据
        if !args.reset {
            Xfel::spinand_read(META as _, Meta::SIZE, &meta_path).invoke();
            File::open(&meta_path)?.read_exact(meta.as_buf())?;
        }
        // 写各模块
        if let Some(see) = target.see {
            meta.set_see(SEE, see.metadata().unwrap().len() as _);
            Xfel::spinand_write(SEE as _, see).invoke();
        }
        if let Some(kernel) = target.kernel {
            meta.set_kernel(KERNEL, kernel.metadata().unwrap().len() as _);
            Xfel::spinand_write(KERNEL as _, kernel).invoke();
        }
        if let Some(dtb) = target.dtb {
            meta.set_dtb(DTB, dtb.metadata().unwrap().len() as _);
            Xfel::spinand_write(DTB as _, dtb).invoke();
        }
        // 元数据写到文件，再从文件写到 flash
        fs::write(&meta_path, meta.as_bytes()).unwrap();
        Xfel::spinand_write(META as _, meta_path).invoke();
        // 重启，必然返回错误
        if args.boot {
            assert!(!Xfel::reset().status().success());
        }
        Ok(())
    }
}
