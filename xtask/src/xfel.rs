use command_ext::{ext, CommandExt, Ext};
use once_cell::sync::Lazy;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

ext!(def; Xfel);

static PATH: Lazy<PathBuf> = Lazy::new(detect_xfel);

impl Xfel {
    #[inline]
    fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut xfel = Command::new(&*PATH);
        xfel.args(args);
        Self(xfel)
    }

    #[inline]
    pub fn version() -> Self {
        Self::new(["version"])
    }

    #[inline]
    pub fn write(address: usize, file: impl AsRef<Path>) -> Self {
        let mut ans = Self::new(["write"]);
        ans.arg(format!("{address:#x}")).arg(file.as_ref());
        ans
    }

    #[inline]
    pub fn write32(address: usize, value: u32) -> Self {
        let mut ans = Self::new(["write32"]);
        ans.arg(format!("{address:#x}")).arg(format!("{value:#x}"));
        ans
    }

    #[inline]
    pub fn exec(address: usize) -> Self {
        let mut ans = Self::new(["exec"]);
        ans.arg(format!("{address:#x}"));
        ans
    }

    #[inline]
    pub fn ddr(ty: &str) -> Self {
        Self::new(["ddr", ty])
    }

    #[inline]
    pub fn erase(address: usize, length: usize) -> Self {
        let mut ans = Self::new(["spinand", "erase"]);
        ans.arg(format!("{address:#x}")).arg(format!("{length}"));
        ans
    }
}

fn detect_xfel() -> PathBuf {
    match Ext::new("xfel").as_mut().output() {
        Ok(output) => {
            if output.status.success() {
                let x = output
                    .stdout
                    .iter()
                    .copied()
                    .skip_while(|c| *c != b'(')
                    .skip(1)
                    .take_while(|c| *c != b')')
                    .collect::<Vec<u8>>();
                info!(
                    "detected xfel of version {:?}",
                    std::str::from_utf8(&x).unwrap()
                );
                PathBuf::from("xfel")
            } else {
                todo!()
            }
        }
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                error!("xfel not found");
                std::process::exit(1);
            } else {
                panic!("{e:?}");
            }
        }
    }
}
