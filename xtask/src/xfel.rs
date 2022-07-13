use command_ext::{ext, Ext};
use once_cell::sync::Lazy;
use std::{ffi::OsStr, path::PathBuf, process::Command};

ext!(def; Xfel);

static PATH: Lazy<PathBuf> = Lazy::new(detect_xfel);

impl Xfel {
    #[inline]
    fn new(command: impl AsRef<OsStr>) -> Self {
        let mut xfel = Command::new(&*PATH);
        xfel.arg(command);
        Self(xfel)
    }

    #[inline]
    pub fn version() -> Self {
        Self::new("version")
    }

    #[inline]
    pub fn write() -> Self {
        Self::new("write")
    }

    #[inline]
    pub fn exec() -> Self {
        Self::new("exec")
    }

    #[inline]
    pub fn ddr() -> Self {
        Self::new("ddr")
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
