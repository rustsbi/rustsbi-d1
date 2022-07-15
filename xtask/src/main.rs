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
    Erase(EraseArg),
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
        Erase(arg) => arg.erase(),
    };
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
            Self::Sram => Package::Spl,
            Self::Dram => Package::See,
        }
    }

    #[inline]
    const fn base_addr(&self) -> usize {
        match self {
            Stage::Sram => 0x0002_0000,
            Stage::Dram => 0x4000_0000,
        }
    }
}

enum Package {
    Spl,
    See,
}

impl Package {
    #[inline]
    const fn name(&self) -> &'static str {
        match self {
            Self::Spl => "spl",
            Self::See => "see",
        }
    }

    #[inline]
    const fn both() -> [Self; 2] {
        [Self::Spl, Self::See]
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
    fn debug(&self) -> bool {
        let address = self.stage.base_addr();
        let file = self.stage.package().objcopy();
        if let Stage::Dram = self.stage {
            Xfel::ddr("d1").invoke();
            common::memory::Meta::DEFAULT
                .as_u32s()
                .into_iter()
                .copied()
                .enumerate()
                .for_each(|(i, value)| {
                    Xfel::write32(common::memory::META + i * 4, value).invoke();
                });
        }
        info!("writing {} to {address:#x}", file.display());
        Xfel::write(address, file).invoke();
        info!("execute from {address:#x}");
        Xfel::exec(address).invoke();
        true
    }
}

#[derive(Args)]
struct EraseArg {
    #[clap(short, long)]
    range: Option<String>,
}

impl EraseArg {
    fn erase(&self) -> bool {
        let range = match &self.range {
            Some(s) => s,
            None => {
                const META: usize = common::flash::Meta::POS as _;
                let range = META..META + 4096;
                info!("erasing range: {range:#x?}");
                Xfel::erase(range.start, range.len()).invoke();
                return true;
            }
        };
        let range = if let Some((start, end)) = range.split_once("..") {
            let start = match parse_num(start) {
                Some(val) => val,
                None => {
                    error!("failed to parse start \"{start}\"");
                    return false;
                }
            };
            let end = match parse_num(end) {
                Some(val) => val,
                None => {
                    error!("failed to parse end \"{end}\"");
                    return false;
                }
            };
            start..end
        } else if let Some(temp) = range.trim_end().strip_suffix(']') {
            let (base, len) = match temp.split_once('[') {
                Some(pair) => pair,
                None => {
                    error!("failed to split base and len: \"{temp}\"");
                    return false;
                }
            };
            let base = match parse_num(base) {
                Some(val) => val,
                None => {
                    error!("failed to parse base \"{base}\"");
                    return false;
                }
            };
            let len = match parse_num(len) {
                Some(val) => val,
                None => {
                    error!("failed to parse len \"{len}\"");
                    return false;
                }
            };
            base..base + len
        } else {
            error!("cannot parse range: {range}");
            return false;
        };
        if range.is_empty() {
            error!("erasing range is empty {range:#x?}");
            false
        } else {
            info!("erasing range: {range:#x?}");
            Xfel::erase(range.start, range.len()).invoke();
            true
        }
    }
}

fn parse_num(s: &str) -> Option<usize> {
    match s.trim_start().strip_prefix("0x") {
        Some(s) => usize::from_str_radix(s, 16).ok(),
        None => usize::from_str_radix(s, 10).ok(),
    }
}
