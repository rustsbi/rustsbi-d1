use crate::{Components, Package, DIRS};
use std::{fs, path::PathBuf};

#[derive(Args)]
pub(crate) struct AsmArg {
    #[clap(short, long)]
    output: Option<PathBuf>,
}

impl AsmArg {
    pub fn asm(&self, components: Components) -> bool {
        // 如果没有设置输出目录，就放在项目根目录
        let output = self.output.as_ref().unwrap_or(&DIRS.workspace).as_path();
        let mut packages = vec![];
        if components.spl {
            packages.push(Package::Spl);
        }
        if components.see {
            packages.push(Package::See);
        }
        // 如果设置了要反汇编哪个模块
        if let [package] = packages.as_slice() {
            // 如果输出是个目录，就放在这个目录下，否则保存为输出指定的文件（可能会覆盖现有文件）
            let path = if output.is_dir() {
                output.join(package.name()).with_extension("asm")
            } else {
                output.to_path_buf()
            };
            // 保存
            info!("dump {} to {}", package.name(), path.display());
            fs::write(path, package.objdump()).is_ok()
        }
        // 如果没有设置要反汇编哪个模块
        else {
            if output.is_dir() {
                // 输出就是目录，什么也不用做
            } else if !output.exists() {
                // 输出不存在，创建为一个目录
                if fs::create_dir_all(output).is_err() {
                    return false;
                }
            } else {
                // 存在一个同名文件，不能用目录替换文件
                error!("output path must be a directory");
                return false;
            }
            // 保存两个文件
            packages.into_iter().all(|package| {
                let path = output.join(package.name()).with_extension("asm");
                info!("dump {} to {}", package.name(), path.display());
                fs::write(path, package.objdump()).is_ok()
            })
        }
    }
}
