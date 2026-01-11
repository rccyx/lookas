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
    dsp::{ema_tc, hann, prepare_fft_input_inplace},
    filterbank::build_filterbank,
    render::{layout_for, render_blocks_vertical_frame},
    utils::scopeguard,
};
use rustfft::FftPlanner;
use std::{
    io::{stdout, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

fn reset_buf(shared: &Arc<Mutex<SharedBuf>>, cap: usize) {
    if let Ok(mut b) = shared.lock() {
        *b = SharedBuf::new(cap);
    }
}

fn main() -> Result<()> {
    let cfg = Config::load()?;

    let fmin: f32 = cfg.fmin;
    let fmax: f32 = cfg.fmax;
    let target_fps_ms: u64 = cfg.target_fps_ms;
    let fft_size: usize = cfg.fft_size;
    let tau_spec: f32 = cfg.tau_spec;
    let gate_db: f32 = cfg.gate_db;
    let tilt_alpha: f32 = cfg.tilt_alpha;
    let flow_k: f32 = cfg.flow_k;
    let spr_k: f32 = cfg.spr_k;
    let spr_zeta: f32 = cfg.spr_zeta;

    let top_pad: u16 = 0;

    let mut out =
        std::io::BufWriter::with_capacity(1024 * 1024, stdout());
    terminal::enable_raw_mode()?;
    execute!(
        out,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All),
        SetForegroundColor(Color::White),
    )?;
    out.flush()?;

    let _cleanup = scopeguard::guard((), |_| {
        let mut out = stdout();
        let _ = execute!(
            out,
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    });

    let ring_cap =
        ((48_000usize / 10).max(fft_size * 3)).max(fft_size * 6);
    let mic_shared = Arc::new(Mutex::new(SharedBuf::new(ring_cap)));
    let sys_shared = Arc::new(Mutex::new(SharedBuf::new(ring_cap)));

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

    let mut sr_u32 = audio.info().sample_rate;
    let mut sr = sr_u32 as f32;

    let window = hann(fft_size);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);
    let half = fft_size / 2;

    let mut last = Instant::now();
    let target_dt = Duration::from_millis(target_fps_ms);
    let mut analyzer = SpectrumAnalyzer::new(half);

    let mut buf = Vec::with_capacity(fft_size);
    let mut spec_pow = vec![0.0; half];

    let mut mic_tail: Vec<f32> = Vec::with_capacity(fft_size);
    let mut sys_tail: Vec<f32> = Vec::with_capacity(fft_size);
    let mut mix: Vec<f32> = vec![0.0f32; fft_size];

    let (mut w, mut h) = terminal::size()?;
    let mut lay = layout_for(w, h, top_pad);
    let mut frame: Vec<u8> = Vec::with_capacity(
        (w as usize * h as usize * 4).max(64 * 1024),
    );

    // gate smoothing + hysteresis + close-confirm.
    // the close-confirm is what removes the “down, up, down” spike on stop.
    let mut gate_pow_ema: f32 = 0.0;
    let gate_attack_s: f32 = 0.012;
    let gate_release_s: f32 = 0.22;

    let gate_open_db: f32 = gate_db;
    let gate_close_db: f32 = (gate_db - 3.0).max(-80.0);

    let mut gate_state: bool = false;
    let mut gate_below_s: f32 = 0.0;
    let gate_close_confirm_s: f32 = 0.12;

    loop {
        let mut layout_dirty = false;

        if event::poll(Duration::ZERO)? {
            match event::read()? {
                event::Event::Resize(nw, nh) => {
                    w = nw;
                    h = nh;
                    layout_dirty = true;
                }
                event::Event::Key(k) => {
                    use crossterm::event::KeyCode::*;
                    match k.code {
                        Char('q') => return Ok(()),
                        Char('1') => {
                            reset_buf(&mic_shared, ring_cap);
                            reset_buf(&sys_shared, ring_cap);
                            audio.start(
                                AudioMode::Mic,
                                mic_shared.clone(),
                                sys_shared.clone(),
                            )?;
                        }
                        Char('2') => {
                            reset_buf(&mic_shared, ring_cap);
                            reset_buf(&sys_shared, ring_cap);
                            audio.start(
                                AudioMode::System,
                                mic_shared.clone(),
                                sys_shared.clone(),
                            )?;
                        }
                        Char('3') => {
                            reset_buf(&mic_shared, ring_cap);
                            reset_buf(&sys_shared, ring_cap);
                            audio.start(
                                AudioMode::Both,
                                mic_shared.clone(),
                                sys_shared.clone(),
                            )?;
                        }
                        Char('r') => {
                            reset_buf(&mic_shared, ring_cap);
                            reset_buf(&sys_shared, ring_cap);
                            audio.reset(
                                mic_shared.clone(),
                                sys_shared.clone(),
                            )?;
                        }
                        _ => {}
                    }

                    let new_sr = audio.info().sample_rate;
                    if new_sr != sr_u32 {
                        sr_u32 = new_sr;
                        sr = sr_u32 as f32;
                        analyzer.filters.clear();
                    }

                    gate_pow_ema = 0.0;
                    gate_state = false;
                    gate_below_s = 0.0;
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last);
        if dt < target_dt {
            thread::sleep(target_dt - dt);
        }
        let now = Instant::now();
        let dt_s = now.duration_since(last).as_secs_f32();
        last = now;

        if layout_dirty {
            lay = layout_for(w, h, top_pad);
            queue!(out, terminal::Clear(ClearType::All),)?;
            out.flush()?;
        }

        let desired_bars = lay.bars;

        if analyzer.filters.len() != desired_bars {
            analyzer.filters = build_filterbank(
                sr,
                fft_size,
                desired_bars,
                fmin,
                fmax,
            );
            analyzer.resize(desired_bars);
        }

        let mic_ok = mic_shared
            .try_lock()
            .ok()
            .map(|b| b.copy_last_n_into(fft_size, &mut mic_tail))
            .unwrap_or(false);

        let sys_ok = sys_shared
            .try_lock()
            .ok()
            .map(|b| b.copy_last_n_into(fft_size, &mut sys_tail))
            .unwrap_or(false);

        let tail: &[f32] = match audio.mode() {
            AudioMode::Mic => {
                if !mic_ok {
                    continue;
                }
                &mic_tail
            }
            AudioMode::System => {
                if !sys_ok {
                    continue;
                }
                &sys_tail
            }
            AudioMode::Both => {
                if !mic_ok || !sys_ok {
                    continue;
                }
                for i in 0..fft_size {
                    mix[i] = (mic_tail[i] + sys_tail[i]) * 0.5;
                }
                &mix
            }
        };

        let mut rms = 0.0f32;
        for &x in tail {
            rms += x * x;
        }
        rms /= fft_size as f32;

        if gate_pow_ema == 0.0 {
            gate_pow_ema = rms;
        } else {
            let tau = if rms > gate_pow_ema {
                gate_attack_s
            } else {
                gate_release_s
            };
            gate_pow_ema = ema_tc(gate_pow_ema, rms, tau, dt_s);
        }

        let rms_db = 10.0 * (gate_pow_ema.max(1e-12)).log10();

        if gate_state {
            if rms_db < gate_close_db {
                gate_below_s += dt_s;
                if gate_below_s >= gate_close_confirm_s {
                    gate_state = false;
                    gate_below_s = 0.0;
                }
            } else {
                gate_below_s = 0.0;
            }
        } else {
            gate_below_s = 0.0;
            if rms_db > gate_open_db {
                gate_state = true;
            }
        }

        let gate_open = gate_state;

        prepare_fft_input_inplace(tail, &window, &mut buf);
        fft.process(&mut buf);

        for i in 0..half {
            let re = buf[i].re;
            let im = buf[i].im;
            spec_pow[i] = (re * re + im * im)
                / (fft_size as f32 * fft_size as f32);
        }

        analyzer.update_spectrum(&spec_pow, tau_spec, dt_s);
        let bars_target =
            analyzer.analyze_bands(tilt_alpha, dt_s, gate_open);
        analyzer.apply_flow_and_spring(
            &bars_target,
            &FlowSpringParams {
                flow_k,
                spr_k,
                spr_zeta,
            },
            dt_s,
            gate_open,
        );

        queue!(out, cursor::MoveTo(0, top_pad))?;
        frame.clear();
        render_blocks_vertical_frame(
            &analyzer.bars_y,
            w,
            h,
            &lay,
            &mut frame,
        )?;
        out.write_all(&frame)?;
        out.flush()?;
    }
}
