use crate::scaler::StandardScaler;
use serde::{Deserialize, Serialize};

/// Errors from fitting or loading a model.
#[derive(Debug, thiserror::Error)]
pub enum MlError {
    #[error("empty training data")]
    Empty,
    #[error("dimension mismatch")]
    DimMismatch,
    #[error("singular matrix — cannot solve")]
    Singular,
    #[error("serialization error: {0}")]
    Serde(String),
}

/// L2-regularized linear regression (ridge) with built-in feature
/// standardization.
///
/// Features are standardized via a fitted [`StandardScaler`]; the target is
/// centered, so the intercept is the target mean. Weights solve
/// `(ZᵀZ + λI) w = Zᵀ(y - ȳ)` where `Z` is the standardized design matrix.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RidgeRegression {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub lambda: f64,
    pub scaler: StandardScaler,
}

impl RidgeRegression {
    /// Fit on `features` (one inner vec per sample) and `targets`.
    ///
    /// `lambda` is the L2 penalty (use a small positive value, e.g. `1.0`).
    /// Returns [`MlError::Empty`] / [`MlError::DimMismatch`] for bad shapes and
    /// [`MlError::Singular`] if the normal matrix cannot be solved.
    pub fn fit(features: &[Vec<f64>], targets: &[f64], lambda: f64) -> Result<Self, MlError> {
        if features.is_empty() || targets.is_empty() {
            return Err(MlError::Empty);
        }
        if features.len() != targets.len() {
            return Err(MlError::DimMismatch);
        }
        let k = features[0].len();
        if k == 0 || features.iter().any(|r| r.len() != k) {
            return Err(MlError::DimMismatch);
        }

        let scaler = StandardScaler::fit(features);
        let z: Vec<Vec<f64>> = features.iter().map(|r| scaler.transform(r)).collect();
        let ybar = targets.iter().sum::<f64>() / targets.len() as f64;

        // A = ZᵀZ + λI   (k×k);   b = Zᵀ(y - ȳ)
        let mut a = vec![vec![0.0; k]; k];
        let mut b = vec![0.0; k];
        for (row, &y) in z.iter().zip(targets.iter()) {
            let yc = y - ybar;
            for i in 0..k {
                b[i] += row[i] * yc;
                for j in 0..k {
                    a[i][j] += row[i] * row[j];
                }
            }
        }
        for (i, ai) in a.iter_mut().enumerate() {
            ai[i] += lambda;
        }

        let weights = solve(a, b)?;
        Ok(RidgeRegression {
            weights,
            bias: ybar,
            lambda,
            scaler,
        })
    }

    /// Predict the target for one feature vector.
    pub fn predict(&self, x: &[f64]) -> f64 {
        let z = self.scaler.transform(x);
        let mut s = self.bias;
        for (w, v) in self.weights.iter().zip(z.iter()) {
            s += w * v;
        }
        s
    }

    /// Serialize to a JSON string (for storage in `adapto_store`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from a JSON string.
    pub fn from_json(s: &str) -> Result<Self, MlError> {
        serde_json::from_str(s).map_err(|e| MlError::Serde(e.to_string()))
    }
}

/// Solve `A x = b` by Gaussian elimination with partial pivoting.
/// Returns [`MlError::Singular`] on a near-zero pivot.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, MlError> {
    let n = b.len();
    for col in 0..n {
        let mut piv = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            let v = a[r][col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best < 1e-12 {
            return Err(MlError::Singular);
        }
        a.swap(col, piv);
        b.swap(col, piv);

        for r in (col + 1)..n {
            let f = a[r][col] / a[col][col];
            if f != 0.0 {
                for c in col..n {
                    a[r][c] -= f * a[col][c];
                }
                b[r] -= f * b[col];
            }
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in (i + 1)..n {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_linear() {
        let x: Vec<Vec<f64>> = (0..=10).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..=10).map(|i| 3.0 * i as f64 + 2.0).collect();
        let m = RidgeRegression::fit(&x, &y, 1e-6).unwrap();
        assert!((m.predict(&[4.0]) - 14.0).abs() < 0.1, "{}", m.predict(&[4.0]));
    }

    #[test]
    fn multi_feature() {
        // y = 1*x0 + 2*x1 over a grid
        let mut x = Vec::new();
        let mut y = Vec::new();
        for a in 0..5 {
            for b in 0..5 {
                x.push(vec![a as f64, b as f64]);
                y.push(a as f64 + 2.0 * b as f64);
            }
        }
        let m = RidgeRegression::fit(&x, &y, 1e-6).unwrap();
        assert!((m.predict(&[3.0, 1.0]) - 5.0).abs() < 0.2, "{}", m.predict(&[3.0, 1.0]));
    }

    #[test]
    fn serde_roundtrip() {
        let x: Vec<Vec<f64>> = (0..=10).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..=10).map(|i| 3.0 * i as f64 + 2.0).collect();
        let m = RidgeRegression::fit(&x, &y, 1e-6).unwrap();
        let m2 = RidgeRegression::from_json(&m.to_json()).unwrap();
        assert!((m.predict(&[7.0]) - m2.predict(&[7.0])).abs() < 1e-9);
    }

    #[test]
    fn empty_errors() {
        assert!(matches!(RidgeRegression::fit(&[], &[], 1.0), Err(MlError::Empty)));
        let r = RidgeRegression::fit(&[vec![1.0], vec![2.0]], &[1.0], 1.0);
        assert!(matches!(r, Err(MlError::DimMismatch)));
    }
}
