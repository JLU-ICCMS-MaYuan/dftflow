//! Eliashberg 对称矩阵通过 LAPACK/OpenBLAS 求解；不保留近似 Jacobi 回退。
use ndarray::Array2;
use ndarray_linalg::{EigValsh, UPLO};

pub fn symmetric_eigenvalues(a: &Array2<f64>) -> Vec<f64> {
    a.eigvalsh(UPLO::Lower)
        .expect("OpenBLAS 对称本征值求解失败")
        .to_vec()
}

pub fn max_eigenvalue(a: &Array2<f64>) -> f64 {
    symmetric_eigenvalues(a)
        .into_iter()
        .fold(f64::NEG_INFINITY, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lapack_known_spectrum_and_dense_matrix() {
        let a = ndarray::array![[2.0, 1.0], [1.0, 2.0]];
        let w = symmetric_eigenvalues(&a);
        assert!((w[0] - 1.0).abs() < 1e-12 && (w[1] - 3.0).abs() < 1e-12);
        // Dense 65x65: I + vv^T has eigenvalues 1 (64 times), 66.
        let mut a = Array2::ones((65, 65));
        for i in 0..65 {
            a[[i, i]] += 1.0;
        }
        assert!((max_eigenvalue(&a) - 66.0).abs() < 1e-10);
    }
}
