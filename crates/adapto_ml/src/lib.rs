//! # adapto_ml
//!
//! Small, dependency-light machine-learning primitives for the Adapto
//! framework. Pure Rust, no GPU, serde-serializable so models persist in
//! `adapto_store`.
//!
//! Currently provides [`StandardScaler`] (per-feature standardization) and
//! [`RidgeRegression`] (L2-regularized linear regression). These power
//! MOS-style forecast bias-correction: blending several model outputs and
//! correcting their per-location error, learned from observed values.
//!
//! ```
//! use adapto_ml::RidgeRegression;
//! let x = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
//! let y = vec![2.0, 5.0, 8.0, 11.0]; // y = 3x + 2
//! let m = RidgeRegression::fit(&x, &y, 1e-6).unwrap();
//! assert!((m.predict(&[4.0]) - 14.0).abs() < 0.1);
//! ```

mod ridge;
mod scaler;

pub use ridge::{MlError, RidgeRegression};
pub use scaler::StandardScaler;
