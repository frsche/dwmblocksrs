use std::borrow::Cow;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Color(u8);

impl Color {
    fn color(&self, text: &str) -> String {
        format!("{}{}\x01", self.0 as char, text)
    }
}

pub(crate) fn color<'a>(c: Option<Color>, text: &'a str) -> Cow<'a, str> {
    match c {
        Some(c) => Cow::Owned(c.color(text)),
        None => Cow::Borrowed(text),
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct SegmentColoring {
    pub text: Option<Color>,
    pub left_separator: Option<Color>,
    pub right_separator: Option<Color>,
    pub icon: Option<Color>,
}

impl SegmentColoring {
    pub(crate) fn or_default(self, default_coloring: &SegmentColoring) -> Self {
        Self {
            text: self.text.or(default_coloring.text),
            left_separator: self.left_separator.or(default_coloring.text),
            right_separator: self.right_separator.or(default_coloring.text),
            icon: self.icon.or(default_coloring.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let c = Color(2);
        assert_eq!(c.color("test"), "\x02test\x01");
    }
}
