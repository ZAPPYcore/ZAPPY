/// Tracks rolling reliability for master controller outputs.
#[derive(Debug, Clone)]
pub struct ReliabilityCalculator {
    history: Vec<f32>,
    capacity: usize,
}

impl Default for ReliabilityCalculator {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            capacity: 32,
        }
    }
}

impl ReliabilityCalculator {
    /// Records a confidence sample.
    pub fn record(&mut self, sample: f32) {
        if self.history.len() == self.capacity {
            self.history.remove(0);
        }
        self.history.push(sample.clamp(0.0, 1.0));
    }

    /// Computes an aggregate reliability score.
    #[must_use]
    pub fn score(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f32>() / self.history.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_records() {
        let mut calc = ReliabilityCalculator::default();
        calc.record(0.5);
        calc.record(0.9);
        assert!(calc.score() > 0.0);
    }
}
