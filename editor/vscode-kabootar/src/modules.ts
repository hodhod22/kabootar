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
fn el_copper_r(cross_mm2, length_m) { return null }
fn el_aluminum_r(cross_mm2, length_m) { return null }
fn el_voltage_drop(i_a, r_ohm) { return null }
fn el_voltage_drop_pct(drop_v, nominal_v) { return null }
fn el_drop_ok(drop_v, nominal_v, max_pct) { return null }
fn el_single_phase_i(p_w, u_v, cos_phi) { return null }
fn el_three_phase_i(p_w, u_ll_v, cos_phi) { return null }
fn el_single_phase_p(u_v, i_a, cos_phi) { return null }
fn el_three_phase_p(u_ll_v, i_a, cos_phi) { return null }
fn el_apparent_va(u_v, i_a) { return null }
fn el_three_phase_s(u_ll_v, i_a) { return null }
fn el_reactive_from_ps(p_w, s_va) { return null }
fn el_reactive_ui(u_v, i_a, sin_phi) { return null }
fn el_power_factor(p_w, s_va) { return null }
fn el_sin_phi(cos_phi) { return null }
fn el_kw_to_kva(kw, cos_phi) { return null }
fn el_kva_to_kw(kva, cos_phi) { return null }
fn el_power_triangle_s(p_w, q_var) { return null }
fn el_fuse_size(load_a, margin) { return null }
fn el_motor_flc(kw, u_v, eff, cos_phi, phases) { return null }
fn el_ampacity_cu(cross_mm2) { return null }
fn el_cable_min_cu(load_a) { return null }
fn el_phase_imbalance(i1, i2, i3) { return null }
fn el_short_circuit_i(u_v, z_ohm) { return null }
fn el_loop_z(r_line, r_pe) { return null }
fn el_cable_derating(ambient_c, ref_c, base_a) { return null }
fn el_1ph_drop(i_a, length_m, cross_mm2) { return null }
fn el_3ph_drop(i_a, length_m, cross_mm2) { return null }
fn plc_and(a, b) { return null }
fn plc_or(a, b) { return null }
fn plc_not(a) { return null }
fn plc_xor(a, b) { return null }
fn plc_rising_edge(curr, prev) { return null }
fn plc_falling_edge(curr, prev) { return null }
fn plc_ton_done(elapsed_ms, preset_ms) { return null }
fn plc_tof_active(elapsed_ms, preset_ms, input_on) { return null }
fn plc_ctu_done(count, preset) { return null }
fn plc_ctd_done(count, preset) { return null }
fn plc_scale_raw(raw, raw_min, raw_max, eng_min, eng_max) { return null }
fn plc_eng_to_raw(eng, eng_min, eng_max, raw_min, raw_max) { return null }
fn plc_4_20_to_pct(ma) { return null }
fn plc_pct_to_4_20(pct) { return null }
fn plc_0_10v_to_pct(v) { return null }
fn plc_pct_to_0_10v(pct) { return null }
fn plc_pid_error(sp, pv) { return null }
fn plc_pid_p(error, kp) { return null }
fn plc_pid_i(error_sum, ki, dt_s) { return null }
fn plc_pid_d(error, prev_error, kd, dt_s) { return null }
fn plc_scan_hz(scan_ms) { return null }
fn plc_debounce_scans(debounce_ms, scan_ms) { return null }
fn plc_modbus_holding(unit, offset) { return null }
fn plc_modbus_input(unit, offset) { return null }
fn plc_modbus_coil(unit, offset) { return null }
fn plc_latch(set, reset, prev) { return null }
fn plc_seal_in(input, memory) { return null }
fn plc_limit_alarm(pv, hi, lo) { return null }
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
