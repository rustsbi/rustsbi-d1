use crate::{Package, Stage, DIRS};
use std::{fs, path::PathBuf};

#[derive(Args)]
pub struct AsmArgs {
    #[clap(long)]
    stage: Option<Stage>,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

impl AsmArgs {
    pub fn asm(&self) -> bool {
        // 如果没有设置输出目录，就放在项目根目录
        let output = self.output.as_ref().unwrap_or(&DIRS.workspace).as_path();
        // 如果设置了要反汇编哪个阶段
        if let Some(stage) = self.stage {
            let package = stage.package();
            // 如果输出是个目录，就放在这个目录下，否则保存为输出指定的文件（可能会覆盖现有文件）
            let path = if output.is_dir() {
                output.join(package.name()).with_extension("asm")
            } else {
                output.to_path_buf()
            };
            // 保存
            fs::write(path, package.objdump()).is_ok()
        }
        // 如果没有设置要反汇编哪个阶段
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
            Package::both().into_iter().all(|package| {
                fs::write(
                    output.join(package.name()).with_extension("asm"),
                    package.objdump(),
                )
                .is_ok()
            })
        }
    }
}
