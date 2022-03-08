use super::SegmentKind;

#[derive(Debug)]
pub(crate) struct Constant {
    text: String,
}

impl Constant {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub(crate) fn compute_value(&self) -> String {
        self.text.clone()
    }
}

impl From<Constant> for SegmentKind {
    fn from(constant: Constant) -> SegmentKind {
        SegmentKind::Constant(constant)
    }
}
