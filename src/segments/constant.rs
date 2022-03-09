use super::SegmentKind;

#[derive(Debug)]
pub struct Constant {
    text: String,
}

impl Constant {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl SegmentKind for Constant {
    fn compute_value(&mut self) -> String {
        self.text.clone()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_segment_kinds;

    test_segment_kinds!(
        constant: Constant::new("constant".into()) => "constant",
    );
}
