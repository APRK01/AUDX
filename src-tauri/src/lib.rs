use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Window};

const FFT_SIZE: usize = 2048;
const NUM_BARS: usize = 64;
const MIN_FREQ: f32 = 20.0;
const MAX_FREQ: f32 = 20000.0;
const SMOOTHING_RISE: f32 = 0.5;
const SMOOTHING_FALL: f32 = 0.85;
const SENSITIVITY: f32 = 1.5;

fn log_scale(value: f32, min: f32, max: f32) -> f32 {
    let log_min = min.max(1.0).ln();
    let log_max = max.ln();
    let log_val = value.max(1.0).ln();
    (log_val - log_min) / (log_max - log_min)
}

fn get_bar_frequencies(num_bars: usize) -> Vec<(f32, f32)> {
    let mut frequencies = Vec::with_capacity(num_bars);
    for i in 0..num_bars {
        let start_ratio = i as f32 / num_bars as f32;
        let end_ratio = (i + 1) as f32 / num_bars as f32;
        
        let start_freq = MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(start_ratio);
        let end_freq = MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(end_ratio);
        
        frequencies.push((start_freq, end_freq));
    }
    frequencies
}

struct AudioProcessor {
    prev_bars: Vec<f32>,
    bar_frequencies: Vec<(f32, f32)>,
    hann_window: Vec<f32>,
}

impl AudioProcessor {
    fn new() -> Self {
        let hann: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();

        Self {
            prev_bars: vec![0.0; NUM_BARS],
            bar_frequencies: get_bar_frequencies(NUM_BARS),
            hann_window: hann,
        }
    }

    fn process(&mut self, samples: &[f32], sample_rate: f32, planner: &mut FftPlanner<f32>) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| Complex::new(s * self.hann_window[i], 0.0))
            .collect();

        let fft = planner.plan_fft_forward(FFT_SIZE);
        fft.process(&mut buffer);

        let freq_resolution = sample_rate / FFT_SIZE as f32;
        let magnitude: Vec<f32> = buffer
            .iter()
            .take(FFT_SIZE / 2)
            .map(|c| (c.norm() * 2.0 / FFT_SIZE as f32))
            .collect();

        let mut bars = vec![0.0f32; NUM_BARS];

        for (bar_idx, &(low_freq, high_freq)) in self.bar_frequencies.iter().enumerate() {
            let low_bin = (low_freq / freq_resolution).floor() as usize;
            let high_bin = (high_freq / freq_resolution).ceil() as usize;
            let low_bin = low_bin.max(1).min(magnitude.len() - 1);
            let high_bin = high_bin.max(low_bin + 1).min(magnitude.len());

            let mut sum = 0.0f32;
            let mut count = 0;

            for bin in low_bin..high_bin {
                sum += magnitude[bin];
                count += 1;
            }

            if count > 0 {
                let avg = sum / count as f32;
                let db = 20.0 * (avg.max(1e-10)).log10();
                let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
                
                // Frequency compensation: boost higher frequencies exponentially
                // Bar 0 = 1.0x, Bar 63 = 4.0x boost
                let freq_boost = 1.0 + (bar_idx as f32 / NUM_BARS as f32).powf(1.5) * 3.0;
                
                bars[bar_idx] = (normalized * SENSITIVITY * freq_boost).min(1.5);
            }
        }

        for i in 0..NUM_BARS {
            let target = bars[i];
            let prev = self.prev_bars[i];

            self.prev_bars[i] = if target > prev {
                prev + (target - prev) * SMOOTHING_RISE
            } else {
                prev + (target - prev) * (1.0 - SMOOTHING_FALL)
            };
        }

        self.prev_bars.clone()
    }
}

#[tauri::command]
fn start_audio_listener(window: Window) -> Result<String, String> {
    std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(d) => d,
            None => return,
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(_) => return,
        };

        let sample_rate = config.sample_rate().0 as f32;
        let stream_config: cpal::StreamConfig = config.clone().into();

        let planner = Arc::new(Mutex::new(FftPlanner::<f32>::new()));
        let sample_buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(FFT_SIZE * 2)));
        let processor = Arc::new(Mutex::new(AudioProcessor::new()));

        let process_fn = {
            let window = window.clone();
            let planner = Arc::clone(&planner);
            let buffer = Arc::clone(&sample_buffer);
            let processor = Arc::clone(&processor);

            move |data: &[f32]| {
                let mut buf = buffer.lock().unwrap();
                buf.extend_from_slice(data);

                while buf.len() >= FFT_SIZE {
                    let chunk: Vec<f32> = buf.drain(0..FFT_SIZE).collect();
                    
                    let mut proc = processor.lock().unwrap();
                    let mut plan = planner.lock().unwrap();
                    let bars = proc.process(&chunk, sample_rate, &mut plan);

                    let _ = window.emit("audio-data", bars);
                }
            }
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| process_fn(data),
                |_| {},
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let floats: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                    process_fn(&floats);
                },
                |_| {},
                None,
            ),
            _ => return,
        };

        if let Ok(s) = stream {
            let _ = s.play();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    });

    Ok("started".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_audio_listener])
        .run(tauri::generate_context!())
        .expect("failed to run");
}
