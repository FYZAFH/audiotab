use rustfft::{FftPlanner, num_complex::Complex};
use std::f64::consts::PI;

pub fn compute_stft(
    samples: &[f64],
    window_size: usize,
    hop_size: usize,
) -> Vec<f64> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let num_windows = (samples.len().saturating_sub(window_size)) / hop_size + 1;
    let num_bins = window_size / 2 + 1;

    let mut result = Vec::with_capacity(num_windows * num_bins);
    let hann_window = create_hann_window(window_size);

    for i in 0..num_windows {
        let start = i * hop_size;
        let end = start + window_size;

        if end > samples.len() {
            break;
        }

        // Apply window function
        let mut windowed: Vec<Complex<f64>> = samples[start..end]
            .iter()
            .zip(hann_window.iter())
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        // Compute FFT
        fft.process(&mut windowed);

        // Compute magnitude spectrum (dB)
        for bin in windowed.iter().take(num_bins) {
            let magnitude = bin.norm();
            let db = 20.0 * (magnitude + 1e-10).log10();
            result.push(db);
        }
    }

    result
}

fn create_hann_window(size: usize) -> Vec<f64> {
    (0..size)
        .map(|i| 0.5 * (1.0 - ((2.0 * PI * i as f64) / (size - 1) as f64).cos()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let window = create_hann_window(4);
        assert_eq!(window.len(), 4);
        assert!(window[0] < 0.1); // First value near 0
        assert!(window[2] > 0.7 && window[2] < 0.8); // Middle value ~0.75
    }

    #[test]
    fn test_stft_dimensions() {
        let samples: Vec<f64> = (0..8192).map(|_| 0.0).collect();
        let window_size = 2048;
        let hop_size = 512;

        let result = compute_stft(&samples, window_size, hop_size);

        let num_bins = window_size / 2 + 1;
        let num_windows = (samples.len() - window_size) / hop_size + 1;

        assert_eq!(result.len(), num_windows * num_bins);
    }
}
