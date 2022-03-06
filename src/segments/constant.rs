use super::SegmentKind;

#[derive(Debug)]
pub(crate) struct Constant {
    text: String,
}

impl Constant {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    pub(crate) fn compute_value(&self) -> String {
        self.text.clone()
    }
}

impl Into<SegmentKind> for Constant {
    fn into(self) -> SegmentKind {
        SegmentKind::Constant(self)
    }
}