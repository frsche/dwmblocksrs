use std::{ffi::CString, ptr};
use x11::xlib::{Display, XDefaultScreen, XOpenDisplay, XRootWindow, XStoreName, XSync};

use crate::SegmentId;

pub(crate) struct StatusBar {
    display: *mut Display,
    window: u64,

    segment_texts: Vec<String>,
    current_text: String,
}

impl StatusBar {
    pub fn new(num_segments: usize) -> Self {
        let display;
        let screen;
        let window;

        unsafe {
            display = XOpenDisplay(ptr::null());
            screen = XDefaultScreen(display);
            window = XRootWindow(display, screen);
        }

        let segment_texts = vec!["".to_string(); num_segments];

        let current_text = segment_texts.join("");

        let s = Self {
            display,
            window,

            segment_texts,
            current_text,
        };

        s.set_status();
        s
    }

    pub(crate) fn update_segment(&mut self, id: SegmentId, text: String) {
        self.segment_texts[id] = text;
        let new_text = self.segment_texts.join("");
        if self.current_text != new_text {
            self.current_text = new_text;
            self.set_status();
        }
    }

    fn set_status(&self) {
        // https://github.com/hugglesfox/statusd/blob/main/src/xsetroot.rs
        // https://github.com/KJ002/simple_status/blob/main/src/status.rs

        let c_str = CString::new(self.current_text.clone()).unwrap();
        unsafe {
            XStoreName(self.display, self.window, c_str.as_ptr() as *const i8);
            XSync(self.display, 0);
        }
    }
}
