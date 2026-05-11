use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct AudioReadinessWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug)]
struct WavMetrics {
    peak: f32,
    rms: f32,
    clipped_ratio: f32,
}

pub fn analyze_wav(audio_data: &[u8], office_mode: bool) -> Vec<AudioReadinessWarning> {
    let Ok(metrics) = parse_wav_metrics(audio_data) else {
        return Vec::new();
    };

    let mut warnings = Vec::new();
    let low_peak_threshold = 0.02;
    let silence_rms_threshold = if office_mode { 0.006 } else { 0.003 };
    let low_level_rms_threshold = if office_mode { 0.025 } else { 0.015 };

    if metrics.peak < low_peak_threshold || metrics.rms < silence_rms_threshold {
        warnings.push(AudioReadinessWarning {
            code: "no_speech_detected",
            message: if office_mode {
                "Office Mode: very little direct speech was detected.".to_string()
            } else {
                "Mic check: very little speech was detected.".to_string()
            },
        });
        return warnings;
    }

    if metrics.rms < low_level_rms_threshold {
        warnings.push(AudioReadinessWarning {
            code: "low_level",
            message: if office_mode {
                "Office Mode: your voice is low for a noisy room; move the mic closer or raise gain."
                    .to_string()
            } else {
                "Mic check: input level is low; move the mic closer or raise gain.".to_string()
            },
        });
    }

    if metrics.clipped_ratio > 0.002 {
        warnings.push(AudioReadinessWarning {
            code: "clipping",
            message: "Mic check: input is clipping; lower mic gain a little.".to_string(),
        });
    }

    warnings
}

fn parse_wav_metrics(audio_data: &[u8]) -> Result<WavMetrics, &'static str> {
    if audio_data.len() < 44 || &audio_data[0..4] != b"RIFF" || &audio_data[8..12] != b"WAVE" {
        return Err("invalid wav header");
    }

    let mut sample_rate = 0;
    let mut channels = 0;
    let mut bits_per_sample = 0;
    let mut data_start = 0;
    let mut data_size = 0;
    let mut position = 12;

    while position + 8 <= audio_data.len() {
        let chunk_id = &audio_data[position..position + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[position + 4],
            audio_data[position + 5],
            audio_data[position + 6],
            audio_data[position + 7],
        ]) as usize;
        let chunk_start = position + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size);
        if chunk_end > audio_data.len() {
            return Err("invalid wav chunk");
        }

        match chunk_id {
            b"fmt " if chunk_size >= 16 => {
                channels =
                    u16::from_le_bytes([audio_data[chunk_start + 2], audio_data[chunk_start + 3]]);
                sample_rate = u32::from_le_bytes([
                    audio_data[chunk_start + 4],
                    audio_data[chunk_start + 5],
                    audio_data[chunk_start + 6],
                    audio_data[chunk_start + 7],
                ]);
                bits_per_sample = u16::from_le_bytes([
                    audio_data[chunk_start + 14],
                    audio_data[chunk_start + 15],
                ]);
            }
            b"data" => {
                data_start = chunk_start;
                data_size = chunk_size;
            }
            _ => {}
        }

        position = chunk_end + usize::from(chunk_size % 2 == 1);
    }

    if channels == 0 || sample_rate == 0 || bits_per_sample != 16 || data_size == 0 {
        return Err("unsupported wav format");
    }

    let bytes_per_frame = channels as usize * 2;
    if bytes_per_frame == 0 {
        return Err("invalid wav channels");
    }

    let mut samples = Vec::with_capacity(data_size / bytes_per_frame);
    for frame in audio_data[data_start..data_start + data_size].chunks_exact(bytes_per_frame) {
        let mut sum = 0.0;
        for channel in 0..channels as usize {
            let offset = channel * 2;
            let sample = i16::from_le_bytes([frame[offset], frame[offset + 1]]) as f32 / 32768.0;
            sum += sample;
        }
        samples.push(sum / channels as f32);
    }

    if samples.is_empty() {
        return Err("empty wav data");
    }

    let mut peak = 0.0f32;
    let mut sum_squares = 0.0f32;
    let mut clipped_samples = 0usize;
    for sample in &samples {
        let absolute = sample.abs();
        peak = peak.max(absolute);
        sum_squares += sample * sample;
        if absolute >= 0.98 {
            clipped_samples += 1;
        }
    }

    Ok(WavMetrics {
        rms: (sum_squares / samples.len() as f32).sqrt(),
        clipped_ratio: clipped_samples as f32 / samples.len() as f32,
        peak,
    })
}

#[cfg(test)]
mod tests {
    use super::analyze_wav;

    #[test]
    fn warns_for_near_silent_audio() {
        let warnings = analyze_wav(&create_wav(&vec![0; 16_000]), false);

        assert!(warnings
            .iter()
            .any(|warning| warning.code == "no_speech_detected"));
    }

    #[test]
    fn warns_for_clipping() {
        let samples = vec![i16::MAX; 16_000];
        let warnings = analyze_wav(&create_wav(&samples), false);

        assert!(warnings.iter().any(|warning| warning.code == "clipping"));
    }

    #[test]
    fn accepts_normal_speech_like_audio() {
        let samples = (0..16_000)
            .map(|index| {
                let phase = index as f32 / 16_000.0 * 440.0 * std::f32::consts::TAU;
                (phase.sin() * 8_000.0) as i16
            })
            .collect::<Vec<_>>();

        assert!(analyze_wav(&create_wav(&samples), false).is_empty());
    }

    #[test]
    fn office_mode_warns_on_marginal_level() {
        let samples = (0..16_000)
            .map(|index| {
                let phase = index as f32 / 16_000.0 * 440.0 * std::f32::consts::TAU;
                (phase.sin() * 800.0) as i16
            })
            .collect::<Vec<_>>();

        assert!(analyze_wav(&create_wav(&samples), true)
            .iter()
            .any(|warning| warning.code == "low_level"));
    }

    fn create_wav(samples: &[i16]) -> Vec<u8> {
        let sample_rate = 16_000u32;
        let channels = 1u16;
        let bits_per_sample = 16u16;
        let data_size = samples.len() as u32 * 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(
            &(sample_rate * channels as u32 * bits_per_sample as u32 / 8).to_le_bytes(),
        );
        wav.extend_from_slice(&(channels * bits_per_sample / 8).to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }
}
