/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::time::SystemTime;

use buck2_core::io_counters::IoCounterKey;
use gazebo::prelude::VecExt;
use superconsole::DrawMode;
use superconsole::Line;
use superconsole::Lines;

use crate::humanized::HumanizedBytes;
use crate::two_snapshots::TwoSnapshots;

#[derive(Default)]
pub struct IoState {
    two_snapshots: TwoSnapshots,
}

/// Place space-separated words on lines.
fn words_to_lines(words: Vec<String>, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    for word in words {
        if current_line.is_empty() {
            current_line = word;
            continue;
        }
        // This works correctly only for ASCII strings.
        if current_line.len() + 1 + word.len() > width {
            lines.push(current_line);
            current_line = word;
        } else {
            current_line.push(' ');
            current_line.push_str(&word);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

pub fn io_in_flight_non_zero_counters(
    snapshot: &buck2_data::Snapshot,
) -> impl Iterator<Item = (IoCounterKey, u32)> + '_ {
    IoCounterKey::ALL
        .iter()
        .map(|key| {
            let value = match key {
                IoCounterKey::Stat => snapshot.io_in_flight_stat,
                IoCounterKey::Copy => snapshot.io_in_flight_copy,
                IoCounterKey::Symlink => snapshot.io_in_flight_symlink,
                IoCounterKey::Hardlink => snapshot.io_in_flight_hardlink,
                IoCounterKey::MkDir => snapshot.io_in_flight_mk_dir,
                IoCounterKey::ReadDir => snapshot.io_in_flight_read_dir,
                IoCounterKey::ReadDirEden => snapshot.io_in_flight_read_dir_eden,
                IoCounterKey::RmDir => snapshot.io_in_flight_rm_dir,
                IoCounterKey::RmDirAll => snapshot.io_in_flight_rm_dir_all,
                IoCounterKey::StatEden => snapshot.io_in_flight_stat_eden,
                IoCounterKey::Chmod => snapshot.io_in_flight_chmod,
                IoCounterKey::ReadLink => snapshot.io_in_flight_read_link,
                IoCounterKey::Remove => snapshot.io_in_flight_remove,
                IoCounterKey::Rename => snapshot.io_in_flight_rename,
                IoCounterKey::Read => snapshot.io_in_flight_read,
                IoCounterKey::Write => snapshot.io_in_flight_write,
                IoCounterKey::Canonicalize => snapshot.io_in_flight_canonicalize,
                IoCounterKey::EdenSettle => snapshot.io_in_flight_eden_settle,
            };
            (*key, value)
        })
        .filter(|(_, value)| *value > 0)
}

impl IoState {
    pub fn update(&mut self, timestamp: SystemTime, snapshot: &buck2_data::Snapshot) {
        self.two_snapshots.update(timestamp, snapshot);
    }

    fn do_render(&self, snapshot: &buck2_data::Snapshot, width: usize) -> anyhow::Result<Lines> {
        let mut lines = Vec::new();
        let mut parts = Vec::new();
        if let Some(buck2_rss) = snapshot.buck2_rss {
            parts.push(format!("RSS = {}", HumanizedBytes::new(buck2_rss)));
        }
        if let Some(cpu) = self.two_snapshots.cpu_percents() {
            parts.push(format!("CPU = {}%", cpu));
        }
        if snapshot.deferred_materializer_queue_size > 0 {
            parts.push(format!(
                "DM Queue = {}",
                snapshot.deferred_materializer_queue_size
            ));
        }
        if snapshot.blocking_executor_io_queue_size > 0 {
            parts.push(format!(
                "IO Queue = {}",
                snapshot.blocking_executor_io_queue_size
            ));
        }
        if !parts.is_empty() {
            lines.push(Line::from_iter([superconsole::Span::new_unstyled(
                parts.join("  "),
            )?]));
        }

        let mut counters = Vec::new();
        for (key, value) in io_in_flight_non_zero_counters(snapshot) {
            counters.push(format!("{:?} = {}", key, value));
        }
        lines.extend(words_to_lines(counters, width).into_try_map(|s| Line::unstyled(&s))?);

        Ok(Lines(lines))
    }

    pub fn render(
        &self,
        draw_mode: DrawMode,
        width: usize,
        enabled: bool,
    ) -> anyhow::Result<Lines> {
        if !enabled {
            return Ok(Lines::new());
        }
        if let DrawMode::Final = draw_mode {
            return Ok(Lines::new());
        }
        if let Some((_, snapshot)) = &self.two_snapshots.last {
            self.do_render(snapshot, width)
        } else {
            Ok(Lines::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::words_to_lines;

    #[test]
    fn test_words_to_lines() {
        assert_eq!(vec![String::new(); 0], words_to_lines(vec![], 5));
        assert_eq!(
            vec!["ab".to_owned()],
            words_to_lines(vec!["ab".to_owned()], 5)
        );
        assert_eq!(
            vec!["ab cd".to_owned()],
            words_to_lines(vec!["ab".to_owned(), "cd".to_owned()], 5)
        );
        assert_eq!(
            vec!["ab".to_owned(), "cd".to_owned()],
            words_to_lines(vec!["ab".to_owned(), "cd".to_owned()], 4)
        );
        assert_eq!(
            vec!["abcd".to_owned()],
            words_to_lines(vec!["abcd".to_owned()], 3)
        );
    }
}
