mod walk;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::{ArchFileScan, ArchPlan};
use crate::output::Violation;
use crate::rules::{RulePass, RuleScan, rule_wants};
use crate::syntax::parse_source;

pub(crate) use walk::{NodeIndex, Wants};

/// Everything the per-file sweep needs, shared immutably across workers.
struct Engine<'a> {
    rule_passes: &'a [RulePass],
    arch: Option<&'a ArchPlan>,
    files: &'a [PathBuf],
    contents: &'a [String],
    root: &'a Path,
    wants: Wants,
}

/// One file's violations, bucketed by pass so the final assembly can emit
/// pass-major order without sorting.
type FileBuckets = Vec<Vec<Violation>>;

/// Walks every file once — read, parse, then run every pass that covers it —
/// across as many threads as the machine has cores. Each file is parsed
/// exactly once by construction, and violations are assembled pass-major
/// afterwards so output order does not depend on completion order.
pub(crate) fn run_engine(
    rule_passes: &[RulePass],
    arch: Option<&ArchPlan>,
    files: &[PathBuf],
    contents: &[String],
    root: &Path,
) -> Vec<Violation> {
    let engine = Engine {
        rule_passes,
        arch,
        files,
        contents,
        root,
        wants: rule_wants(rule_passes),
    };

    let scanned = engine.sweep_files();
    engine.assemble(scanned)
}

impl Engine<'_> {
    fn pass_count(&self) -> usize {
        self.rule_passes.len() + self.arch.map_or(0, ArchPlan::pass_count)
    }

    fn sweep_files(&self) -> Vec<FileBuckets> {
        let next = AtomicUsize::new(0);
        let workers = worker_count(self.files.len());

        let claimed = std::thread::scope(|scope| {
            (0..workers)
                .map(|_| scope.spawn(|| self.drain_queue(&next)))
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|handle| handle.join().expect("engine worker thread panicked"))
                .collect::<Vec<_>>()
        });

        let mut scanned: Vec<FileBuckets> = (0..self.files.len()).map(|_| Vec::new()).collect();
        for (index, buckets) in claimed {
            scanned[index] = buckets;
        }
        scanned
    }

    fn drain_queue(&self, next: &AtomicUsize) -> Vec<(usize, FileBuckets)> {
        let mut done = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            if index >= self.files.len() {
                return done;
            }
            done.push((index, self.scan_file(index)));
        }
    }

    fn scan_file(&self, file_index: usize) -> FileBuckets {
        let file = &self.files[file_index];
        let content = &self.contents[file_index];
        let source = content.as_bytes();

        let tree = self
            .needs_tree(file_index, file)
            .then(|| parse_source(file, content))
            .flatten();
        let tree_root = tree.as_ref().map(|tree| tree.root_node());

        let scans_rules =
            tree_root.is_some() && self.rule_passes.iter().any(|pass| pass.covers(file_index));
        let index = match tree_root {
            Some(root) if scans_rules => NodeIndex::build(root, source, self.wants),
            _ => NodeIndex::default(),
        };

        let mut buckets = Vec::with_capacity(self.pass_count());
        for pass in self.rule_passes {
            buckets.push(if scans_rules && pass.covers(file_index) {
                pass.scan(
                    file,
                    self.root,
                    RuleScan {
                        index: &index,
                        source,
                        content,
                    },
                )
            } else {
                Vec::new()
            });
        }

        if let Some(arch) = self.arch {
            arch.scan_file(
                ArchFileScan {
                    file_index,
                    file,
                    root: self.root,
                    tree_root,
                    source,
                    content,
                },
                &mut buckets,
            );
        }
        buckets
    }

    fn needs_tree(&self, file_index: usize, file: &Path) -> bool {
        self.rule_passes.iter().any(|pass| pass.covers(file_index))
            || self
                .arch
                .is_some_and(|arch| arch.needs_tree(file_index, file))
    }

    fn assemble(&self, scanned: Vec<FileBuckets>) -> Vec<Violation> {
        let mut scanned = scanned;
        let mut violations = Vec::new();
        for pass in 0..self.pass_count() {
            for buckets in &mut scanned {
                if let Some(bucket) = buckets.get_mut(pass) {
                    violations.append(bucket);
                }
            }
        }
        violations
    }
}

fn worker_count(files: usize) -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |count| count.get());
    cores.min(files).max(1)
}
