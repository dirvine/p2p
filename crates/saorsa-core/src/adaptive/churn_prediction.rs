//! LSTM-based churn prediction module
//!
//! This module implements Long Short-Term Memory (LSTM) neural networks for
//! predicting node churn in the P2P network. It provides predictions for
//! 1-hour, 6-hour, and 24-hour horizons to enable proactive replication.

use crate::Result;
use crate::adaptive::NodeId;
use crate::adaptive::gossip::ChurnDetector;
use crate::identity::NodeIdentity;
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Feature vector for LSTM input
#[derive(Debug, Clone)]
pub struct ChurnFeatures {
    /// Historical churn rates (last 24 hours, hourly)
    pub churn_history: Vec<f64>,
    /// Time of day (0-23)
    pub hour_of_day: u8,
    /// Day of week (0-6)
    pub day_of_week: u8,
    /// Current network size
    pub network_size: usize,
    /// Average session duration
    pub avg_session_duration: Duration,
    /// Node uptime
    pub node_uptime: Duration,
    /// Recent join/leave ratio
    pub join_leave_ratio: f64,
}

/// LSTM cell state
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LSTMCell {
    hidden_state: Vec<f64>,
    cell_state: Vec<f64>,
}

/// LSTM layer configuration
#[derive(Debug, Clone)]
struct LSTMLayer {
    input_size: usize,
    hidden_size: usize,
    // Weight matrices
    weight_ih: Vec<Vec<f64>>, // Input to hidden
    weight_hh: Vec<Vec<f64>>, // Hidden to hidden
    bias_ih: Vec<f64>,
    bias_hh: Vec<f64>,
}

impl LSTMLayer {
    fn new(input_size: usize, hidden_size: usize) -> Self {
        Self {
            input_size,
            hidden_size,
            weight_ih: Self::xavier_init(4 * hidden_size, input_size),
            weight_hh: Self::xavier_init(4 * hidden_size, hidden_size),
            bias_ih: vec![0.0; 4 * hidden_size],
            bias_hh: vec![0.0; 4 * hidden_size],
        }
    }

    /// Xavier/Glorot initialization
    fn xavier_init(rows: usize, cols: usize) -> Vec<Vec<f64>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let scale = (2.0 / (rows + cols) as f64).sqrt();

        (0..rows)
            .map(|_| (0..cols).map(|_| rng.gen_range(-scale..scale)).collect())
            .collect()
    }

    /// Forward pass through LSTM cell
    fn forward(&self, input: &[f64], prev_h: &[f64], prev_c: &[f64]) -> (Vec<f64>, Vec<f64>) {
        // Compute gates
        let gates = self.compute_gates(input, prev_h);

        // Split gates into i, f, g, o
        let chunk_size = self.hidden_size;
        let i_gate = &gates[0..chunk_size];
        let f_gate = &gates[chunk_size..2 * chunk_size];
        let g_gate = &gates[2 * chunk_size..3 * chunk_size];
        let o_gate = &gates[3 * chunk_size..];

        // Apply gate activations
        let i_gate: Vec<f64> = i_gate.iter().map(|x| sigmoid(*x)).collect();
        let f_gate: Vec<f64> = f_gate.iter().map(|x| sigmoid(*x)).collect();
        let g_gate: Vec<f64> = g_gate.iter().map(|x| x.tanh()).collect();
        let o_gate: Vec<f64> = o_gate.iter().map(|x| sigmoid(*x)).collect();

        // Update cell state
        let mut new_c = vec![0.0; self.hidden_size];
        for i in 0..self.hidden_size {
            new_c[i] = f_gate[i] * prev_c[i] + i_gate[i] * g_gate[i];
        }

        // Update hidden state
        let mut new_h = vec![0.0; self.hidden_size];
        for i in 0..self.hidden_size {
            new_h[i] = o_gate[i] * new_c[i].tanh();
        }

        (new_h, new_c)
    }

    fn compute_gates(&self, input: &[f64], hidden: &[f64]) -> Vec<f64> {
        let mut gates = vec![0.0; 4 * self.hidden_size];

        // Input contribution
        for i in 0..4 * self.hidden_size {
            for j in 0..self.input_size {
                gates[i] += self.weight_ih[i][j] * input[j];
            }
            gates[i] += self.bias_ih[i];
        }

        // Hidden contribution
        for i in 0..4 * self.hidden_size {
            for j in 0..self.hidden_size {
                gates[i] += self.weight_hh[i][j] * hidden[j];
            }
            gates[i] += self.bias_hh[i];
        }

        gates
    }
}

/// LSTM model for churn prediction
pub struct LSTMChurnPredictor {
    /// LSTM layers
    layers: Vec<LSTMLayer>,
    /// Output layer weights
    output_weights: Vec<Vec<f64>>,
    output_bias: Vec<f64>,
    /// Experience replay buffer
    replay_buffer: Arc<RwLock<VecDeque<Experience>>>,
    /// Model version for persistence
    version: u32,
    /// Learning rate
    learning_rate: f64,
}

/// Experience for replay buffer
#[derive(Debug, Clone)]
pub struct Experience {
    features: ChurnFeatures,
    actual_churn: Vec<f64>, // [1h, 6h, 24h]
    _timestamp: Instant,
}

/// Prediction horizons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnPrediction {
    /// Probability of churn in next 1 hour
    pub hour_1: f64,
    /// Probability of churn in next 6 hours
    pub hour_6: f64,
    /// Probability of churn in next 24 hours
    pub hour_24: f64,
    /// Confidence score (0-1)
    pub confidence: f64,
}

impl LSTMChurnPredictor {
    /// Create new LSTM predictor
    pub fn new() -> Self {
        // Architecture: Input -> LSTM(128) -> LSTM(64) -> Dense(3)
        let layers = vec![
            LSTMLayer::new(32, 128), // Input features to first LSTM
            LSTMLayer::new(128, 64), // Second LSTM layer
        ];

        Self {
            layers,
            output_weights: Self::xavier_init_2d(3, 64),
            output_bias: vec![0.0; 3],
            replay_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            version: 1,
            learning_rate: 0.001,
        }
    }

    fn xavier_init_2d(rows: usize, cols: usize) -> Vec<Vec<f64>> {
        LSTMLayer::xavier_init(rows, cols)
    }

    /// Extract features from node and network state
    pub async fn extract_features(
        &self,
        node_id: &NodeIdentity,
        churn_detector: &ChurnDetector,
        network_size: usize,
    ) -> ChurnFeatures {
        // Get churn history
        let churn_history = churn_detector.get_hourly_rates(24).await;

        // Get current time features
        let now = chrono::Local::now();
        let hour_of_day = now.hour() as u8;
        let day_of_week = now.weekday().num_days_from_monday() as u8;

        // Calculate join/leave ratio
        let recent_stats = churn_detector
            .get_recent_stats(Duration::from_secs(3600))
            .await;
        let join_leave_ratio = if recent_stats.leaves > 0 {
            recent_stats.joins as f64 / recent_stats.leaves as f64
        } else {
            1.0
        };

        ChurnFeatures {
            churn_history,
            hour_of_day,
            day_of_week,
            network_size,
            avg_session_duration: recent_stats.avg_session_duration,
            node_uptime: recent_stats
                .get_node_uptime(&NodeId::from_bytes(*node_id.node_id().to_bytes())),
            join_leave_ratio,
        }
    }

    /// Predict churn probability for a node
    pub async fn predict(&self, features: &ChurnFeatures) -> Result<ChurnPrediction> {
        // Normalize features
        let input = self.normalize_features(features);

        // Forward pass through LSTM layers
        let mut hidden_states = vec![];
        let mut cell_states = vec![];

        // Initialize states
        for layer in &self.layers {
            hidden_states.push(vec![0.0; layer.hidden_size]);
            cell_states.push(vec![0.0; layer.hidden_size]);
        }

        // Process through LSTM layers
        let mut current_input = input;
        for (i, layer) in self.layers.iter().enumerate() {
            let (new_h, new_c) = layer.forward(&current_input, &hidden_states[i], &cell_states[i]);
            hidden_states[i] = new_h.clone();
            cell_states[i] = new_c;
            current_input = new_h;
        }

        // Output layer
        let mut output = vec![0.0; 3];
        for i in 0..3 {
            for j in 0..current_input.len() {
                output[i] += self.output_weights[i][j] * current_input[j];
            }
            output[i] += self.output_bias[i];
            output[i] = sigmoid(output[i]); // Probability output
        }

        // Calculate confidence based on feature quality
        let confidence = self.calculate_confidence(features);

        Ok(ChurnPrediction {
            hour_1: output[0],
            hour_6: output[1],
            hour_24: output[2],
            confidence,
        })
    }

    /// Normalize input features
    fn normalize_features(&self, features: &ChurnFeatures) -> Vec<f64> {
        let mut normalized = Vec::with_capacity(32);

        // Churn history (24 values)
        for rate in &features.churn_history {
            normalized.push(rate.min(1.0)); // Cap at 100% churn
        }

        // Time features
        normalized.push(features.hour_of_day as f64 / 23.0);
        normalized.push(features.day_of_week as f64 / 6.0);

        // Network features
        normalized.push((features.network_size as f64).ln() / 10.0); // Log scale
        normalized.push(features.avg_session_duration.as_secs() as f64 / 86400.0); // Normalize to days
        normalized.push(features.node_uptime.as_secs() as f64 / 86400.0);
        normalized.push(features.join_leave_ratio.min(10.0) / 10.0);

        // Pad to 32 features
        while normalized.len() < 32 {
            normalized.push(0.0);
        }

        normalized
    }

    /// Calculate prediction confidence
    fn calculate_confidence(&self, features: &ChurnFeatures) -> f64 {
        let mut confidence = 1.0;

        // Reduce confidence for sparse history
        let valid_history = features.churn_history.iter().filter(|&&r| r > 0.0).count();
        confidence *= (valid_history as f64 / 24.0).min(1.0);

        // Reduce confidence for very new nodes
        if features.node_uptime < Duration::from_secs(3600) {
            confidence *= 0.5;
        }

        confidence
    }

    /// Online learning with new experience
    pub async fn learn(&mut self, experience: Experience) -> Result<()> {
        // Add to replay buffer
        {
            let mut buffer = self.replay_buffer.write().await;
            buffer.push_back(experience.clone());
            if buffer.len() > 10000 {
                buffer.pop_front();
            }
        }

        // Sample mini-batch for training
        let batch = self.sample_batch(32).await;
        if batch.len() < 16 {
            return Ok(()); // Not enough data yet
        }

        // Perform gradient descent
        for exp in batch {
            self.backward_pass(&exp)?;
        }

        Ok(())
    }

    /// Sample batch from replay buffer
    async fn sample_batch(&self, size: usize) -> Vec<Experience> {
        use rand::seq::SliceRandom;
        let buffer = self.replay_buffer.read().await;
        let all: Vec<_> = buffer.iter().cloned().collect();

        let mut rng = rand::thread_rng();
        all.choose_multiple(&mut rng, size).cloned().collect()
    }

    /// Backward pass for learning (simplified)
    fn backward_pass(&mut self, experience: &Experience) -> Result<()> {
        // Get prediction
        let features = &experience.features;
        let prediction = tokio::runtime::Handle::current().block_on(self.predict(features))?;

        // Calculate loss
        let pred_vec = vec![prediction.hour_1, prediction.hour_6, prediction.hour_24];
        let _losses: Vec<f64> = pred_vec
            .iter()
            .zip(&experience.actual_churn)
            .map(|(p, a)| (p - a).powi(2))
            .collect();

        // Simple gradient descent on output layer
        for i in 0..3 {
            let grad = 2.0 * (pred_vec[i] - experience.actual_churn[i]);
            self.output_bias[i] -= self.learning_rate * grad;

            // Update output weights (simplified)
            for j in 0..self.output_weights[i].len() {
                self.output_weights[i][j] -= self.learning_rate * grad * 0.1;
            }
        }

        Ok(())
    }

    /// Save model to disk
    pub async fn save(&self, path: &std::path::Path) -> Result<()> {
        let model_data = ModelData {
            version: self.version,
            layers: self.layers.iter().map(|l| LayerData::from(l)).collect(),
            output_weights: self.output_weights.clone(),
            output_bias: self.output_bias.clone(),
        };

        let json = serde_json::to_string_pretty(&model_data)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Load model from disk
    pub async fn load(path: &std::path::Path) -> Result<Self> {
        let json = tokio::fs::read_to_string(path).await?;
        let model_data: ModelData = serde_json::from_str(&json)?;

        let layers = model_data.layers.into_iter().map(|d| d.into()).collect();

        Ok(Self {
            layers,
            output_weights: model_data.output_weights,
            output_bias: model_data.output_bias,
            replay_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            version: model_data.version,
            learning_rate: 0.001,
        })
    }

    /// Get proactive replication recommendations
    pub async fn get_replication_recommendations(
        &self,
        nodes: &[NodeIdentity],
        churn_detector: &ChurnDetector,
        network_size: usize,
        threshold: f64,
    ) -> Vec<(NodeIdentity, ChurnPrediction)> {
        let mut recommendations = Vec::new();

        for node in nodes {
            let features = self
                .extract_features(node, churn_detector, network_size)
                .await;
            if let Ok(prediction) = self.predict(&features).await {
                // Recommend replication if any horizon exceeds threshold
                if prediction.hour_1 > threshold
                    || prediction.hour_6 > threshold
                    || prediction.hour_24 > threshold
                {
                    recommendations.push((node.clone(), prediction));
                }
            }
        }

        // Sort by urgency (1-hour prediction)
        recommendations.sort_by(|a, b| {
            b.1.hour_1
                .partial_cmp(&a.1.hour_1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        recommendations
    }
}

/// Model data for serialization
#[derive(Serialize, Deserialize)]
struct ModelData {
    version: u32,
    layers: Vec<LayerData>,
    output_weights: Vec<Vec<f64>>,
    output_bias: Vec<f64>,
}

/// Layer data for serialization
#[derive(Serialize, Deserialize)]
struct LayerData {
    input_size: usize,
    hidden_size: usize,
    weight_ih: Vec<Vec<f64>>,
    weight_hh: Vec<Vec<f64>>,
    bias_ih: Vec<f64>,
    bias_hh: Vec<f64>,
}

impl From<&LSTMLayer> for LayerData {
    fn from(layer: &LSTMLayer) -> Self {
        LayerData {
            input_size: layer.input_size,
            hidden_size: layer.hidden_size,
            weight_ih: layer.weight_ih.clone(),
            weight_hh: layer.weight_hh.clone(),
            bias_ih: layer.bias_ih.clone(),
            bias_hh: layer.bias_hh.clone(),
        }
    }
}

impl From<LayerData> for LSTMLayer {
    fn from(data: LayerData) -> Self {
        LSTMLayer {
            input_size: data.input_size,
            hidden_size: data.hidden_size,
            weight_ih: data.weight_ih,
            weight_hh: data.weight_hh,
            bias_ih: data.bias_ih,
            bias_hh: data.bias_hh,
        }
    }
}

/// Sigmoid activation function
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(10.0) > 0.99);
        assert!(sigmoid(-10.0) < 0.01);
    }

    #[test]
    fn test_lstm_layer_creation() {
        let layer = LSTMLayer::new(10, 20);
        assert_eq!(layer.input_size, 10);
        assert_eq!(layer.hidden_size, 20);
        assert_eq!(layer.weight_ih.len(), 80); // 4 * hidden_size
        assert_eq!(layer.weight_ih[0].len(), 10); // input_size
    }

    #[tokio::test]
    async fn test_feature_normalization() {
        let predictor = LSTMChurnPredictor::new();

        let features = ChurnFeatures {
            churn_history: vec![0.1; 24],
            hour_of_day: 12,
            day_of_week: 3,
            network_size: 1000,
            avg_session_duration: Duration::from_secs(7200),
            node_uptime: Duration::from_secs(86400),
            join_leave_ratio: 1.5,
        };

        let normalized = predictor.normalize_features(&features);
        assert_eq!(normalized.len(), 32);

        // Check normalization ranges
        for value in &normalized {
            assert!(*value >= 0.0 && *value <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_prediction() {
        let predictor = LSTMChurnPredictor::new();

        let features = ChurnFeatures {
            churn_history: vec![0.05; 24],
            hour_of_day: 14,
            day_of_week: 2,
            network_size: 500,
            avg_session_duration: Duration::from_secs(3600),
            node_uptime: Duration::from_secs(172800),
            join_leave_ratio: 1.0,
        };

        let prediction = predictor.predict(&features).await.unwrap();

        // Check probability ranges
        assert!(prediction.hour_1 >= 0.0 && prediction.hour_1 <= 1.0);
        assert!(prediction.hour_6 >= 0.0 && prediction.hour_6 <= 1.0);
        assert!(prediction.hour_24 >= 0.0 && prediction.hour_24 <= 1.0);
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);

        // Longer horizons should have higher or equal probability
        assert!(prediction.hour_6 >= prediction.hour_1);
        assert!(prediction.hour_24 >= prediction.hour_6);
    }

    #[tokio::test]
    async fn test_experience_replay() {
        let mut predictor = LSTMChurnPredictor::new();

        let features = ChurnFeatures {
            churn_history: vec![0.1; 24],
            hour_of_day: 10,
            day_of_week: 1,
            network_size: 200,
            avg_session_duration: Duration::from_secs(1800),
            node_uptime: Duration::from_secs(3600),
            join_leave_ratio: 2.0,
        };

        let experience = Experience {
            features,
            actual_churn: vec![0.1, 0.2, 0.3],
            _timestamp: Instant::now(),
        };

        // Add experience and learn
        predictor.learn(experience).await.unwrap();

        // Check replay buffer
        let buffer = predictor.replay_buffer.read().await;
        assert_eq!(buffer.len(), 1);
    }

    #[tokio::test]
    async fn test_model_persistence() {
        let predictor = LSTMChurnPredictor::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let model_path = temp_dir.path().join("lstm_model.json");

        // Save model
        predictor.save(&model_path).await.unwrap();
        assert!(model_path.exists());

        // Load model
        let loaded = LSTMChurnPredictor::load(&model_path).await.unwrap();
        assert_eq!(loaded.version, predictor.version);
        assert_eq!(loaded.layers.len(), predictor.layers.len());
    }
}
