use std::{collections::HashMap, sync::mpsc::Sender, thread};

use signal_hook::iterator::Signals;

use crate::segments::{Segment, SegmentReference};

pub(crate) fn spawn_signal_handler(segments: &[Segment], channel: Sender<SegmentReference>) {
    let signals_map = segments
        .iter()
        .enumerate()
        .flat_map(|(seg_ref, segment)| {
            segment
                .signals
                .iter()
                .copied()
                .map(move |signal| (signal, seg_ref))
        })
        .fold(
            HashMap::new(),
            |mut map: HashMap<i32, Vec<SegmentReference>>, (signal, segment)| {
                map.entry(signal).or_default().push(segment);
                map
            },
        );

    let used_signals = signals_map.keys().copied();
    let mut signals = Signals::new(used_signals).unwrap();

    thread::spawn(move || {
        for signal in signals.forever() {
            for &segment_ref in &signals_map[&signal] {
                channel.send(segment_ref).unwrap();
            }
        }
    });
}
