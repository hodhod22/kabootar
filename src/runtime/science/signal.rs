//! FFT + SVD subset (SC1c/d).

use super::helpers::{matrix_at, matrix_out, require_square, vector_at, vector_out};
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
    Ok(Value::Object(out))
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

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_fft", "num_fft"], num_fft);
    bind(&["science_num_ifft", "num_ifft"], num_ifft);
    bind(&["science_num_conv1d", "num_conv1d"], num_conv1d);
    bind(&["science_mat_svd2", "mat_svd2"], mat_svd2);
}
