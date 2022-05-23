use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

struct SampleRequestOptions {
    sample_rate: f32,
    sample_clock: f32,
    nchannels: usize,
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
        use std::f32::consts::{PI, TAU};
        let period = self.sample_clock / self.sample_rate;
        if period % 1.0 < 0.1 {
            println!("{} {}", freq, period);
        }
        match wave {
            Waveform::Sine => (freq * period * TAU).sin(),
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
    let stream = stream_setup_for(sample_next)?;
    stream.play()?;
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}

fn sample_next(o: &mut SampleRequestOptions, d: &SampleRequestData) -> f32 {
    let offset = match d.channel {
        0 => -1.0,
        _ => 1.0,
    } * 0.1;
    let period = o.sample_clock / o.sample_rate;
    let detune = (period * 10.0).sin();
    let mut output = 0.0;
    /*output += o.tone(80.0 + detune, Waveform::Sine) * 0.5;
    output += o.tone(160.0 + detune, Waveform::Sine);
    output += o.tone(380.546 + offset + detune, Waveform::Triangle);
    output += o.tone(479.458 - offset + detune, Waveform::Triangle);
    output += o.tone(570.175 + offset + detune, Waveform::Triangle);
    output += o.tone(718.376 + offset + detune, Waveform::Triangle);*/
    //output * 0.1
    o.tone(210.0 + period, Waveform::Sine) * 0.1 + o.tone(218.0, Waveform::Sine) * 0.1
}

fn stream_setup_for<F>(on_sample: F) -> Result<cpal::Stream>
where
    F: Fn(&mut SampleRequestOptions, &SampleRequestData) -> f32 + Send + 'static + Copy,
{
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => stream_make::<f32, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::I16 => stream_make::<i16, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U16 => stream_make::<u16, _>(&device, &config.into(), on_sample),
    }
}

fn host_device_setup() -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or(anyhow!("Default output device is not available"))?;
    println!("Output device: {}", device.name()?);

    let _config = device.default_output_config()?;
    let mut formats: Vec<_> = device.supported_output_configs()?.collect();
    formats.sort_by(|a, b| a.cmp_default_heuristics(b));

    let f = formats
        .into_iter()
        .last()
        .ok_or(anyhow!("Stream type not supported"))?;
    let config = f.with_sample_rate(cpal::SampleRate(48000));
    println!("Output config: {:?}", config);

    Ok((host, device, config))
}

fn stream_make<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    on_sample: F,
) -> Result<cpal::Stream>
where
    T: cpal::Sample,
    F: Fn(&mut SampleRequestOptions, &SampleRequestData) -> f32 + Send + 'static + Copy,
{
    let sample_rate = config.sample_rate.0 as f32;
    let sample_clock = 0.0;
    let nchannels = config.channels as usize;
    let mut request = SampleRequestOptions {
        sample_rate,
        sample_clock,
        nchannels,
    };
    let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            on_window(output, &mut request, on_sample)
        },
        err_fn,
    )?;
    Ok(stream)
}

fn on_window<T, F>(output: &mut [T], request: &mut SampleRequestOptions, on_sample: F)
where
    T: cpal::Sample,
    F: Fn(&mut SampleRequestOptions, &SampleRequestData) -> f32 + Send + 'static,
{
    for frame in output.chunks_mut(request.nchannels) {
        request.tick();
        for (channel, sample) in frame.iter_mut().enumerate() {
            let data = SampleRequestData { channel };
            let value: T = cpal::Sample::from::<f32>(&on_sample(request, &data));
            *sample = value;
        }
    }
}
