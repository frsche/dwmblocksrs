use std::{path::PathBuf, process::Command};

use log::warn;

use super::SegmentKind;

#[derive(Debug)]
pub(crate) struct ProgramOutput {
    program: PathBuf,
    args: Vec<String>,
}

impl ProgramOutput {
    pub fn new(program: PathBuf, args: Vec<String>) -> Self {
        ProgramOutput { program, args }
    }

    pub(crate) fn compute_value(&self) -> String {
        let output = match Command::new(&self.program).args(&self.args).output() {
            Ok(output) => output,
            Err(e) => {
                warn!(
                    "error running program {} {:?}: {}",
                    self.program.to_str().unwrap(),
                    self.args,
                    e
                );
                return "ERROR".into();
            }
        };

        if !output.status.success() {
            warn!(
                "program {} {:?} exited with non-zero error code ({}): {}",
                self.program.to_str().unwrap(),
                self.args,
                output.status,
                String::from_utf8(output.stderr).unwrap().trim()
            );
        }

        String::from_utf8(output.stdout).unwrap().trim().into()
    }
}

impl Into<SegmentKind> for ProgramOutput {
    fn into(self) -> SegmentKind {
        SegmentKind::ProgramOutput(self)
    }
}
