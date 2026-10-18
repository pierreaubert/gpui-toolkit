//! Small dense eigensolvers shared by flow diagnostics and glyphs.

// Rust guideline compliant 2026-02-21

/// Eigendecomposition of a symmetric 3x3 matrix by cyclic Jacobi rotation.
///
/// Returns `(eigenvalues, eigenvectors)` with eigenvectors as columns:
/// `eigenvectors[axis][i]` is the `axis` component of the `i`-th vector.
#[expect(
    clippy::needless_range_loop,
    reason = "fixed-size 3x3 matrix indexing is clearer than iterator zips"
)]
pub(crate) fn symmetric_eigen(m: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut v = [[0.0; 3]; 3];
    for i in 0..3 {
        v[i][i] = 1.0;
    }
    if !m.iter().flatten().all(|v| v.is_finite()) {
        return ([f64::NAN; 3], [[f64::NAN; 3]; 3]);
    }
    let scale = m.iter().flatten().map(|v| v.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        return ([0.0; 3], v);
    }
    // Normalize once: the stopping criterion and rotation remain independent
    // of field units, without squaring huge or subnormal source components.
    let mut a = m.map(|row| row.map(|value| value / scale));
    // Cyclic Jacobi: 12 sweeps converge far past f64 needs for 3x3.
    for _ in 0..12 {
        let off = (a[0][1].powi(2) + a[0][2].powi(2) + a[1][2].powi(2)).sqrt();
        if off <= f64::EPSILON * 8.0 {
            break;
        }
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            if a[p][q] == 0.0 {
                continue;
            }
            let theta = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
            let (s, c) = theta.sin_cos();
            for k in 0..3 {
                let (akp, akq) = (a[k][p], a[k][q]);
                a[k][p] = c * akp - s * akq;
                a[k][q] = s * akp + c * akq;
            }
            for k in 0..3 {
                let (apk, aqk) = (a[p][k], a[q][k]);
                a[p][k] = c * apk - s * aqk;
                a[q][k] = s * apk + c * aqk;
            }
            for k in 0..3 {
                let (vkp, vkq) = (v[k][p], v[k][q]);
                v[k][p] = c * vkp - s * vkq;
                v[k][q] = s * vkp + c * vkq;
            }
        }
    }
    ([a[0][0] * scale, a[1][1] * scale, a[2][2] * scale], v)
}

/// Imaginary part of the complex eigenvalue pair of a real 3x3 matrix.
///
/// Zero when all three eigenvalues are real. Computed from Cardano's
/// formula on the characteristic cubic; for a solid-body rotation `J`
/// this returns the angular speed.
pub(crate) fn swirling_strength(j: [[f64; 3]; 3]) -> f64 {
    // Characteristic polynomial: -l^3 + I1 l^2 - I2 l + I3 = 0, solved as
    // l^3 - I1 l^2 + I2 l - I3 = 0.
    let i1 = j[0][0] + j[1][1] + j[2][2];
    let i2 = 0.5
        * ((j[0][0] + j[1][1] + j[2][2]).powi(2)
            - (j[0][0].powi(2)
                + j[1][1].powi(2)
                + j[2][2].powi(2)
                + 2.0 * (j[0][1] * j[1][0] + j[0][2] * j[2][0] + j[1][2] * j[2][1])));
    let i3 = det3(j);
    // Depressed cubic t^3 + p t + q with l = t + I1/3.
    let p = i2 - i1 * i1 / 3.0;
    let q = -2.0 * i1.powi(3) / 27.0 + i1 * i2 / 3.0 - i3;
    let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
    if disc <= 0.0 {
        // Three real roots (or a repeated root): no swirling.
        return 0.0;
    }
    let sqrt_disc = disc.sqrt();
    let u = (-q / 2.0 + sqrt_disc).cbrt();
    let v = if u.abs() > 1e-300 {
        -p / (3.0 * u)
    } else {
        (-q / 2.0 - sqrt_disc).cbrt()
    };
    // Complex pair: real -(u+v)/2 + I1/3, imaginary (sqrt(3)/2)|u - v|.
    (3.0f64.sqrt() / 2.0 * (u - v)).abs()
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_eigen_resolves_coupled_equal_diagonals_independent_of_units() {
        for (source, expected) in [
            (
                [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 4.0]],
                [1.0, 3.0, 4.0],
            ),
            (
                [[3.0, 0.0, -2.0], [0.0, 3.0, -2.0], [-2.0, -2.0, 5.0]],
                [1.0, 3.0, 7.0],
            ),
        ] {
            for scale in [1.0, 1e-200, 1e200] {
                let matrix = source.map(|row| row.map(|v| v * scale));
                let (values, vectors) = symmetric_eigen(matrix);
                let mut sorted = values.map(|v| v / scale);
                sorted.sort_by(f64::total_cmp);
                for (actual, expected) in sorted.iter().zip(expected) {
                    assert!((actual - expected).abs() < 1e-12);
                }
                for (column, value) in values.iter().enumerate() {
                    for row in 0..3 {
                        let product = matrix[row]
                            .iter()
                            .enumerate()
                            .map(|(k, component)| (component / scale) * vectors[k][column])
                            .sum::<f64>();
                        assert!((product - (value / scale) * vectors[row][column]).abs() < 1e-12);
                    }
                    for other in 0..3 {
                        let dot = (0..3)
                            .map(|row| vectors[row][column] * vectors[row][other])
                            .sum::<f64>();
                        assert!((dot - if column == other { 1.0 } else { 0.0 }).abs() < 1e-12);
                    }
                }
            }
        }
    }

    #[test]
    #[expect(
        clippy::needless_range_loop,
        reason = "columns of a fixed 3x3 matrix are indexed directly"
    )]
    fn symmetric_eigen_recovers_diagonal() {
        let (values, vectors) =
            symmetric_eigen([[4.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 9.0]]);
        let mut sorted = values;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(sorted, [1.0, 4.0, 9.0]);
        // Eigenvectors stay orthonormal.
        for column in 0..3 {
            let len = (vectors[0][column].powi(2)
                + vectors[1][column].powi(2)
                + vectors[2][column].powi(2))
            .sqrt();
            assert!((len - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn swirling_of_rotation_is_angular_speed() {
        // Solid-body rotation omega = 2 about z.
        let j = [[0.0, -2.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert!((swirling_strength(j) - 2.0).abs() < 1e-9);
        let strain = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(swirling_strength(strain), 0.0);
    }
}
