use async_std::{channel::Sender, prelude::*, task};
use lazy_static::lazy_static;
use signal_hook_async_std::Signals;
use std::collections::HashMap;

use crate::segments::Segment;
use crate::SegmentId;

lazy_static! {
    static ref SIGRTMIN: i32 = libc::SIGRTMIN();
    static ref SIGRTMAX: i32 = libc::SIGRTMAX();
}

pub(crate) async fn spawn_signal_handler(
    segments: &[Segment],
    channel: Sender<SegmentId>,
) -> Result<(), String> {
    // maps each signal to all the SegmentIds that have to get updated when that signal comes
    let signals_map = segments
        .iter()
        .enumerate()
        .flat_map(|(seg_ref, segment)| {
            segment
                .signals
                .iter()
                .copied()
                // compute the signal by adding the offset to SIGRTMIN
                .map(move |signal| (signal as i32 + *SIGRTMIN, seg_ref))
        })
        .fold(
            HashMap::new(),
            |mut map: HashMap<i32, Vec<SegmentId>>, (signal, segment)| {
                map.entry(signal).or_default().push(segment);
                map
            },
        );

    // all the signals that are used by any of the segments
    let used_signals = signals_map.keys().copied().collect::<Vec<_>>();

    if used_signals.iter().any(|signal| *signal > *SIGRTMAX) {
        return Err("A used signal is greater than SIGRTMAX.".into());
    }

    let mut signals = Signals::new(used_signals).unwrap();
    task::spawn(async move {
        // when a signal arrives, look up which segments need updating and send them
        // through the channel
        while let Some(signal) = signals.next().await {
            println!("signal arrived");
            for &segment_ref in &signals_map[&signal] {
                channel.send(segment_ref).await.unwrap();
            }
        }
    });

    Ok(())
}
