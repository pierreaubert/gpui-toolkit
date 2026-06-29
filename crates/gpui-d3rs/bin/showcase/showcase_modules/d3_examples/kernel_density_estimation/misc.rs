/// Epanechnikov (parabolic) kernel
/// K(x) = 0.75 * (1 - x^2) for |x| <= 1, 0 otherwise
pub(super) fn epanechnikov_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| {
        let u = x / bandwidth;
        if u.abs() <= 1.0 {
            0.75 * (1.0 - u * u) / bandwidth
        } else {
            0.0
        }
    }
}

/// Gaussian kernel
/// K(x) = (1/sqrt(2*pi)) * exp(-0.5 * x^2)
pub(super) fn gaussian_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    let sqrt_2pi = (2.0 * std::f64::consts::PI).sqrt();
    move |x: f64| {
        let u = x / bandwidth;
        ((-0.5 * u * u).exp()) / (sqrt_2pi * bandwidth)
    }
}

/// Uniform (box) kernel
/// K(x) = 0.5 for |x| <= 1, 0 otherwise
pub(super) fn uniform_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| {
        let u = x / bandwidth;
        if u.abs() <= 1.0 { 0.5 / bandwidth } else { 0.0 }
    }
}

/// Compute kernel density estimation
pub(super) fn kde<F>(kernel: F, thresholds: &[f64], data: &[f64]) -> Vec<(f64, f64)>
where
    F: Fn(f64) -> f64,
{
    thresholds
        .iter()
        .map(|&t| {
            let density = data.iter().map(|&d| kernel(t - d)).sum::<f64>() / data.len() as f64;
            (t, density)
        })
        .collect()
}

/// Compute histogram bins
pub(super) fn histogram(
    data: &[f64],
    bin_count: usize,
    min: f64,
    max: f64,
) -> Vec<(f64, f64, usize)> {
    let bin_width = (max - min) / bin_count as f64;
    let mut bins = vec![0usize; bin_count];

    for &value in data {
        let bin_idx = ((value - min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(bin_count - 1);
        bins[bin_idx] += 1;
    }

    bins.iter()
        .enumerate()
        .map(|(i, &count)| {
            let bin_start = min + i as f64 * bin_width;
            let bin_end = bin_start + bin_width;
            (bin_start, bin_end, count)
        })
        .collect()
}
