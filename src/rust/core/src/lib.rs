//! BMS analyzer core.
//! Column-major matrix format: matrix[col][row].

pub mod bms;
pub mod bocf_mocf;
pub mod expand;
pub mod hydra;
pub mod ihss;
pub mod lmn;
pub mod mbocf;
pub mod one_y;
pub mod parser;
pub mod term;
pub mod triangular;
pub mod seq_std;
pub mod sss;
pub mod ocf;
pub mod upms;
pub mod wy;
pub mod y_dbms;
pub mod zero_y;

/// Column-major matrix: matrix[colIndex][rowIndex].
pub type Matrix = Vec<Vec<i32>>;

/// Layered mountain: outer = layer, inner = (value, parentDist).
pub type Mountain = Vec<Vec<(i32, i32)>>;
