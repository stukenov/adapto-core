use serde::{Deserialize, Serialize};

/// Per-feature standardization to zero mean and unit variance.
///
/// Zero-variance columns get `std = 1`, so they pass through as `x - mean`
/// (contributing a constant 0 on the training set rather than dividing by ~0).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StandardScaler {
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
}

impl StandardScaler {
    /// Fit to feature rows using population standard deviation.
    /// An empty input yields empty mean/std vectors.
    pub fn fit(rows: &[Vec<f64>]) -> Self {
        let k = rows.first().map(|r| r.len()).unwrap_or(0);
        if k == 0 {
            return StandardScaler { mean: vec![], std: vec![] };
        }
        let n = rows.len().max(1) as f64;

        let mut mean = vec![0.0; k];
        for r in rows {
            for (i, &v) in r.iter().enumerate() {
                mean[i] += v;
            }
        }
        for m in &mut mean {
            *m /= n;
        }

        let mut var = vec![0.0; k];
        for r in rows {
            for (i, &v) in r.iter().enumerate() {
                let d = v - mean[i];
                var[i] += d * d;
            }
        }
        let std = var
            .into_iter()
            .map(|s| {
                let sd = (s / n).sqrt();
                if sd < 1e-9 {
                    1.0
                } else {
                    sd
                }
            })
            .collect();

        StandardScaler { mean, std }
    }

    /// Standardize one feature vector. Missing indices fall back to mean 0 / std 1.
    pub fn transform(&self, x: &[f64]) -> Vec<f64> {
        x.iter()
            .enumerate()
            .map(|(i, &v)| {
                let m = self.mean.get(i).copied().unwrap_or(0.0);
                let s = self.std.get(i).copied().unwrap_or(1.0);
                (v - m) / s
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_and_transform() {
        let s = StandardScaler::fit(&[vec![0.0], vec![2.0], vec![4.0]]);
        assert!((s.mean[0] - 2.0).abs() < 1e-9);
        assert!((s.std[0] - 1.632_993).abs() < 1e-4);
        assert!((s.transform(&[2.0])[0] - 0.0).abs() < 1e-9);
        assert!((s.transform(&[4.0])[0] - 1.224_745).abs() < 1e-4);
    }

    #[test]
    fn zero_variance_column_passes_through() {
        let s = StandardScaler::fit(&[vec![5.0, 1.0], vec![5.0, 3.0]]);
        assert_eq!(s.std[0], 1.0); // forced
        assert!((s.transform(&[5.0, 2.0])[0]).abs() < 1e-9); // (5-5)/1 = 0
    }
}
