/**
 * Inbyggda moduler — håll i synk med src/modules/mod.rs
 */
export const BUILTIN_MODULES: Record<string, string> = {
  math: `fn add(a, b) {
    return a + b
}
fn mul(a, b) {
    return a * b
}
`,
  http: `fn ok(body) {
    return http_response(200, body)
}
fn not_found() {
    return http_response(404, "Not Found")
}
`,
  crypto: `fn sha256(data) {
    return crypto_sha3_256(data)
}
fn secure(data) {
    return crypto_secure(data)
}
`,
  // Stub för IDE — natives registreras vid import (se docs/SCIENCE.md)
  science: `fn cplx(re, im) { return null }
fn c_add(a, b) { return null }
fn c_sub(a, b) { return null }
fn c_mul(a, b) { return null }
fn c_div(a, b) { return null }
fn c_conj(z) { return null }
fn c_abs(z) { return null }
fn c_arg(z) { return null }
fn c_exp(z) { return null }
fn c_sqrt(z) { return null }
fn c_polar(r, theta) { return null }
fn sqrt(x) { return null }
fn pow(x, y) { return null }
fn fact(n) { return null }
fn gcd(a, b) { return null }
fn lcm(a, b) { return null }
fn sin(x) { return null }
fn cos(x) { return null }
fn tan(x) { return null }
fn ln(x) { return null }
fn log10(x) { return null }
fn deg2rad(d) { return null }
fn rad2deg(r) { return null }
fn quadratic(a, b, c) { return null }
fn kinetic_energy(m, v) { return null }
fn potential_energy(m, g, h) { return null }
fn force(m, a) { return null }
fn ohms_v(i, r) { return null }
fn ohms_p(v, i) { return null }
fn wavelength(f) { return null }
fn photon_energy(f) { return null }
fn relativity_e(m) { return null }
fn ph(h_plus) { return null }
fn h_plus(ph_val) { return null }
fn molarity(moles, volume_l) { return null }
fn ideal_gas_p(n, temp_k, volume_l) { return null }
fn dilution(c1, v1, c2) { return null }
fn compound(principal, rate, years) { return null }
fn present_value(fv, rate, years) { return null }
fn break_even(fixed, price, variable) { return null }
fn roi(gain, cost) { return null }
fn margin(revenue, cost) { return null }
fn bit_and(a, b) { return null }
fn bit_or(a, b) { return null }
fn bit_xor(a, b) { return null }
fn bit_not(a) { return null }
fn shl(a, n) { return null }
fn shr(a, n) { return null }
fn hex(s) { return null }
fn bin(s) { return null }
fn hamming_weight(n) { return null }
fn stat_mean(data) { return null }
fn stat_std(data) { return null }
fn stat_linreg(x, y) { return null }
fn mat(rows, cols) { return null }
fn mat_mul(a, b) { return null }
fn mat_det(m) { return null }
fn mat_inv(m) { return null }
fn num_trapz(xs, ys) { return null }
fn num_solve(a, b) { return null }
fn num_interp_linear(xs, ys, x) { return null }
`,
  docai: `fn doc_ask(query) { return null }
fn doc_search(query) { return null }
fn doc_sources(query) { return null }
fn doc_topics() { return null }
`,
};

export function moduleSource(name: string): string | undefined {
  return BUILTIN_MODULES[name];
}
