use std::borrow::Cow;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Color {
    Colored(u8),
    Uncolored,
}

pub(crate) trait Colorable {
    fn color(&self, color: Color) -> Cow<'_, str>;
}

impl<A: AsRef<str>> Colorable for A {
    fn color(&self, color: Color) -> Cow<'_, str> {
        let text = self.as_ref();
        match color {
            Color::Uncolored => Cow::Borrowed(text),
            Color::Colored(c) => Cow::Owned(format!("{}{}\x01", c as char, text)),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::Uncolored
    }
}

impl From<Option<u8>> for Color {
    fn from(c: Option<u8>) -> Color {
        match c {
            Some(c) => Color::Colored(c),
            None => Color::Uncolored,
        }
    }
}

impl Color {
    fn or_default(self, default: Color) -> Self {
        match self {
            Self::Uncolored => default,
            _ => self,
        }
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct SegmentColoring {
    pub text: Color,
    pub left_separator: Color,
    pub right_separator: Color,
    pub icon: Color,
}

impl SegmentColoring {
    pub(crate) fn or_default(self, default_coloring: &SegmentColoring) -> Self {
        Self {
            text: self.text.or_default(default_coloring.text),
            left_separator: self
                .left_separator
                .or_default(default_coloring.left_separator),
            right_separator: self
                .right_separator
                .or_default(default_coloring.right_separator),
            icon: self.icon.or_default(default_coloring.icon),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let c = Color::Colored(2);
        assert_eq!("test".color(c), "\x02test\x01");
    }
}
