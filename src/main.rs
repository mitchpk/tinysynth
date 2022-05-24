use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::{PI, TAU};

struct Note {
    freq: f32,
    clock: f32,
}

impl From<f32> for Note {
    fn from(freq: f32) -> Self {
        Note { freq, clock: 0.0 }
    }
}

struct SampleRequestOptions {
    sample_rate: f32,
    sample_clock: f32,
    nchannels: usize,
    notes: Vec<Note>,
}

struct SampleRequestData {
    channel: usize,
}

#[allow(dead_code)]
enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl SampleRequestOptions {
    fn tick(&mut self) {
        self.sample_clock = self.sample_clock + 1.0;
    }
    fn tone(&self, freq: f32, wave: Waveform) -> f32 {
        let period = self.sample_clock / self.sample_rate;
        let phase = freq * period;
        match wave {
            Waveform::Sine => (phase * TAU).sin(),
            Waveform::Square => match (freq * TAU * period).sin() {
                i if i > 0.0 => 1.0,
                _ => -1.0,
            },
            Waveform::Triangle => (freq * TAU * period).sin().asin() * (2.0 / PI),
            Waveform::Sawtooth => ((freq * period) % 1.0 - 0.5) * 2.0,
        }
    }
}

fn main() -> Result<()> {
    let stream = stream_setup()?;
    stream.play()?;
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}

fn sample_next(o: &mut SampleRequestOptions, d: &SampleRequestData) -> f32 {
    let offset = match d.channel {
        0 => -1.0,
        _ => 1.0,
    } * 0.1;
    let mut output = 0.0;
    for n in &o.notes {
        output += (n.freq * (n.clock / o.sample_rate) * TAU).sin();
    }
    //output += o.tone(80.0, Waveform::Sine) * 0.5;
    //output += o.tone(160.0 + offset, Waveform::Sine);
    //output += o.tone(380.546 - offset, Waveform::Triangle);
    //output += o.tone(479.458 + offset, Waveform::Triangle);
    //output += o.tone(570.175 - offset, Waveform::Triangle);
    //output += o.tone(718.376 + offset, Waveform::Triangle);
    output * 0.2 / o.notes.len() as f32
}

fn stream_setup() -> Result<cpal::Stream> {
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => stream_make::<f32>(&device, &config.into()),
        cpal::SampleFormat::I16 => stream_make::<i16>(&device, &config.into()),
        cpal::SampleFormat::U16 => stream_make::<u16>(&device, &config.into()),
    }
}

fn host_device_setup() -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .context("Default output device is not available")?;
    println!("Output device: {}", device.name()?);

    let _config = device.default_output_config()?;
    let mut formats: Vec<_> = device.supported_output_configs()?.collect();
    formats.sort_by(|a, b| a.cmp_default_heuristics(b));

    let f = formats
        .into_iter()
        .last()
        .context("Stream type not supported")?;
    let config = f.with_max_sample_rate();
    println!("Output config: {:?}", config);

    Ok((host, device, config))
}

fn stream_make<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<cpal::Stream>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let sample_clock = 0.0;
    let nchannels = config.channels as usize;
    let notes: Vec<Note> = vec![
        80.0.into(),
        160.0.into(),
        380.546.into(),
        479.458.into(),
        570.175.into(),
        718.376.into(),
    ];
    let mut request = SampleRequestOptions {
        sample_rate,
        sample_clock,
        nchannels,
        notes,
    };
    let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| on_window(output, &mut request),
        err_fn,
    )?;
    Ok(stream)
}

fn on_window<T>(output: &mut [T], request: &mut SampleRequestOptions)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(request.nchannels) {
        for n in &mut request.notes {
            n.clock = (n.clock + 1.0) % (request.sample_rate / n.freq);
        }
        request.tick();
        for (channel, sample) in frame.iter_mut().enumerate() {
            let data = SampleRequestData { channel };
            let value: T = cpal::Sample::from::<f32>(&sample_next(request, &data));
            *sample = value;
        }
    }
}
