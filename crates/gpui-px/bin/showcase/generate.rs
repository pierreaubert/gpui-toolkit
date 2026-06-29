/// Generate spiral scatter data
pub(super) fn generate_scatter_data() -> (Vec<f64>, Vec<f64>) {
    (0..100)
        .map(|i| {
            let t = i as f64 * 0.15;
            let r = 10.0 + t * 3.0;
            (50.0 + r * t.cos(), 50.0 + r * t.sin())
        })
        .unzip()
}

/// Generate sine wave data
pub(super) fn generate_line_data() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&xi| (xi * 2.0).sin() * 40.0 + 50.0).collect();
    (x, y)
}

/// Generate weekly bar data
pub(super) fn generate_bar_data() -> (Vec<String>, Vec<f64>) {
    let categories = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .map(String::from)
        .collect();
    let values = vec![45.0, 62.0, 55.0, 78.0, 68.0, 35.0, 28.0];
    (categories, values)
}

/// Generate box plot data (simulated Morley speed of light experiment)
///
/// This generates data similar to the classic Morley experiment dataset,
/// with 5 experiments each containing 20 measurements.
pub(super) fn generate_boxplot_data() -> (Vec<f64>, Vec<f64>) {
    // Morley experiment: 5 runs, each with 20 measurements
    // Values are speed of light measurements (deviations from 299,000 km/s)
    // Based on the original D3 example data patterns
    let experiment_data: Vec<Vec<f64>> = vec![
        vec![
            850., 740., 900., 1070., 930., 850., 950., 980., 980., 880., 1000., 980., 930., 650.,
            760., 810., 1000., 1000., 960., 960.,
        ],
        vec![
            960., 940., 960., 940., 880., 800., 850., 880., 900., 840., 830., 790., 810., 880.,
            880., 830., 800., 790., 760., 800.,
        ],
        vec![
            880., 880., 880., 860., 720., 720., 620., 860., 970., 950., 880., 910., 850., 870.,
            840., 840., 850., 840., 840., 840.,
        ],
        vec![
            890., 810., 810., 820., 800., 770., 760., 740., 750., 760., 910., 920., 890., 860.,
            880., 720., 840., 850., 850., 780.,
        ],
        vec![
            890., 840., 780., 810., 760., 810., 790., 810., 820., 850., 870., 870., 810., 740.,
            810., 940., 950., 800., 810., 870.,
        ],
    ];

    let mut x = Vec::new();
    let mut y = Vec::new();

    for (exp_idx, measurements) in experiment_data.iter().enumerate() {
        let exp_num = (exp_idx + 1) as f64;
        for &value in measurements {
            x.push(exp_num);
            y.push(value);
        }
    }

    (x, y)
}

/// Generate 2D grid data with Gaussian peaks
pub(super) fn generate_grid_data(size: usize) -> Vec<f64> {
    let mut z = vec![0.0; size * size];
    for j in 0..size {
        for i in 0..size {
            let x = (i as f64 / size as f64) * 4.0 - 2.0;
            let y = (j as f64 / size as f64) * 4.0 - 2.0;
            // Two Gaussian peaks
            let peak1 = (-((x - 0.5).powi(2) + (y - 0.5).powi(2)) / 0.5).exp();
            let peak2 = 0.7 * (-((x + 0.8).powi(2) + (y + 0.8).powi(2)) / 0.3).exp();
            z[j * size + i] = peak1 + peak2;
        }
    }
    z
}
