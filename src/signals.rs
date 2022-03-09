use async_std::{channel::Sender, prelude::*, task};
use signal_hook_async_std::Signals;
use std::collections::HashMap;

use crate::segments::{Segment};
use crate::SegmentId;

pub(crate) async fn spawn_signal_handler(segments: &[Segment], channel: Sender<SegmentId>) {
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
            |mut map: HashMap<i32, Vec<SegmentId>>, (signal, segment)| {
                map.entry(signal).or_default().push(segment);
                map
            },
        );

    let used_signals = signals_map.keys().copied();
    let mut signals = Signals::new(used_signals).unwrap();

    task::spawn(async move {
        while let Some(signal) = signals.next().await {
            for &segment_ref in &signals_map[&signal] {
                channel.send(segment_ref).await.unwrap();
            }
        }
    });
}
