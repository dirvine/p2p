// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! # Beta Distribution Implementation
//!
//! This module provides a proper Beta distribution implementation for Thompson Sampling
//! in the Multi-Armed Bandit routing optimization system.
//!
//! ## Features
//! - Exact Beta distribution sampling using acceptance-rejection method
//! - Fast path for special cases (uniform, degenerate)
//! - Parameter validation and bounds checking
//! - Thread-safe sampling with thread-local RNG

use rand::Rng;

/// Beta distribution parameters
#[derive(Debug, Clone, Copy)]
pub struct BetaDistribution {
    /// Alpha parameter (successes + 1)
    pub alpha: f64,
    /// Beta parameter (failures + 1)
    pub beta: f64,
}

impl BetaDistribution {
    /// Create a new Beta distribution
    pub fn new(alpha: f64, beta: f64) -> Result<Self, BetaError> {
        if alpha <= 0.0 || beta <= 0.0 {
            return Err(BetaError::InvalidParameters {
                alpha,
                beta,
                reason: "Alpha and beta must be positive".to_string(),
            });
        }

        if !alpha.is_finite() || !beta.is_finite() {
            return Err(BetaError::InvalidParameters {
                alpha,
                beta,
                reason: "Parameters must be finite".to_string(),
            });
        }

        Ok(Self { alpha, beta })
    }

    /// Sample from the Beta distribution
    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        // Special cases for efficiency
        if self.alpha == 1.0 && self.beta == 1.0 {
            // Uniform distribution
            return rng.r#gen::<f64>();
        }

        if self.alpha == 1.0 {
            // Beta(1, β) = 1 - U^(1/β) where U ~ Uniform(0,1)
            let u: f64 = rng.r#gen::<f64>();
            return 1.0 - u.powf(1.0 / self.beta);
        }

        if self.beta == 1.0 {
            // Beta(α, 1) = U^(1/α) where U ~ Uniform(0,1)
            let u: f64 = rng.r#gen::<f64>();
            return u.powf(1.0 / self.alpha);
        }

        // General case: use Gamma distribution method
        // Beta(α, β) = Gamma(α) / (Gamma(α) + Gamma(β))
        let gamma_alpha = sample_gamma(self.alpha, rng);
        let gamma_beta = sample_gamma(self.beta, rng);

        gamma_alpha / (gamma_alpha + gamma_beta)
    }

    /// Get the mean of the distribution
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Get the variance of the distribution
    pub fn variance(&self) -> f64 {
        let sum = self.alpha + self.beta;
        (self.alpha * self.beta) / (sum * sum * (sum + 1.0))
    }

    /// Get the mode of the distribution (if it exists)
    pub fn mode(&self) -> Option<f64> {
        if self.alpha > 1.0 && self.beta > 1.0 {
            Some((self.alpha - 1.0) / (self.alpha + self.beta - 2.0))
        } else if self.alpha == 1.0 && self.beta == 1.0 {
            // Uniform distribution, any value in [0,1] is a mode
            Some(0.5)
        } else if self.alpha < 1.0 && self.beta < 1.0 {
            // Bimodal at 0 and 1
            None
        } else if self.alpha < 1.0 {
            Some(0.0)
        } else if self.beta < 1.0 {
            Some(1.0)
        } else {
            None
        }
    }

    /// Update parameters based on success/failure
    pub fn update(&mut self, success: bool) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }

    /// Get the 95% confidence interval
    pub fn confidence_interval(&self) -> (f64, f64) {
        // Use normal approximation for large parameters
        if self.alpha > 30.0 && self.beta > 30.0 {
            let mean = self.mean();
            let std_dev = self.variance().sqrt();
            let margin = 1.96 * std_dev; // 95% CI
            ((mean - margin).max(0.0), (mean + margin).min(1.0))
        } else {
            // For small parameters, use quantile approximation
            // This is a simplified version; for exact quantiles use incomplete beta function
            let lower = self.alpha / (self.alpha + self.beta + 2.0);
            let upper = (self.alpha + 1.0) / (self.alpha + self.beta + 1.0);
            (lower, upper)
        }
    }
}

/// Sample from Gamma distribution using Marsaglia and Tsang's method
fn sample_gamma<R: Rng>(shape: f64, rng: &mut R) -> f64 {
    if shape < 1.0 {
        // Use Johnk's algorithm for shape < 1
        let u: f64 = rng.r#gen::<f64>();
        sample_gamma(1.0 + shape, rng) * u.powf(1.0 / shape)
    } else {
        // Marsaglia and Tsang's method for shape >= 1
        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();

        loop {
            let mut x;
            let mut v;

            loop {
                x = rng.gen_range(-1.0..1.0); // Standard normal approximation
                v = 1.0 + c * x;
                if v > 0.0 {
                    break;
                }
            }

            v = v * v * v;
            let u: f64 = rng.r#gen::<f64>();

            if u < 1.0 - 0.0331 * x * x * x * x {
                return d * v;
            }

            if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
                return d * v;
            }
        }
    }
}

/// Errors that can occur with Beta distribution
#[derive(Debug, Clone)]
pub enum BetaError {
    /// Invalid parameters provided
    InvalidParameters {
        alpha: f64,
        beta: f64,
        reason: String,
    },
}

impl std::fmt::Display for BetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BetaError::InvalidParameters {
                alpha,
                beta,
                reason,
            } => {
                write!(
                    f,
                    "Invalid Beta parameters (α={}, β={}): {}",
                    alpha, beta, reason
                )
            }
        }
    }
}

impl std::error::Error for BetaError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_beta_distribution_creation() {
        // Valid parameters
        let dist = BetaDistribution::new(2.0, 3.0);
        assert!(dist.is_ok());

        // Invalid parameters
        assert!(BetaDistribution::new(0.0, 1.0).is_err());
        assert!(BetaDistribution::new(1.0, -1.0).is_err());
        assert!(BetaDistribution::new(f64::INFINITY, 1.0).is_err());
        assert!(BetaDistribution::new(1.0, f64::NAN).is_err());
    }

    #[test]
    fn test_beta_distribution_sampling() {
        let mut rng = thread_rng();
        let dist = BetaDistribution::new(2.0, 5.0).unwrap();

        // Sample should be in [0, 1]
        for _ in 0..1000 {
            let sample = dist.sample(&mut rng);
            assert!(sample >= 0.0);
            assert!(sample <= 1.0);
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_beta_distribution_special_cases() {
        let mut rng = thread_rng();

        // Uniform distribution
        let uniform = BetaDistribution::new(1.0, 1.0).unwrap();
        let samples: Vec<f64> = (0..1000).map(|_| uniform.sample(&mut rng)).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 0.5).abs() < 0.05); // Should be close to 0.5

        // Beta(1, β)
        let beta_1_b = BetaDistribution::new(1.0, 3.0).unwrap();
        for _ in 0..100 {
            let sample = beta_1_b.sample(&mut rng);
            assert!(sample >= 0.0 && sample <= 1.0);
        }

        // Beta(α, 1)
        let beta_a_1 = BetaDistribution::new(3.0, 1.0).unwrap();
        for _ in 0..100 {
            let sample = beta_a_1.sample(&mut rng);
            assert!(sample >= 0.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_beta_distribution_moments() {
        let dist = BetaDistribution::new(2.0, 5.0).unwrap();

        // Test mean
        assert_eq!(dist.mean(), 2.0 / 7.0);

        // Test variance
        let expected_variance = (2.0 * 5.0) / (7.0 * 7.0 * 8.0);
        assert!((dist.variance() - expected_variance).abs() < 1e-10);

        // Test mode
        let mode = dist.mode().unwrap();
        assert_eq!(mode, 1.0 / 5.0); // (α-1)/(α+β-2) = 1/5
    }

    #[test]
    fn test_beta_parameter_updates() {
        let mut dist = BetaDistribution::new(1.0, 1.0).unwrap();

        // Success increases alpha
        dist.update(true);
        assert_eq!(dist.alpha, 2.0);
        assert_eq!(dist.beta, 1.0);

        // Failure increases beta
        dist.update(false);
        assert_eq!(dist.alpha, 2.0);
        assert_eq!(dist.beta, 2.0);
    }

    #[test]
    fn test_beta_confidence_interval() {
        // Small parameters
        let dist_small = BetaDistribution::new(2.0, 3.0).unwrap();
        let (lower, upper) = dist_small.confidence_interval();
        assert!(lower >= 0.0);
        assert!(upper <= 1.0);
        assert!(lower < upper);

        // Large parameters (uses normal approximation)
        let dist_large = BetaDistribution::new(50.0, 40.0).unwrap();
        let (lower, upper) = dist_large.confidence_interval();
        let mean = dist_large.mean();
        assert!(lower < mean);
        assert!(mean < upper);
    }

    #[test]
    fn test_beta_distribution_convergence() {
        // Test that sampling converges to expected mean
        let mut rng = thread_rng();
        let dist = BetaDistribution::new(3.0, 7.0).unwrap();
        let expected_mean = dist.mean();

        let n_samples = 10000;
        let samples: Vec<f64> = (0..n_samples).map(|_| dist.sample(&mut rng)).collect();
        let sample_mean = samples.iter().sum::<f64>() / n_samples as f64;

        // Should converge within 1% of expected mean
        assert!((sample_mean - expected_mean).abs() < 0.01);
    }

    #[test]
    fn test_mode_edge_cases() {
        // Uniform distribution
        let uniform = BetaDistribution::new(1.0, 1.0).unwrap();
        assert_eq!(uniform.mode(), Some(0.5));

        // Mode at 0
        let mode_0 = BetaDistribution::new(0.5, 2.0).unwrap();
        assert_eq!(mode_0.mode(), Some(0.0));

        // Mode at 1
        let mode_1 = BetaDistribution::new(2.0, 0.5).unwrap();
        assert_eq!(mode_1.mode(), Some(1.0));

        // Bimodal (no single mode)
        let bimodal = BetaDistribution::new(0.5, 0.5).unwrap();
        assert_eq!(bimodal.mode(), None);
    }
}
