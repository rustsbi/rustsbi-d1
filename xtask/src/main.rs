mod asm;
mod xfel;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use asm::AsmArgs;
use clap::Parser;
use command_ext::{BinUtil, Cargo, CommandExt};
use once_cell::sync::Lazy;
use std::{
    error::Error,
    path::{Path, PathBuf},
    str::FromStr,
};
use xfel::Xfel;

#[derive(Parser)]
#[clap(name = "NeZha Boot Util")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Asm(AsmArgs),
    Boot,
    Debug(DebugArg),
}

static DIRS: Lazy<Dirs> = Lazy::new(Dirs::new);

fn main() -> Result<(), Box<dyn Error>> {
    use simplelog::*;
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    use Commands::*;
    match Cli::parse().command {
        Asm(args) => args.asm(),
        Boot => todo!(),
        Debug(arg) => arg.debug(),
    }
    Ok(())
}

struct Dirs {
    workspace: PathBuf,
    target: PathBuf,
}

impl Dirs {
    fn new() -> Self {
        let workspace = Path::new(std::env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let target = workspace.join("target/riscv64imac-unknown-none-elf/release");
        Self { workspace, target }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Sram,
    Dram,
}

impl FromStr for Stage {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sram" => Ok(Self::Sram),
            "dram" => Ok(Self::Dram),
            _ => Err("unknown stage"),
        }
    }
}

impl Stage {
    #[inline]
    const fn package(&self) -> Package {
        match self {
            Self::Sram => Package::Bt0,
            Self::Dram => Package::See,
        }
    }

    #[inline]
    const fn base_addr(&self) -> &'static str {
        match self {
            Stage::Sram => "0x20000",
            Stage::Dram => "0x40000000",
        }
    }
}

enum Package {
    Bt0,
    See,
}

impl Package {
    #[inline]
    const fn name(&self) -> &'static str {
        match self {
            Self::Bt0 => "bt0",
            Self::See => "see",
        }
    }

    #[inline]
    const fn both() -> [Self; 2] {
        [Self::Bt0, Self::See]
    }

    #[inline]
    fn build(&self) {
        Cargo::build().package(self.name()).release().invoke();
    }

    #[inline]
    fn target(&self) -> PathBuf {
        DIRS.target.join(self.name())
    }

    fn objdump(&self) -> Vec<u8> {
        self.build();
        BinUtil::objdump()
            .arg(self.target())
            .arg("-d")
            .output()
            .stdout
    }

    fn objcopy(&self) -> PathBuf {
        self.build();
        let target = self.target();
        let bin = target.with_extension("bin");
        BinUtil::objcopy()
            .arg("--binary-architecture=riscv64")
            .arg(target)
            .args(["--strip-all", "-O", "binary"])
            .arg(&bin)
            .invoke();
        bin
    }
}

#[derive(Args)]
struct DebugArg {
    #[clap(long)]
    stage: Stage,
}

impl DebugArg {
    fn debug(&self) {
        let base_addr = self.stage.base_addr();
        let bin = self.stage.package().objcopy();
        if let Stage::Dram = self.stage {
            Xfel::ddr().arg("d1").invoke();
        }
        Xfel::write().arg(base_addr).arg(bin).invoke();
        Xfel::exec().arg(base_addr).invoke();
    }
}
