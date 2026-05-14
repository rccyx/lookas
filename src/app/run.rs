use anyhow::Result;
use crossterm::{
    cursor, event, execute, queue,
    style::{Color, SetForegroundColor},
    terminal::{self, ClearType},
};
use lookas::{
    analyzer::{FlowSpringParams, SpectrumAnalyzer},
    audio::{AudioController, AudioMode},
    buffer::SharedBuf,
    config::Config,
    dsp::hann,
    filterbank::{build_filterbank, FilterbankParams},
    render::{draw_blocks_vertical, layout_for, Layout},
    utils::scopeguard,
};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::{
    io::{stdout, BufWriter, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use super::{
    fft::{compute_spectrum, FftContext},
    gate::GateState,
    input::{handle_key, InputContext},
    mix::compute_rms,
};

#[allow(clippy::arithmetic_side_effects)]
fn ring_cap(fft_size: usize) -> usize {
    ((48_000usize / 10).max(fft_size * 3))
        .max(fft_size * 6)
        .next_power_of_two()
}

struct AppResources {
    fft_size: usize,
    cap: usize,
    audio: AudioController,
    mic_shared: Arc<Mutex<SharedBuf>>,
    sys_shared: Arc<Mutex<SharedBuf>>,
    sr_u32: u32,
    sr: f32,
}

struct FrameState {
    analyzer: SpectrumAnalyzer,
    gate: GateState,
    window: Vec<f32>,
    half: usize,
    fft: Arc<dyn RealToComplex<f32>>,
    buf: Vec<f32>,
    fft_out: Vec<Complex<f32>>,
    spec_pow: Vec<f32>,
    mic_tail: Vec<f32>,
    sys_tail: Vec<f32>,
    mix: Vec<f32>,
    lay: Layout,
    frame: Vec<u8>,
    w: u16,
    h: u16,
    top_pad: u16,
    dt_s: f32,
}

fn init_audio(cfg: &Config) -> Result<AppResources> {
    let fft_size = cfg.fft_size;
    let cap = ring_cap(fft_size);
    let mic_shared = Arc::new(Mutex::new(SharedBuf::new(cap)));
    let sys_shared = Arc::new(Mutex::new(SharedBuf::new(cap)));

    let mut audio = AudioController::new();
    if audio
        .start(
            AudioMode::System,
            mic_shared.clone(),
            sys_shared.clone(),
        )
        .is_err()
    {
        audio.start(
            AudioMode::Mic,
            mic_shared.clone(),
            sys_shared.clone(),
        )?;
    }

    let sr_u32 = audio.info().sample_rate;
    #[allow(clippy::cast_precision_loss)]
    let sr = sr_u32 as f32;

    Ok(AppResources {
        fft_size,
        cap,
        audio,
        mic_shared,
        sys_shared,
        sr_u32,
        sr,
    })
}

fn init_frame(
    cfg: &Config,
    res: &AppResources,
    w: u16,
    h: u16,
) -> FrameState {
    let fft_size = res.fft_size;
    let top_pad: u16 = 0;
    let half = fft_size / 2;
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let buf = fft.make_input_vec();
    let fft_out = fft.make_output_vec();

    #[allow(clippy::arithmetic_side_effects)]
    let frame_cap = (w as usize * h as usize * 4).max(64 * 1024);

    FrameState {
        analyzer: SpectrumAnalyzer::new(half),
        gate: GateState {
            pow_ema: 0.0,
            open: false,
            below_s: 0.0,
            attack_s: 0.012,
            release_s: 0.22,
            open_db: cfg.gate_db,
            close_db: (cfg.gate_db - 3.0).max(-80.0),
            confirm_s: 0.12,
        },
        window: hann(fft_size),
        half,
        fft,
        buf,
        fft_out,
        spec_pow: vec![0.0f32; half],
        mic_tail: Vec::with_capacity(fft_size),
        sys_tail: Vec::with_capacity(fft_size),
        mix: vec![0.0f32; fft_size],
        lay: layout_for(w, h, top_pad),
        frame: Vec::with_capacity(frame_cap),
        w,
        h,
        top_pad,
        dt_s: 0.0,
    }
}

fn tick<W: Write>(
    fs: &mut FrameState,
    res: &AppResources,
    cfg: &Config,
    out: &mut W,
) -> Result<()> {
    let mic_ok = res.mic_shared.try_lock().ok().is_some_and(|b| {
        b.copy_last_n_into(res.fft_size, &mut fs.mic_tail)
    });
    let sys_ok = res.sys_shared.try_lock().ok().is_some_and(|b| {
        b.copy_last_n_into(res.fft_size, &mut fs.sys_tail)
    });

    let tail: Option<&[f32]> = match res.audio.mode() {
        AudioMode::Mic => mic_ok.then_some(&fs.mic_tail),
        AudioMode::System => sys_ok.then_some(&fs.sys_tail),
        AudioMode::Both => {
            if !mic_ok || !sys_ok {
                None
            } else {
                #[allow(clippy::indexing_slicing)]
                for i in 0..res.fft_size {
                    fs.mix[i] =
                        (fs.mic_tail[i] + fs.sys_tail[i]) * 0.5;
                }
                Some(&fs.mix)
            }
        }
    };

    let Some(tail) = tail else {
        return Ok(());
    };

    fs.gate.tick(compute_rms(tail, res.fft_size), fs.dt_s);

    compute_spectrum(&mut FftContext {
        tail,
        window: &fs.window,
        buf: &mut fs.buf,
        fft_out: &mut fs.fft_out,
        fft: &fs.fft,
        spec_pow: &mut fs.spec_pow,
        half: fs.half,
        fft_size: res.fft_size,
    });

    fs.analyzer
        .update_spectrum(&fs.spec_pow, cfg.tau_spec, fs.dt_s);
    fs.analyzer.analyze_bands(fs.dt_s, fs.gate.open);
    fs.analyzer.apply_flow_and_spring(
        &FlowSpringParams {
            flow_k: cfg.flow_k,
            spr_k: cfg.spr_k,
            spr_zeta: cfg.spr_zeta,
        },
        fs.dt_s,
        fs.gate.open,
    );

    queue!(out, cursor::MoveTo(0, fs.top_pad))?;
    fs.frame.clear();
    draw_blocks_vertical(
        &mut fs.frame,
        &fs.analyzer.bars_y,
        fs.w,
        fs.h,
        &fs.lay,
        &mut fs.analyzer.render_fulls,
        &mut fs.analyzer.render_fracs,
    )?;
    out.write_all(&fs.frame)?;
    out.flush()?;
    Ok(())
}

pub fn run() -> Result<()> {
    let cfg = Config::load()?;

    let mut out = BufWriter::with_capacity(1024 * 1024, stdout());
    terminal::enable_raw_mode()?;
    execute!(
        out,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All),
        SetForegroundColor(Color::White),
    )?;
    out.flush()?;

    let _cleanup = scopeguard::guard((), |()| {
        let mut o = stdout();
        let _ =
            execute!(o, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    });

    let mut res = init_audio(&cfg)?;
    let (w, h) = terminal::size()?;
    let mut fs = init_frame(&cfg, &res, w, h);
    let target_dt = Duration::from_millis(cfg.frame_ms);
    let mut last = Instant::now();

    loop {
        let mut layout_dirty = false;

        if event::poll(Duration::ZERO)? {
            match event::read()? {
                event::Event::Resize(nw, nh) => {
                    fs.w = nw;
                    fs.h = nh;
                    layout_dirty = true;
                }
                event::Event::Key(k) => {
                    let mut ctx = InputContext {
                        audio: &mut res.audio,
                        mic_shared: &res.mic_shared,
                        sys_shared: &res.sys_shared,
                        ring_cap: res.cap,
                    };
                    if handle_key(k.code, &mut ctx)? {
                        return Ok(());
                    }
                    let new_sr = res.audio.info().sample_rate;
                    if new_sr != res.sr_u32 {
                        res.sr_u32 = new_sr;
                        #[allow(clippy::cast_precision_loss)]
                        {
                            res.sr = res.sr_u32 as f32;
                        }
                        fs.analyzer.filters.clear();
                    }
                    fs.gate.reset();
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last);
        if dt < target_dt {
            if let Some(diff) = target_dt.checked_sub(dt) {
                thread::sleep(diff);
            }
        }
        let now = Instant::now();
        let dt_s = now.duration_since(last).as_secs_f32();
        last = now;

        if layout_dirty {
            fs.lay = layout_for(fs.w, fs.h, fs.top_pad);
            queue!(out, terminal::Clear(ClearType::All),)?;
            out.flush()?;
        }

        if fs.analyzer.filters.len() != fs.lay.bars {
            fs.analyzer.filters =
                build_filterbank(FilterbankParams {
                    sr: res.sr,
                    fft_size: res.fft_size,
                    bands: fs.lay.bars,
                    fmin: cfg.fmin,
                    fmax: cfg.fmax,
                });
            fs.analyzer.resize(fs.lay.bars);
        }

        fs.dt_s = dt_s;
        tick(&mut fs, &res, &cfg, &mut out)?;
    }
}
