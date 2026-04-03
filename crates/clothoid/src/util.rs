// Adapted from Numerical Recepies
// m[0][0] and m[n-1][2] are assumed to be zero and are not checked
pub fn solve_tridiag(m: &[[f64; 3]], r: &[f64]) -> Vec<f64> {
    let n = m.len();
    let mut u = vec![0.0; n];

    let mut gamma = vec![0.0; n];
    let mut beta = m[0][1];

    u[0] = r[0] / beta;

    for j in 1..n {
        gamma[j] = m[j-1][2] / beta;
        beta = m[j][1] - m[j][0] * gamma[j];
        u[j] = (r[j] - m[j][0]*u[j-1]) / beta;
    }
    for j in (0..(n-1)).rev() {
        u[j] -= gamma[j+1] * u[j +1];
    }
    u
}

// Solves a cyclic tridiagonal linear system using the Sherman-Morrison technique
// Adapted from Numerical Recepies
pub fn solve_cyclic_tridiag(mut m: Vec<[f64; 3]>, r: &[f64]) -> Vec<f64> {
    let n = m.len();
    let alpha = m[n-1][2];
    let beta = m[0][0];
    let gamma = (alpha * beta).abs().sqrt();
    let vn = (beta) / gamma;
    let mut u = vec![0.0; n];
    u[0] = gamma; u[n-1] = alpha;

    m[0][1] -= gamma;
    m[n-1][1] -= alpha * vn;

    let mut x = solve_tridiag(&m, r);
    let z = solve_tridiag(&m, &u);

    let fact = (gamma * x[0] + alpha * x[n-1]) / (1.0 + z[0] + vn * z[n-1]);
    for i in 0..n {
        x[i] -= fact * z[i];
    }
    x
}
