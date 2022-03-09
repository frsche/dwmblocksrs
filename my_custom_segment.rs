use std::time::Duration;

use async_std::task::block_on;
use dwmblocksrs::run;
use dwmblocksrs::segments::{constant::Constant, Segment, SegmentKind};

/// Custom segment that displays a number and increment every time the segment updates
#[derive(Debug)]
struct MyCustomSegment {
    counter: u64,
}

impl MyCustomSegment {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl SegmentKind for MyCustomSegment {
    fn compute_value(&mut self) -> String {
        let r = self.counter.to_string();
        self.counter += 1;
        r
    }
}

fn main() {
    let custom_segment = Segment::new(
        Box::new(MyCustomSegment::new()),
        // update every 10 seconds
        Duration::from_secs(10).into(),
        // and also update when signal SIGRTMIN+1 comes
        vec![48],
    );

    let arrow = Segment::new(
        Box::new(Constant::new("<--".into())),
        // the constant is never updated
        None,
        vec![],
    );

    let segments = vec![custom_segment, arrow];
    block_on(run(segments)).unwrap()
}
