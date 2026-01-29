use anyhow::{Context, Result};
use rustfft::{num_complex::Complex, FftPlanner};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::Signal;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const WINDOW_SIZE: usize = 4096;
const HOP_SIZE: usize = 2048;
const TARGET_ZONE_SIZE: usize = 5;
const ANCHOR_OFFSET: usize = 1;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioHash {
    pub hash: u64,
    pub time_offset: u32,
}

pub fn compute_audio_fingerprints(audio_path: &Path) -> Result<Vec<AudioHash>> {
    let src = File::open(audio_path).context("failed to open audio")?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let hint = Hint::new();

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("unsupported audio format")?;

    let mut format = probed.format;
    let track = format.default_track().context("no track found")?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("unsupported codec")?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(e) => return Err(e.into()),
        };

        let decoded = decoder.decode(&packet)?; // Convert to mono f32
        let mut spec = *decoded.spec();
        let _duration = decoded.capacity();

        use symphonia::core::audio::AudioBufferRef;
        match decoded {
            AudioBufferRef::F32(buf) => {
                for i in 0..buf.frames() {
                    let s = buf.chan(0)[i];
                    samples.push(s);
                }
            }
            AudioBufferRef::U8(buf) => {
                for i in 0..buf.frames() {
                    let s = (buf.chan(0)[i] as f32 - 128.0) / 128.0;
                    samples.push(s);
                }
            }
            AudioBufferRef::S16(buf) => {
                for i in 0..buf.frames() {
                    let s = (buf.chan(0)[i] as f32) / 32768.0;
                    samples.push(s);
                }
            }
            _ => { /* Handle other formats if needed */ }
        }
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);

    let mut peaks: Vec<(usize, usize)> = Vec::new();

    let num_windows = (samples.len() - WINDOW_SIZE) / HOP_SIZE;

    for w in 0..num_windows {
        let start = w * HOP_SIZE;
        let end = start + WINDOW_SIZE;
        let window = &samples[start..end];

        let mut buffer: Vec<Complex<f32>> =
            window.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

        fft.process(&mut buffer);
        // Simple strategy: Divide into bands and find max in each.
        // Bands: Low (10-20), Mid (20-40), High (40-80), etc. bins?
        // 44100Hz / 4096 = ~10.7Hz per bin.
        let bands = vec![(10, 40), (40, 80), (80, 160), (160, 511)];

        for (min_bin, max_bin) in bands {
            let mut max_mag = 0.0;
            let mut max_idx = 0;

            for i in min_bin..max_bin {
                let mag = buffer[i].norm(); // Magnitude
                if mag > max_mag {
                    max_mag = mag;
                    max_idx = i;
                }
            }

            if max_mag > 10.0 {
                peaks.push((w, max_idx));
            }
        }
    }

    let mut hashes = Vec::new();

    for i in 0..peaks.len() {
        let (t1, f1) = peaks[i];

        for j in (i + 1)..peaks.len() {
            let (t2, f2) = peaks[j];
            let dt = t2 - t1;

            if dt < ANCHOR_OFFSET {
                continue;
            }
            if dt > TARGET_ZONE_SIZE + ANCHOR_OFFSET {
                break;
            }

            let hash = ((f1 as u64) << 23) | ((f2 as u64) << 9) | (dt as u64);

            hashes.push(AudioHash {
                hash,
                time_offset: t1 as u32,
            });
        }
    }

    Ok(hashes)
}
