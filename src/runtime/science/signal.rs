//! FFT + SVD subset (SC1c/d).

use super::helpers::{float_out, int_out, matrix_at, matrix_out, num, num_at, require_square, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

/// Cooley–Tukey radix-2 FFT on real input → interleaved [re0,im0,re1,im1,…].
fn fft_radix2(re: &mut [f64], im: &mut [f64]) -> Result<(), String> {
    let n = re.len();
    if n == 0 || (n & (n - 1)) != 0 {
        return Err("fft: length must be power of two".into());
    }
    if im.len() != n {
        return Err("fft: re/im length mismatch".into());
    }
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wlen_re, wlen_im) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut wr, mut wi) = (1.0, 0.0);
            for k in 0..half {
                let u_re = re[i + k];
                let u_im = im[i + k];
                let v_re = re[i + k + half] * wr - im[i + k + half] * wi;
                let v_im = re[i + k + half] * wi + im[i + k + half] * wr;
                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;
                re[i + k + half] = u_re - v_re;
                im[i + k + half] = u_im - v_im;
                let nwr = wr * wlen_re - wi * wlen_im;
                wi = wr * wlen_im + wi * wlen_re;
                wr = nwr;
            }
        }
        len *= 2;
    }
    Ok(())
}

fn num_fft(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_fft")?;
    let mut re = x;
    let mut im = vec![0.0; re.len()];
    fft_radix2(&mut re, &mut im)?;
    let mut out = Vec::with_capacity(re.len() * 2);
    for i in 0..re.len() {
        out.push(re[i]);
        out.push(im[i]);
    }
    Ok(vector_out(&out))
}

fn num_ifft(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let interleaved = vector_at(args, 0, "num_ifft")?;
    if interleaved.len() % 2 != 0 {
        return Err("num_ifft: expected interleaved complex".into());
    }
    let n = interleaved.len() / 2;
    let mut re = Vec::with_capacity(n);
    let mut im = Vec::with_capacity(n);
    for i in 0..n {
        re.push(interleaved[2 * i]);
        im.push(-interleaved[2 * i + 1]);
    }
    fft_radix2(&mut re, &mut im)?;
    let scale = 1.0 / n as f64;
    for v in &mut re {
        *v *= scale;
    }
    Ok(vector_out(&re))
}

/// Complex FFT: interleaved [re,im,…] → same length interleaved spectrum.
fn num_fft_c(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let interleaved = vector_at(args, 0, "num_fft_c")?;
    if interleaved.len() % 2 != 0 {
        return Err("num_fft_c: expected interleaved complex".into());
    }
    let n = interleaved.len() / 2;
    let mut re = Vec::with_capacity(n);
    let mut im = Vec::with_capacity(n);
    for i in 0..n {
        re.push(interleaved[2 * i]);
        im.push(interleaved[2 * i + 1]);
    }
    fft_radix2(&mut re, &mut im)?;
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        out.push(re[i]);
        out.push(im[i]);
    }
    Ok(vector_out(&out))
}

/// Real FFT: returns n/2+1 complex bins (interleaved), input padded to power-of-two.
fn num_rfft(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_rfft")?;
    if x.is_empty() {
        return Err("num_rfft: empty".into());
    }
    let n = next_pow2(x.len());
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    for (i, v) in x.iter().enumerate() {
        re[i] = *v;
    }
    fft_radix2(&mut re, &mut im)?;
    let bins = n / 2 + 1;
    let mut out = Vec::with_capacity(bins * 2);
    for i in 0..bins {
        out.push(re[i]);
        out.push(im[i]);
    }
    Ok(vector_out(&out))
}

/// Inverse real FFT: spectrum of n/2+1 bins → real signal of length n (= (bins-1)*2).
fn num_irfft(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let half = vector_at(args, 0, "num_irfft")?;
    if half.len() < 2 || half.len() % 2 != 0 {
        return Err("num_irfft: expected interleaved Hermitian spectrum".into());
    }
    let bins = half.len() / 2;
    if bins < 2 {
        return Err("num_irfft: need at least 2 bins".into());
    }
    let n = (bins - 1) * 2;
    if n == 0 || (n & (n - 1)) != 0 {
        return Err("num_irfft: inferred length must be power of two".into());
    }
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    for i in 0..bins {
        re[i] = half[2 * i];
        im[i] = half[2 * i + 1];
    }
    for i in 1..bins.saturating_sub(1) {
        let j = n - i;
        re[j] = re[i];
        im[j] = -im[i];
    }
    // IFFT via conjugate + FFT + conjugate/scale
    for v in &mut im {
        *v = -*v;
    }
    fft_radix2(&mut re, &mut im)?;
    let scale = 1.0 / n as f64;
    for v in &mut re {
        *v *= scale;
    }
    Ok(vector_out(&re))
}

/// Pad real vector to next power of two, then FFT (interleaved full spectrum).
fn num_fft_pad(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_fft_pad")?;
    let n = next_pow2(x.len().max(1));
    let mut padded = vec![0.0; n];
    for (i, v) in x.iter().enumerate() {
        padded[i] = *v;
    }
    num_fft(&[vector_out(&padded)], env)
}

/// num_fftfreq(n, d?) — DFT sample frequencies (numpy-like).
fn num_fftfreq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "num_fftfreq")? as usize;
    if n == 0 {
        return Err("num_fftfreq: n > 0".into());
    }
    let d = args.get(1).and_then(|v| num(v).ok()).unwrap_or(1.0);
    if d.abs() < 1e-15 {
        return Err("num_fftfreq: d too small".into());
    }
    let mut out = vec![0.0; n];
    let val = 1.0 / (n as f64 * d);
    let n2 = (n as i64 + 1) / 2;
    for i in 0..n2 {
        out[i as usize] = i as f64 * val;
    }
    for i in n2..n as i64 {
        out[i as usize] = (i - n as i64) as f64 * val;
    }
    Ok(vector_out(&out))
}

/// Linear resample to new length.
fn num_resample(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_resample")?;
    let n_out = num_at(args, 1, "num_resample")? as usize;
    if x.is_empty() || n_out == 0 {
        return Err("num_resample: empty".into());
    }
    if n_out == 1 {
        return Ok(vector_out(&[x[0]]));
    }
    let mut out = vec![0.0; n_out];
    let last = (x.len() - 1) as f64;
    for i in 0..n_out {
        let t = i as f64 * last / (n_out - 1) as f64;
        let i0 = t.floor() as usize;
        let i1 = (i0 + 1).min(x.len() - 1);
        let f = t - i0 as f64;
        out[i] = x[i0] * (1.0 - f) + x[i1] * f;
    }
    Ok(vector_out(&out))
}

/// Analytic signal via FFT Hilbert transform → interleaved complex.
fn num_hilbert(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_hilbert")?;
    if x.is_empty() {
        return Err("num_hilbert: empty".into());
    }
    let n = next_pow2(x.len());
    let mut padded = vec![0.0; n];
    for (i, v) in x.iter().enumerate() {
        padded[i] = *v;
    }
    let spec = num_fft(&[vector_out(&padded)], env)?;
    let interleaved = vector_at(&[spec], 0, "num_hilbert")?;
    let mut re = Vec::with_capacity(n);
    let mut im = Vec::with_capacity(n);
    for i in 0..n {
        re.push(interleaved[2 * i]);
        im.push(interleaved[2 * i + 1]);
    }
    // Multiply by 2 for positive freqs (except DC/Nyquist), zero negative.
    let mut h = vec![0.0; n];
    h[0] = 1.0;
    if n > 1 {
        h[n / 2] = 1.0;
    }
    for i in 1..n / 2 {
        h[i] = 2.0;
    }
    for i in 0..n {
        re[i] *= h[i];
        im[i] *= h[i];
    }
    // IFFT complex
    for v in &mut im {
        *v = -*v;
    }
    fft_radix2(&mut re, &mut im)?;
    let scale = 1.0 / n as f64;
    let mut out = Vec::with_capacity(x.len() * 2);
    for i in 0..x.len() {
        out.push(re[i] * scale);
        out.push(-im[i] * scale);
    }
    Ok(vector_out(&out))
}

fn num_conv1d(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let signal = vector_at(args, 0, "num_conv1d")?;
    let kernel = vector_at(args, 1, "num_conv1d")?;
    if signal.is_empty() || kernel.is_empty() {
        return Err("num_conv1d: empty".into());
    }
    let out_len = signal.len() + kernel.len() - 1;
    let mut out = vec![0.0; out_len];
    for i in 0..signal.len() {
        for j in 0..kernel.len() {
            out[i + j] += signal[i] * kernel[j];
        }
    }
    Ok(vector_out(&out))
}

fn mat_svd2(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_svd2")?;
    require_square(&m, "mat_svd2")?;
    if m.len() != 2 {
        return Err("mat_svd2: only 2x2 supported".into());
    }
    let a = m[0][0];
    let b = m[0][1];
    let c = m[1][0];
    let d = m[1][1];
    let ata00 = a * a + c * c;
    let ata01 = a * b + c * d;
    let ata11 = b * b + d * d;
    let tr = ata00 + ata11;
    let det = ata00 * ata11 - ata01 * ata01;
    let disc = (tr * tr - 4.0 * det).max(0.0).sqrt();
    let l1 = (tr + disc) / 2.0;
    let l2 = (tr - disc) / 2.0;
    let s1 = l1.max(0.0).sqrt();
    let s2 = l2.max(0.0).sqrt();
    let (v0x, v0y) = eigenvec2(ata00, ata01, l1);
    let (v1x, v1y) = (-v0y, v0x);
    let vt = matrix_out(&[vec![v0x, v0y], vec![v1x, v1y]]);
    let mut u0 = [a * v0x + b * v0y, c * v0x + d * v0y];
    let mut u1 = [a * v1x + b * v1y, c * v1x + d * v1y];
    if s1 > 1e-12 {
        u0[0] /= s1;
        u0[1] /= s1;
    }
    if s2 > 1e-12 {
        u1[0] /= s2;
        u1[1] /= s2;
    }
    let u = matrix_out(&[vec![u0[0], u1[0]], vec![u0[1], u1[1]]]);
    let mut out = HashMap::new();
    out.insert("u".into(), u);
    out.insert("s".into(), vector_out(&[s1, s2]));
    out.insert("vt".into(), vt);
    Ok(Value::from_object(out))
}

fn eigenvec2(a: f64, b: f64, lambda: f64) -> (f64, f64) {
    let x = b;
    let y = lambda - a;
    let n = (x * x + y * y).sqrt();
    if n < 1e-12 {
        (1.0, 0.0)
    } else {
        (x / n, y / n)
    }
}

/// num_window_hann(n)
fn num_window_hann(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "num_window_hann")? as usize;
    if n == 0 {
        return Err("num_window_hann: n > 0".into());
    }
    let out: Vec<f64> = (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0).max(1.0)).cos())
        .collect();
    Ok(vector_out(&out))
}

/// num_window_hamming(n)
fn num_window_hamming(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "num_window_hamming")? as usize;
    if n == 0 {
        return Err("num_window_hamming: n > 0".into());
    }
    let out: Vec<f64> = (0..n)
        .map(|i| {
            0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0).max(1.0)).cos()
        })
        .collect();
    Ok(vector_out(&out))
}

fn next_pow2(n: usize) -> usize {
    let mut p = 1;
    while p < n {
        p *= 2;
    }
    p
}

fn fft_mag(signal: &[f64]) -> Result<Vec<f64>, String> {
    let n = next_pow2(signal.len());
    let mut re: Vec<f64> = signal.iter().copied().chain(std::iter::repeat(0.0)).take(n).collect();
    let mut im = vec![0.0; n];
    fft_radix2(&mut re, &mut im)?;
    let mut mag = Vec::with_capacity(n);
    for i in 0..n {
        mag.push((re[i] * re[i] + im[i] * im[i]).sqrt());
    }
    Ok(mag)
}

/// num_stft(signal, winSize, hop?) → {frames, freqs, data[magnitude rows]}
fn num_stft(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sig = vector_at(args, 0, "num_stft")?;
    let win_size = num_at(args, 1, "num_stft")? as usize;
    let hop = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(win_size as f64 / 2.0)
        .max(1.0) as usize;
    if win_size == 0 || sig.is_empty() {
        return Err("num_stft: invalid".into());
    }
    let window = num_window_hann(&[float_out(win_size as f64)], _env)?;
    let w = vector_at(&[window], 0, "w")?;
    let mut frames = 0usize;
    let mut start = 0usize;
    while start + win_size <= sig.len() {
        frames += 1;
        start += hop;
    }
    if frames == 0 {
        return Err("num_stft: signal shorter than window".into());
    }
    let nfft = next_pow2(win_size);
    let mut rows = Vec::new();
    start = 0;
    for _ in 0..frames {
        let mut chunk = Vec::with_capacity(win_size);
        for i in 0..win_size {
            chunk.push(sig[start + i] * w[i]);
        }
        let mag = fft_mag(&chunk)?;
        rows.push(Value::from_array(mag.iter().map(|v| float_out(*v)).collect()));
        start += hop;
    }
    let mut out = HashMap::new();
    out.insert("frames".into(), int_out(frames as i64));
    out.insert("freqs".into(), int_out(nfft as i64));
    out.insert("data".into(), Value::from_array(rows));
    Ok(Value::from_object(out))
}

/// num_fft2d(matrix) — separable 2D FFT magnitude
fn num_fft2d(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "num_fft2d")?;
    let rows = m.len();
    let cols = m.first().map(|r| r.len()).unwrap_or(0);
    if rows == 0 || cols == 0 {
        return Err("num_fft2d: empty".into());
    }
    let mut data = m.clone();
    for r in 0..rows {
        let mag = fft_mag(&data[r])?;
        for c in 0..cols {
            data[r][c] = mag[c];
        }
    }
    let ncols = next_pow2(cols);
    for c in 0..ncols {
        let col: Vec<f64> = data.iter().map(|row| row.get(c).copied().unwrap_or(0.0)).collect();
        let mag = fft_mag(&col)?;
        for r in 0..rows {
            if c < data[r].len() {
                data[r][c] = mag[r];
            }
        }
    }
    Ok(matrix_out(&data))
}

/// num_fftn(matrix) — 2D nested real matrix: row FFTs then column FFTs (interleaved complex flattened rows).
/// Returns { rows, cols, data } where data[r] is interleaved complex spectrum for row r after 2D FFT.
fn num_fftn(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "num_fftn")?;
    let rows = m.len();
    let cols = m.first().map(|r| r.len()).unwrap_or(0);
    if rows == 0 || cols == 0 {
        return Err("num_fftn: empty".into());
    }
    let nr = next_pow2(rows);
    let nc = next_pow2(cols);
    // Work as complex grid [nr][nc] interleaved in separate re/im mats
    let mut re = vec![vec![0.0; nc]; nr];
    let mut im = vec![vec![0.0; nc]; nr];
    for r in 0..rows {
        for c in 0..cols {
            re[r][c] = m[r][c];
        }
    }
    // FFT along rows
    for r in 0..nr {
        let mut rr = re[r].clone();
        let mut ii = im[r].clone();
        fft_radix2(&mut rr, &mut ii)?;
        re[r] = rr;
        im[r] = ii;
    }
    // FFT along columns
    for c in 0..nc {
        let mut rr: Vec<f64> = (0..nr).map(|r| re[r][c]).collect();
        let mut ii: Vec<f64> = (0..nr).map(|r| im[r][c]).collect();
        fft_radix2(&mut rr, &mut ii)?;
        for r in 0..nr {
            re[r][c] = rr[r];
            im[r][c] = ii[r];
        }
    }
    let mut data_rows = Vec::with_capacity(nr);
    for r in 0..nr {
        let mut row = Vec::with_capacity(nc * 2);
        for c in 0..nc {
            row.push(re[r][c]);
            row.push(im[r][c]);
        }
        data_rows.push(vector_out(&row));
    }
    let mut out = HashMap::new();
    out.insert("rows".into(), int_out(nr as i64));
    out.insert("cols".into(), int_out(nc as i64));
    out.insert("data".into(), Value::from_array(data_rows));
    out.insert("kind".into(), Value::String("fftn".into()));
    Ok(Value::from_object(out))
}

/// num_firwin(numtaps, cutoff) — windowed-sinc lowpass FIR (cutoff in Nyquist units 0..1).
fn num_firwin(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "num_firwin")? as usize;
    let cutoff = num_at(args, 1, "num_firwin")?;
    if n < 1 || cutoff <= 0.0 || cutoff >= 1.0 {
        return Err("num_firwin: numtaps>=1, cutoff in (0,1)".into());
    }
    let m = (n - 1) as f64 / 2.0;
    let mut h = vec![0.0; n];
    let mut sum = 0.0;
    for i in 0..n {
        let x = i as f64 - m;
        let sinc = if x.abs() < 1e-12 {
            2.0 * cutoff
        } else {
            (2.0 * std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * x)
        };
        let w = if n == 1 {
            1.0
        } else {
            0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0)).cos()
        };
        h[i] = sinc * w;
        sum += h[i];
    }
    if sum.abs() > 1e-15 {
        for v in &mut h {
            *v /= sum;
        }
    }
    Ok(vector_out(&h))
}

/// num_butter_biquad(kind, cutoff, q?) — RBJ low/high shelf-less butterworth-ish biquad.
/// kind: "low"|"high"; cutoff in (0,1) Nyquist; returns {b0,b1,b2,a0,a1,a2}.
fn num_butter_biquad(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kind = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("num_butter_biquad(kind, cutoff, q?)".into()),
    };
    let cutoff = num_at(args, 1, "num_butter_biquad")?;
    let q = args.get(2).and_then(|v| num(v).ok()).unwrap_or(std::f64::consts::FRAC_1_SQRT_2);
    if cutoff <= 0.0 || cutoff >= 1.0 || q <= 0.0 {
        return Err("num_butter_biquad: bad params".into());
    }
    let w0 = std::f64::consts::PI * cutoff;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let (b0, b1, b2, a0, a1, a2) = match kind {
        "low" => {
            let b0 = (1.0 - cos_w0) / 2.0;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        "high" => {
            let b0 = (1.0 + cos_w0) / 2.0;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        _ => return Err("num_butter_biquad: kind low|high".into()),
    };
    let mut out = HashMap::new();
    out.insert("b0".into(), float_out(b0));
    out.insert("b1".into(), float_out(b1));
    out.insert("b2".into(), float_out(b2));
    out.insert("a0".into(), float_out(a0));
    out.insert("a1".into(), float_out(a1));
    out.insert("a2".into(), float_out(a2));
    Ok(Value::from_object(out))
}

/// num_polyphase_resample(x, up, down) — upsample-by-up, FIR lowpass, downsample-by-down.
fn num_polyphase_resample(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_polyphase_resample")?;
    let up = num_at(args, 1, "num_polyphase_resample")? as usize;
    let down = num_at(args, 2, "num_polyphase_resample")? as usize;
    if x.is_empty() || up == 0 || down == 0 {
        return Err("num_polyphase_resample: bad args".into());
    }
    // Insert up-1 zeros between samples.
    let mut upsampled = vec![0.0; x.len() * up];
    for (i, v) in x.iter().enumerate() {
        upsampled[i * up] = *v;
    }
    // Lowpass cutoff at min(1/up, 1/down) in firwin Nyquist units relative to upsampled rate.
    let cutoff = (1.0 / up as f64).min(1.0 / down as f64) * 0.9;
    let taps = (8 * up.max(down) + 1) | 1; // odd length
    let h = num_firwin(&[float_out(taps as f64), float_out(cutoff.max(0.01).min(0.99))], env)?;
    let filtered = num_fir(&[vector_out(&upsampled), h], env)?;
    let y = vector_at(&[filtered], 0, "num_polyphase_resample")?;
    // Compensate upsample gain
    let gain = up as f64;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < y.len() {
        out.push(y[i] * gain);
        i += down;
    }
    Ok(vector_out(&out))
}

/// num_polyphase_decompose(h, n) — split FIR h into n polyphase branches (commutator).
fn num_polyphase_decompose(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let h = vector_at(args, 0, "num_polyphase_decompose")?;
    let n = num_at(args, 1, "num_polyphase_decompose")? as usize;
    if h.is_empty() || n == 0 {
        return Err("num_polyphase_decompose: bad args".into());
    }
    let mut branches = vec![Vec::new(); n];
    for (i, v) in h.iter().enumerate() {
        branches[i % n].push(*v);
    }
    Ok(Value::from_array(
        branches.iter().map(|b| vector_out(b)).collect(),
    ))
}

/// num_polyphase_analyze(x, h, n) — FIR bank: filter+downsample by n per branch.
fn num_polyphase_analyze(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_polyphase_analyze")?;
    let h = vector_at(args, 1, "num_polyphase_analyze")?;
    let n = num_at(args, 2, "num_polyphase_analyze")? as usize;
    if x.is_empty() || h.is_empty() || n == 0 {
        return Err("num_polyphase_analyze: bad args".into());
    }
    let branches_v = num_polyphase_decompose(&[vector_out(&h), float_out(n as f64)], env)?;
    let Value::Array(branches) = branches_v else {
        return Err("num_polyphase_analyze: internal".into());
    };
    let mut bands = Vec::new();
    for (b_idx, br) in branches.iter().enumerate() {
        let coeffs = vector_at(std::slice::from_ref(br), 0, "num_polyphase_analyze")?;
        // Phase-shifted input: x[b], x[b+n], ...
        let mut phase: Vec<f64> = Vec::new();
        let mut i = b_idx;
        while i < x.len() {
            phase.push(x[i]);
            i += n;
        }
        let filtered = num_fir(&[vector_out(&phase), vector_out(&coeffs)], env)?;
        bands.push(filtered);
    }
    Ok(Value::from_array(bands))
}

/// num_polyphase_synthesize(bands, h, n) — inverse FIR bank (upsample+filter+sum).
fn num_polyphase_synthesize(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let bands = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("num_polyphase_synthesize(bands, h, n)".into()),
    };
    let h = vector_at(args, 1, "num_polyphase_synthesize")?;
    let n = num_at(args, 2, "num_polyphase_synthesize")? as usize;
    if bands.is_empty() || h.is_empty() || n == 0 || bands.len() != n {
        return Err("num_polyphase_synthesize: bad args".into());
    }
    let branches_v = num_polyphase_decompose(&[vector_out(&h), float_out(n as f64)], env)?;
    let Value::Array(branches) = branches_v else {
        return Err("num_polyphase_synthesize: internal".into());
    };
    let mut max_len = 0usize;
    let mut ups: Vec<Vec<f64>> = Vec::new();
    for (b_idx, (band_v, br)) in bands.iter().zip(branches.iter()).enumerate() {
        let band = vector_at(std::slice::from_ref(band_v), 0, "num_polyphase_synthesize")?;
        let coeffs = vector_at(std::slice::from_ref(br), 0, "num_polyphase_synthesize")?;
        let filtered = num_fir(&[vector_out(&band), vector_out(&coeffs)], env)?;
        let y = vector_at(&[filtered], 0, "num_polyphase_synthesize")?;
        let mut up = vec![0.0; y.len() * n];
        for (i, v) in y.iter().enumerate() {
            up[i * n + b_idx] = *v;
        }
        max_len = max_len.max(up.len());
        ups.push(up);
    }
    let mut out = vec![0.0; max_len];
    for up in ups {
        for (i, v) in up.iter().enumerate() {
            out[i] += *v;
        }
    }
    Ok(vector_out(&out))
}

/// num_dwt_haar(x) -> { a, d } — one-level Haar discrete wavelet transform.
fn num_dwt_haar(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_dwt_haar")?;
    if x.len() < 2 {
        return Err("num_dwt_haar: length >= 2".into());
    }
    let n = x.len() / 2;
    let s2 = 2.0_f64.sqrt();
    let mut a = Vec::with_capacity(n);
    let mut d = Vec::with_capacity(n);
    for i in 0..n {
        a.push((x[2 * i] + x[2 * i + 1]) / s2);
        d.push((x[2 * i] - x[2 * i + 1]) / s2);
    }
    let mut out = HashMap::new();
    out.insert("a".into(), vector_out(&a));
    out.insert("d".into(), vector_out(&d));
    out.insert("kind".into(), Value::String("haar".into()));
    Ok(Value::from_object(out))
}

/// num_idwt_haar(a, d) — inverse Haar DWT.
fn num_idwt_haar(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "num_idwt_haar")?;
    let d = vector_at(args, 1, "num_idwt_haar")?;
    if a.len() != d.len() || a.is_empty() {
        return Err("num_idwt_haar: a/d length".into());
    }
    let s2 = 2.0_f64.sqrt();
    let mut x = Vec::with_capacity(a.len() * 2);
    for i in 0..a.len() {
        x.push((a[i] + d[i]) / s2);
        x.push((a[i] - d[i]) / s2);
    }
    Ok(vector_out(&x))
}

fn haar_forward_once(x: &[f64]) -> Result<(Vec<f64>, Vec<f64>), String> {
    if x.len() < 2 {
        return Err("haar: length >= 2".into());
    }
    let n = x.len() / 2;
    let s2 = 2.0_f64.sqrt();
    let mut a = Vec::with_capacity(n);
    let mut d = Vec::with_capacity(n);
    for i in 0..n {
        a.push((x[2 * i] + x[2 * i + 1]) / s2);
        d.push((x[2 * i] - x[2 * i + 1]) / s2);
    }
    Ok((a, d))
}

fn haar_inverse_once(a: &[f64], d: &[f64]) -> Result<Vec<f64>, String> {
    if a.len() != d.len() || a.is_empty() {
        return Err("haar: a/d length".into());
    }
    let s2 = 2.0_f64.sqrt();
    let mut x = Vec::with_capacity(a.len() * 2);
    for i in 0..a.len() {
        x.push((a[i] + d[i]) / s2);
        x.push((a[i] - d[i]) / s2);
    }
    Ok(x)
}

/// num_dwt_haar_levels(x, levels) -> { a, details: [d1..], kind, levels }
fn num_dwt_haar_levels(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut x = vector_at(args, 0, "num_dwt_haar_levels")?;
    let levels = num_at(args, 1, "num_dwt_haar_levels")?.max(1.0) as usize;
    let mut details = Vec::new();
    for _ in 0..levels {
        if x.len() < 2 {
            return Err("num_dwt_haar_levels: signal too short".into());
        }
        let (a, d) = haar_forward_once(&x)?;
        details.push(vector_out(&d));
        x = a;
    }
    let mut out = HashMap::new();
    out.insert("a".into(), vector_out(&x));
    out.insert("details".into(), Value::from_array(details));
    out.insert("kind".into(), Value::String("haar_levels".into()));
    out.insert("levels".into(), Value::Number(levels as i64));
    Ok(Value::from_object(out))
}

/// num_idwt_haar_levels(a, details) — inverse multi-level Haar.
fn num_idwt_haar_levels(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut a = vector_at(args, 0, "num_idwt_haar_levels")?;
    let details = match args.get(1) {
        Some(Value::Array(d)) => d.clone(),
        _ => return Err("num_idwt_haar_levels(a, details)".into()),
    };
    for d_v in details.iter().rev() {
        let d = vector_at(std::slice::from_ref(d_v), 0, "num_idwt_haar_levels")?;
        a = haar_inverse_once(&a, &d)?;
    }
    Ok(vector_out(&a))
}

/// num_wpt_haar(x, levels) — Haar wavelet packet tree (full binary).
fn num_wpt_haar(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_wpt_haar")?;
    let levels = num_at(args, 1, "num_wpt_haar")?.max(1.0) as usize;
    let mut leaves = vec![x];
    for _ in 0..levels {
        let mut nxt = Vec::new();
        for node in leaves {
            let (a, d) = haar_forward_once(&node)?;
            nxt.push(a);
            nxt.push(d);
        }
        leaves = nxt;
    }
    let mut out = HashMap::new();
    out.insert(
        "packets".into(), Value::from_array(leaves.iter().map(|v| vector_out(v)).collect()),
    );
    out.insert("kind".into(), Value::String("wpt_haar".into()));
    out.insert("levels".into(), Value::Number(levels as i64));
    Ok(Value::from_object(out))
}

/// num_iwpt_haar(packets, levels) — inverse Haar wavelet packet.
fn num_iwpt_haar(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let packets = match args.first() {
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| vector_at(std::slice::from_ref(v), 0, "num_iwpt_haar"))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("num_iwpt_haar(packets, levels)".into()),
    };
    let levels = num_at(args, 1, "num_iwpt_haar")?.max(1.0) as usize;
    let expect = 1usize << levels;
    if packets.len() != expect {
        return Err(format!("num_iwpt_haar: expect {expect} packets"));
    }
    let mut nodes = packets;
    for _ in 0..levels {
        let mut nxt = Vec::new();
        let mut i = 0;
        while i + 1 < nodes.len() {
            nxt.push(haar_inverse_once(&nodes[i], &nodes[i + 1])?);
            i += 2;
        }
        nodes = nxt;
    }
    Ok(vector_out(&nodes[0]))
}

/// Dual-tree complex wavelet (Haar trees): tree A on x, tree B on circular shift.
/// num_dtcwt(x, levels) -> { aRe, aIm, detailsRe, detailsIm, kind, levels }
fn num_dtcwt(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "num_dtcwt")?;
    let levels = num_at(args, 1, "num_dtcwt")?.max(1.0) as usize;
    if x.len() < 2 {
        return Err("num_dtcwt: length >= 2".into());
    }
    let mut x_shift = x.clone();
    x_shift.rotate_left(1);
    let ta = num_dwt_haar_levels(&[vector_out(&x), float_out(levels as f64)], env)?;
    let tb = num_dwt_haar_levels(&[vector_out(&x_shift), float_out(levels as f64)], env)?;
    let Value::Object(ma) = ta else {
        return Err("num_dtcwt: tree A".into());
    };
    let Value::Object(mb) = tb else {
        return Err("num_dtcwt: tree B".into());
    };
    let mut out = HashMap::new();
    out.insert("aRe".into(), ma.get("a").cloned().ok_or("num_dtcwt: a")?);
    out.insert("aIm".into(), mb.get("a").cloned().ok_or("num_dtcwt: aIm")?);
    out.insert(
        "detailsRe".into(),
        ma.get("details").cloned().ok_or("num_dtcwt: details")?,
    );
    out.insert(
        "detailsIm".into(),
        mb.get("details").cloned().ok_or("num_dtcwt: detailsIm")?,
    );
    out.insert("kind".into(), Value::String("dtcwt".into()));
    out.insert("levels".into(), Value::Number(levels as i64));
    Ok(Value::from_object(out))
}

/// num_idtcwt(aRe, aIm, detailsRe, detailsIm) — inverse dual-tree (average of two iDWTs).
fn num_idtcwt(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a_re = args.get(0).ok_or("num_idtcwt(aRe,aIm,detailsRe,detailsIm)")?.clone();
    let a_im = args.get(1).ok_or("num_idtcwt: aIm")?.clone();
    let d_re = args.get(2).ok_or("num_idtcwt: detailsRe")?.clone();
    let d_im = args.get(3).ok_or("num_idtcwt: detailsIm")?.clone();
    let xa = num_idwt_haar_levels(&[a_re, d_re], env)?;
    let xb = num_idwt_haar_levels(&[a_im, d_im], env)?;
    let va = vector_at(&[xa], 0, "num_idtcwt")?;
    let mut vb = vector_at(&[xb], 0, "num_idtcwt")?;
    if va.len() != vb.len() {
        return Err("num_idtcwt: length mismatch".into());
    }
    // Undo circular shift on tree B.
    if !vb.is_empty() {
        vb.rotate_right(1);
    }
    let out: Vec<f64> = va
        .iter()
        .zip(vb.iter())
        .map(|(a, b)| 0.5 * (a + b))
        .collect();
    Ok(vector_out(&out))
}

/// num_fir(signal, coeffs) — FIR filter (convolution, 'same' length as signal).
fn num_fir(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let signal = vector_at(args, 0, "num_fir")?;
    let coeffs = vector_at(args, 1, "num_fir")?;
    if signal.is_empty() || coeffs.is_empty() {
        return Err("num_fir: empty".into());
    }
    let m = coeffs.len();
    let mut out = vec![0.0; signal.len()];
    for n in 0..signal.len() {
        let mut s = 0.0;
        for k in 0..m {
            if n >= k {
                s += coeffs[k] * signal[n - k];
            }
        }
        out[n] = s;
    }
    Ok(vector_out(&out))
}

/// num_moving_average(signal, window) — boxcar FIR.
fn num_moving_average(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let signal = vector_at(args, 0, "num_moving_average")?;
    let w = num_at(args, 1, "num_moving_average")? as usize;
    if w == 0 {
        return Err("num_moving_average: window > 0".into());
    }
    let coeffs = vec![1.0 / w as f64; w];
    num_fir(&[vector_out(&signal), vector_out(&coeffs)], _env)
}

/// num_iir(signal, b, a) — Direct Form I; a[0] is a0 (normalized if needed).
fn num_iir(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let signal = vector_at(args, 0, "num_iir")?;
    let b = vector_at(args, 1, "num_iir")?;
    let a = vector_at(args, 2, "num_iir")?;
    if signal.is_empty() || b.is_empty() || a.is_empty() {
        return Err("num_iir: empty".into());
    }
    if a[0].abs() < 1e-15 {
        return Err("num_iir: a0 near zero".into());
    }
    let a0 = a[0];
    let mut y = vec![0.0; signal.len()];
    for n in 0..signal.len() {
        let mut s = 0.0;
        for k in 0..b.len() {
            if n >= k {
                s += b[k] * signal[n - k];
            }
        }
        for k in 1..a.len() {
            if n >= k {
                s -= a[k] * y[n - k];
            }
        }
        y[n] = s / a0;
    }
    Ok(vector_out(&y))
}

/// num_biquad(signal, b0,b1,b2,a0,a1,a2) — second-order IIR convenience.
fn num_biquad(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let signal = args.first().ok_or("num_biquad")?.clone();
    let b0 = num_at(args, 1, "num_biquad")?;
    let b1 = num_at(args, 2, "num_biquad")?;
    let b2 = num_at(args, 3, "num_biquad")?;
    let a0 = num_at(args, 4, "num_biquad")?;
    let a1 = num_at(args, 5, "num_biquad")?;
    let a2 = num_at(args, 6, "num_biquad")?;
    num_iir(
        &[
            signal,
            vector_out(&[b0, b1, b2]),
            vector_out(&[a0, a1, a2]),
        ],
        env,
    )
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_fft", "num_fft"], num_fft);
    bind(&["science_num_ifft", "num_ifft"], num_ifft);
    bind(&["science_num_fft_c", "num_fft_c"], num_fft_c);
    bind(&["science_num_rfft", "num_rfft"], num_rfft);
    bind(&["science_num_irfft", "num_irfft"], num_irfft);
    bind(&["science_num_fft_pad", "num_fft_pad"], num_fft_pad);
    bind(&["science_num_fftfreq", "num_fftfreq"], num_fftfreq);
    bind(&["science_num_resample", "num_resample"], num_resample);
    bind(&["science_num_hilbert", "num_hilbert"], num_hilbert);
    bind(&["science_num_conv1d", "num_conv1d"], num_conv1d);
    bind(&["science_mat_svd2", "mat_svd2"], mat_svd2);
    bind(&["science_num_window_hann", "num_window_hann"], num_window_hann);
    bind(&["science_num_window_hamming", "num_window_hamming"], num_window_hamming);
    bind(&["science_num_stft", "num_stft"], num_stft);
    bind(&["science_num_fft2d", "num_fft2d"], num_fft2d);
    bind(&["science_num_fftn", "num_fftn"], num_fftn);
    bind(&["science_num_firwin", "num_firwin"], num_firwin);
    bind(
        &["science_num_butter_biquad", "num_butter_biquad"],
        num_butter_biquad,
    );
    bind(
        &["science_num_polyphase_resample", "num_polyphase_resample"],
        num_polyphase_resample,
    );
    bind(
        &["science_num_polyphase_decompose", "num_polyphase_decompose"],
        num_polyphase_decompose,
    );
    bind(
        &["science_num_polyphase_analyze", "num_polyphase_analyze"],
        num_polyphase_analyze,
    );
    bind(
        &["science_num_polyphase_synthesize", "num_polyphase_synthesize"],
        num_polyphase_synthesize,
    );
    bind(&["science_num_dwt_haar", "num_dwt_haar"], num_dwt_haar);
    bind(&["science_num_idwt_haar", "num_idwt_haar"], num_idwt_haar);
    bind(
        &["science_num_dwt_haar_levels", "num_dwt_haar_levels"],
        num_dwt_haar_levels,
    );
    bind(
        &["science_num_idwt_haar_levels", "num_idwt_haar_levels"],
        num_idwt_haar_levels,
    );
    bind(&["science_num_wpt_haar", "num_wpt_haar"], num_wpt_haar);
    bind(&["science_num_iwpt_haar", "num_iwpt_haar"], num_iwpt_haar);
    bind(&["science_num_dtcwt", "num_dtcwt"], num_dtcwt);
    bind(&["science_num_idtcwt", "num_idtcwt"], num_idtcwt);
    bind(&["science_num_fir", "num_fir"], num_fir);
    bind(&["science_num_moving_average", "num_moving_average"], num_moving_average);
    bind(&["science_num_iir", "num_iir"], num_iir);
    bind(&["science_num_biquad", "num_biquad"], num_biquad);
}
