use std::{path::PathBuf, process::Command};

use log::warn;

use super::SegmentKind;

#[derive(Debug)]
pub struct ProgramOutput {
    program: PathBuf,
    args: Vec<String>,
}

impl ProgramOutput {
    pub fn new(program: PathBuf, args: Vec<String>) -> Self {
        ProgramOutput { program, args }
    }
}

impl SegmentKind for ProgramOutput {
    fn compute_value(&mut self) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_segment_kinds;

    test_segment_kinds!(
        program: ProgramOutput::new("echo".into(),vec!["hello".into()]) => "hello",
    );
}
