use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle, ProgressState, ProgressFinish};

pub struct ProgressReporter {
    bar: ProgressBar,
    start_time: Instant,
    last_update: Instant,
}

impl ProgressReporter {
    pub fn new(total: u64, task_name: &str) -> Self {
        let bar = ProgressBar::new(total);
        
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        bar.set_message(format!("Starting {}", task_name));
        
        let now = Instant::now();
        
        Self {
            bar,
            start_time: now,
            last_update: now,
        }
    }

    pub fn new_spinner(task_name: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        
        bar.set_message(format!("Starting {}", task_name));
        
        let now = Instant::now();
        
        Self {
            bar,
            start_time: now,
            last_update: now,
        }
    }

    pub fn set_position(&mut self, pos: u64) {
        self.bar.set_position(pos);
        self.last_update = Instant::now();
    }

    pub fn inc(&mut self, delta: u64) {
        self.bar.inc(delta);
        self.last_update = Instant::now();
    }

    pub fn set_message<S: AsRef<str>>(&mut self, msg: S) {
        self.bar.set_message(msg.as_ref().to_string());
    }

    pub fn update_with_message<S: AsRef<str>>(&mut self, pos: u64, msg: S) {
        self.set_position(pos);
        self.set_message(msg);
    }

    pub fn finish_with_message<S: AsRef<str>>(self, msg: S) {
        self.bar.finish_with_message(msg.as_ref().to_string());
    }

    pub fn abandon_with_message<S: AsRef<str>>(self, msg: S) {
        self.bar.abandon_with_message(msg.as_ref().to_string());
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn estimated_total_time(&self) -> Option<Duration> {
        let pos = self.bar.position();
        let total = self.bar.length()?;
        
        if pos == 0 {
            return None;
        }
        
        let elapsed = self.elapsed();
        let rate = pos as f64 / elapsed.as_secs_f64();
        
        if rate > 0.0 {
            let total_seconds = total as f64 / rate;
            Some(Duration::from_secs_f64(total_seconds))
        } else {
            None
        }
    }

    pub fn items_per_second(&self) -> f64 {
        let pos = self.bar.position();
        let elapsed = self.elapsed().as_secs_f64();
        
        if elapsed > 0.0 {
            pos as f64 / elapsed
        } else {
            0.0
        }
    }
}

pub struct MultiProgressReporter {
    bars: Vec<ProgressBar>,
    multi: indicatif::MultiProgress,
}

impl MultiProgressReporter {
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            multi: indicatif::MultiProgress::new(),
        }
    }

    pub fn add_bar(&mut self, total: u64, task_name: &str) -> usize {
        let bar = self.multi.add(ProgressBar::new(total));
        
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.dim} {spinner:.green} [{bar:25.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        bar.set_prefix(task_name.to_string());
        
        let index = self.bars.len();
        self.bars.push(bar);
        index
    }

    pub fn add_spinner(&mut self, task_name: &str) -> usize {
        let bar = self.multi.add(ProgressBar::new_spinner());
        
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{prefix:.bold.dim} {spinner:.green} {msg}")
                .unwrap()
        );
        
        bar.set_prefix(task_name.to_string());
        
        let index = self.bars.len();
        self.bars.push(bar);
        index
    }

    pub fn update_bar(&mut self, index: usize, pos: u64, msg: &str) {
        if let Some(bar) = self.bars.get(index) {
            bar.set_position(pos);
            bar.set_message(msg.to_string());
        }
    }

    pub fn finish_bar(&mut self, index: usize, msg: &str) {
        if let Some(bar) = self.bars.get(index) {
            bar.finish_with_message(msg.to_string());
        }
    }

    pub fn finish_all(self, msg: &str) {
        for bar in self.bars {
            bar.finish_with_message(msg.to_string());
        }
    }
}

impl Default for MultiProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_progress_reporter_creation() {
        let reporter = ProgressReporter::new(100, "Test Task");
        assert_eq!(reporter.bar.length(), Some(100));
        assert_eq!(reporter.bar.position(), 0);
    }

    #[test]
    fn test_progress_reporter_update() {
        let mut reporter = ProgressReporter::new(100, "Test Task");
        
        reporter.set_position(50);
        assert_eq!(reporter.bar.position(), 50);
        
        reporter.inc(10);
        assert_eq!(reporter.bar.position(), 60);
    }

    #[test]
    fn test_progress_reporter_timing() {
        let mut reporter = ProgressReporter::new(100, "Test Task");
        
        // Small delay to ensure elapsed time > 0
        thread::sleep(StdDuration::from_millis(10));
        
        reporter.set_position(10);
        
        assert!(reporter.elapsed() > StdDuration::from_millis(5));
        assert!(reporter.items_per_second() > 0.0);
    }

    #[test]
    fn test_spinner_creation() {
        let reporter = ProgressReporter::new_spinner("Test Spinner");
        assert_eq!(reporter.bar.length(), None); // Spinners have no length
    }

    #[test]
    fn test_multi_progress_reporter() {
        let mut multi = MultiProgressReporter::new();
        
        let bar1 = multi.add_bar(100, "Task 1");
        let bar2 = multi.add_spinner("Task 2");
        
        assert_eq!(bar1, 0);
        assert_eq!(bar2, 1);
        
        multi.update_bar(0, 50, "Half done");
        multi.update_bar(1, 0, "Processing...");
        
        multi.finish_bar(0, "Task 1 complete");
        multi.finish_bar(1, "Task 2 complete");
    }
}