# Kabootar — `import "science"`

Vetenskaplig och teknisk beräkningsmodul. Laddas **endast vid import**.

```kabootar
import "science";
```

## Innehåll

1. [Kom igång](#kom-igång)
2. [Konventioner](#konventioner)
3. [Komplexa tal](#komplexa-tal)
4. [Matematik](#matematik)
5. [Fysik](#fysik)
6. [Kemi](#kemi)
7. [Ekonomi](#ekonomi)
8. [Digitalt / binärt](#digitalt--binärt)
9. [Statistik](#statistik)
10. [Matriser](#matriser)
11. [Numerisk analys](#numerisk-analys)
12. [Ndarray (SC0)](#ndarray-sc0)
13. [ML / AI (SC2)](#ml--ai-sc2)
14. [Felsökning](#felsökning)

**Roadmap:** [ROADMAP.md](ROADMAP.md) **Våg SC** — ta över Pythons roll för forskning & AI. Gap vs NumPy/SciPy/sklearn/PyTorch + **Kab-first / SC5 self-host** + **SC6 ✅** + **SC7 ✅ subset** (`io`, `parallel`, `visualize`, `nd_gpu`).
---

## Kom igång

```kabootar
import "science";

// Komplexa tal
c_abs(cplx(3, 4));              // 5

// Fysik — Ohms lag
ohms_v(10, 2);                  // 20 V

// Statistik
stat_mean([1, 2, 3, 4, 5]);     // 3
```

Funktionerna finns **inte** i global miljö förrän du importerar modulen.

---

## Konventioner

| Typ | Format | Exempel |
|-----|--------|---------|
| Komplext tal | `[re, im]` | `cplx(3, 4)` → `[3.0, 4.0]` |
| Matris | `[[a, b], [c, d]]` | rader × kolumner |
| Vektor | `[1, 2, 3]` | numerisk array |
| Flyttal | `Float` | `3.14` |
| Heltal | `Number` | `42` |
| Sträng (hex/bin) | `"FF"`, `"1010"` | `hex("FF")` → `255` |

---

## Komplexa tal

Komplexa tal returneras alltid som `[re, im]`.

### `cplx(re, im)`

Skapar ett komplext tal.

```kabootar
import "science";
cplx(3, 4);        // [3.0, 4.0]
cplx(0, -1);       // [0.0, -1.0]
```

### `c_add(a, b)` / `c_sub(a, b)` / `c_mul(a, b)` / `c_div(a, b)`

Aritmetik. Båda argumenten ska vara `[re, im]`.

```kabootar
import "science";
let a = cplx(1, 2);
let b = cplx(3, 1);
c_add(a, b);       // [4.0, 3.0]
c_mul(a, b);       // [1.0, 7.0]
```

**Fel:** `c_div` med nämnare `[0, 0]` → `"complex division by zero"`.

### `c_conj(z)`

Konjugat: `(a+bi)* → (a-bi)`.

```kabootar
import "science";
c_conj(cplx(2, 3));   // [2.0, -3.0]
```

### `c_abs(z)`

Belopp \|z\| = √(re² + im²).

```kabootar
import "science";
c_abs(cplx(3, 4));    // 5
```

### `c_arg(z)`

Argument (radianer).

```kabootar
import "science";
c_arg(cplx(1, 1));    // ~0.785 (π/4)
rad2deg(c_arg(cplx(1, 1)));  // ~45
```

### `c_exp(z)` / `c_sqrt(z)`

Exponential och kvadratrot i komplexa tal.

```kabootar
import "science";
c_exp(cplx(0, 3.14159));   // ~[-1.0, 0.0] (e^iπ)
c_sqrt(cplx(-4, 0));       // [0.0, 2.0]
```

### `c_polar(r, theta)`

Polär → rektangulär. `theta` i radianer.

```kabootar
import "science";
c_polar(5, deg2rad(90));   // [0.0, 5.0]
```

---

## Matematik

### `sqrt(x)`

Kvadratrot (reella tal ≥ 0).

```kabootar
import "science";
sqrt(16);          // 4
// sqrt(-1);       // fel — använd c_sqrt(cplx(-1, 0))
```

### `pow(x, y)`

Potens x^y.

```kabootar
import "science";
pow(2, 10);        // 1024
pow(9, 0.5);       // 3
```

### `fact(n)`

Fakultet. `n` heltal 0–20.

```kabootar
import "science";
fact(5);           // 120
fact(0);           // 1
```

### `gcd(a, b)` / `lcm(a, b)`

Största gemensamma delare / minsta gemensamma multipel.

```kabootar
import "science";
gcd(48, 18);       // 6
lcm(4, 6);         // 12
```

### `sin(x)` / `cos(x)` / `tan(x)`

Trigonometri. `x` i radianer.

```kabootar
import "science";
sin(deg2rad(30));  // 0.5
cos(0);            // 1
```

### `ln(x)` / `log10(x)`

Naturlig respektive briggsk logaritm.

```kabootar
import "science";
ln(2.718281828);   // ~1
log10(1000);       // 3
```

### `deg2rad(d)` / `rad2deg(r)`

Vinkelkonvertering.

```kabootar
import "science";
deg2rad(180);      // ~3.14159
rad2deg(3.14159);  // ~180
```

### `quadratic(a, b, c)`

Löser ax² + bx + c = 0. Returnerar array med två rötter (reella `Float` eller komplexa `[re,im]`).

```kabootar
import "science";
quadratic(1, -5, 6);    // [3.0, 2.0]
quadratic(1, 0, 1);     // komplexa: [0, 1], [0, -1]
quadratic(1, -2, 5);    // [1+2i, 1-2i]
```

---

## Fysik

### `kinetic_energy(m, v)`

Kinetisk energi ½mv² (J). `m` kg, `v` m/s.

```kabootar
import "science";
kinetic_energy(2, 3);   // 9
```

### `potential_energy(m, g, h)`

Potentiell energi mgh (J).

```kabootar
import "science";
potential_energy(10, 9.81, 2);   // ~196.2
```

### `force(m, a)`

Kraft F = ma (N).

```kabootar
import "science";
force(5, 2);       // 10
```

### `ohms_v(i, r)` / `ohms_p(v, i)`

Ohms lag: V = IR, P = VI.

```kabootar
import "science";
ohms_v(2, 10);     // 20 (V)
ohms_p(230, 5);    // 1150 (W)
```

### `wavelength(f)` / `photon_energy(f)`

`f` i Hz. λ = c/f, E = hf.

```kabootar
import "science";
wavelength(100000000);     // 2.99792458 (m) vid 100 MHz
photon_energy(500000000000000);  // E för 500 THz
```

### `relativity_e(m)`

Relativistisk energi E = mc² (J). `m` kg.

```kabootar
import "science";
relativity_e(0.001);   // ~9e13 J
```

---

## Kemi

### `ph(h_plus)` / `h_plus(ph_val)`

pH = −log₁₀[H⁺]. `h_plus` i mol/L.

```kabootar
import "science";
ph(0.001);         // 3
ph(1e-7);          // 7
h_plus(7);         // 1e-7
```

### `molarity(moles, volume_l)`

Koncentration mol/L.

```kabootar
import "science";
molarity(2, 0.5);  // 4
```

### `ideal_gas_p(n, temp_k, volume_l)`

PV = nRT (Pa). `volume_l` i liter (omvandlas internt).

```kabootar
import "science";
ideal_gas_p(1, 298, 24.5);   // tryck i Pa
```

### `dilution(c1, v1, c2)`

C₁V₁ = C₂V₂ → beräknar V₂.

```kabootar
import "science";
dilution(1, 100, 0.1);   // 1000 (volymenhet samma som v1)
```

---

## Ekonomi

### `compound(P, r, years)`

Sammansatt ränta: P(1+r)^n.

```kabootar
import "science";
compound(1000, 0.05, 2);   // 1102.5
```

### `present_value(fv, r, years)`

Nuvarande värde.

```kabootar
import "science";
present_value(1102.5, 0.05, 2);   // ~1000
```

### `break_even(fixed, price, variable)`

Break-even-volym: fixed / (price − variable).

```kabootar
import "science";
break_even(10000, 50, 30);   // 500 enheter
```

### `roi(gain, cost)`

Avkastning i procent.

```kabootar
import "science";
roi(15000, 10000);   // 50 (%)
```

### `margin(revenue, cost)`

Bruttomarginal i procent.

```kabootar
import "science";
margin(1000, 600);   // 40 (%)
```

---

## Digitalt / binärt

### `bit_and(a, b)` / `bit_or(a, b)` / `bit_xor(a, b)` / `bit_not(a)`

Bitvisa operationer på heltal.

```kabootar
import "science";
bit_and(12, 10);   // 8  (1100 & 1010)
bit_or(12, 10);    // 14
bit_xor(12, 10);   // 6
bit_not(0);        // -1
```

### `shl(a, n)` / `shr(a, n)`

Bitshift. `n` kläms till 0–63.

```kabootar
import "science";
shl(1, 4);         // 16
shr(16, 2);        // 4
```

### `hex(s)` / `bin(s)`

Parsa sträng till heltal.

```kabootar
import "science";
hex("FF");         // 255
hex("2A");         // 42
bin("1010");       // 10
```

### `hamming_weight(n)`

Antal ettor i binär representation.

```kabootar
import "science";
hamming_weight(7);   // 3  (111)
hamming_weight(255); // 8
```

---

## Statistik

Data som numeriska arrayer `[x1, x2, ...]`.

| Funktion | Beskrivning |
|----------|-------------|
| `stat_mean(data)` | Medelvärde |
| `stat_median(data)` | Median |
| `stat_std(data)` | Standardavvikelse (population) |
| `stat_sample_std(data)` | Standardavvikelse (urval) |
| `stat_var(data)` / `stat_sample_var(data)` | Varians |
| `stat_min(data)` / `stat_max(data)` | Min/max |
| `stat_sum(data)` / `stat_count(data)` | Summa / antal |
| `stat_percentile(data, p)` | Percentil (0–100) |
| `stat_covariance(x, y)` | Kovarians |
| `stat_correlation(x, y)` | Korrelationskoefficient |
| `stat_linreg(x, y)` | Linjär regression → `[slope, intercept, r²]` |

```kabootar
import "science";
stat_mean([2, 4, 4, 5, 7, 9]);           // 5.166...
stat_std([2, 4, 4, 5, 7, 9]);
let fit = stat_linreg([1, 2, 3], [2, 4, 6]); // [2, 0, 1]
```

---

## Matriser

Matriser som array av rader: `[[1, 2], [3, 4]]`.

| Funktion | Beskrivning |
|----------|-------------|
| `mat(rows, cols, fill)` | Skapa matris (standard fill=0) |
| `mat_identity(n)` | Identitetsmatris n×n |
| `mat_rows(m)` / `mat_cols(m)` | Dimensioner |
| `mat_transpose(m)` | Transponering |
| `mat_add(a, b)` / `mat_sub(a, b)` | Addition / subtraktion |
| `mat_scale(m, s)` | Skalning |
| `mat_mul(a, b)` | Matrismultiplikation |
| `mat_vec_mul(m, v)` | Matris × vektor |
| `mat_dot(a, b)` | Skalärprodukt (vektorer) |
| `mat_norm(v)` | Euklidisk norm |
| `mat_det(m)` | Determinant |
| `mat_inv(m)` | Invers (Gauss-Jordan) |
| `mat_eigen2(m)` | Egenvärden (2×2) |

```kabootar
import "science";
let a = [[1, 2], [3, 4]];
mat_det(a);                              // -2
mat_mul(a, [[1, 0], [0, 1]]);
mat_inv(a);
```

---

## Numerisk analys

| Funktion | Beskrivning |
|----------|-------------|
| `num_trapz(y, dx)` / `num_trapz(xs, ys)` | Trapesintegration |
| `num_simpson(xs, ys)` | Simpsons regel (jämnt fördelade xs) |
| `num_interp_linear(xs, ys, x)` | Linjär interpolation |
| `num_lerp(x0, y0, x1, y1, x)` | Linjär interpolering mellan två punkter |
| `num_poly_eval(coeffs, x)` | Polynom \(c_0 + c_1 x + c_2 x^2 + \ldots\) |
| `num_newton_step(x, f(x), f'(x))` | Ett Newton-Raphson-steg |
| `num_bisect_mid(a, b)` | Bisektionsmittpunkt |
| `num_diff_forward(f(x), f(x+h), h)` | Framåtdifferens |
| `num_diff_central(f(x-h), f(x+h), h)` | Central differens |
| `num_solve(A, b)` | Lös \(Ax = b\) (Gauss elimination) |

```kabootar
import "science";
num_trapz([1, 1], 1);                // 1 — konstant integrand
num_solve([[2, 1], [1, 3]], [4, 5]);   // lösning som vektor

// Newton: hitta rot till x² - 2 = 0
let x = 1.5;
let fx = x * x - 2;
let dfx = 2 * x;
x = num_newton_step(x, fx, dfx);
```

---

## Ndarray (SC0)

Kontiguösa arrayer med `shape` + flat `data` (NumPy-klass subset). Kräver `import "science"`. Valfritt: `import "science/nd"`.

```kabootar
import "science";
let a = nd_from([[1.0, 2.0], [3.0, 4.0]]);
nd_shape(a);                 // [2, 2]
let eye = nd_from([[1.0, 0.0], [0.0, 1.0]]);
let b = nd_matmul(a, eye);
let x = nd_solve(nd_from([[2.0, 1.0], [1.0, 3.0]]), nd_from([5.0, 10.0]));
```

| API | Beskrivning |
|-----|-------------|
| `nd_zeros` / `nd_ones` / `nd_full` / `nd_arange` | Skapa |
| `nd_from` / `nd_reshape` / `nd_shape` / `nd_size` | Layout |
| `nd_get` / `nd_set` | Index (flat eller multi) |
| `nd_add` / `nd_mul` / `nd_scale` | Elementvis |
| `nd_sum` / `nd_mean` | Reductions |
| `nd_dot` / `nd_matmul` / `nd_solve` | Linalg |
| `nd_from_f64` / `nd_to_f64` | Float64Array zero-copy wrap |
| `nd_sub` / `nd_div` / broadcast `nd_add`/`nd_mul` | Broadcast-binop |
| `nd_broadcast_to` / `nd_broadcast_shapes` / `broadcastTo` | Explicit expand + shape algebra |
| `nd_max` / `nd_min` / `nd_argmax` / `nd_transpose` / `nd_swapaxes` / `nd_pad` / `nd_roll` / `nd_tensordot` / `nd_einsum` | Reductions + layout + einsum (ellipsis `...`, broadcast) |
| `mat_batch_qr` / `mat_batch_svd` / `mat_batch_solve` / `mat_batch_eig` / `mat_randomized_svd` | Batched decomps + randomized SVD |
| `num_fftn` / `num_firwin` / `num_polyphase_*` / `num_dwt_haar*` / `num_wpt_haar` | FFT + FIR/polyphase + multilevel Haar / WPT |
| `sparse_*` ILU/IC + `sparse_spsolve` / `sparse_ilu_solve` / `sparse_icc_solve` / `sparse_triangular_solve` | Incomplete factors + sparse direct solve |
| `science/domain/pde` | 1D/2D/3D FD + FEM 1D/2D triangles |
| `ag_softmax` create_graph / `scaledDotAttn` | Softmax + attention HOAD subset |
| `nd_abs` / `nd_exp` / `nd_log` / `nd_sqrt` / `nd_clip` / `nd_where` | Ufuncs |
| `nd_slice` / `nd_concat` / `nd_stack` / `nd_split` | Slice / stack |
| `nd_astype` / `nd_dtype` / `nd_seed` / `nd_rand_*` / `nd_save` / `nd_load` | Dtypes, RNG, KND1 I/O |
| `mat_qr` / `mat_qr_err` / `mat_svd` / `mat_pinv` / `mat_eig` / `mat_cholesky` / `mat_lstsq` / `mat_cond` / `mat_lu` / `mat_slogdet` / `mat_norm_ord` | Linalg (SC1e+): QR·SVD·LU, norms, pinv |
| `num_rk4` / `num_odeint` / `num_odeint_adaptive` / `num_quad` | ODE + adaptive + quadrature |
| `ag_*` (+ matmul/conv2d/sigmoid/softmax/ce/add/mul/sub/div/sum/exp/no_grad; `backward(..., true)` / `gradTensor`) | Autograd tape + HOAD subset (SC2n) |
| `ml_adam_update` / `ml_accuracy` / `ml_f1` / `ml_confusion` / `ml_shuffle` / `ml_batch_slices` / `ml_train_test_split` | Adam + metrics + batch |
| `ml_save_checkpoint` / `ml_load_checkpoint` / `ml_cross_entropy` | Model I/O + CE |
| `num_fft` / `num_ifft` / `num_fft_c` / `num_rfft` / `num_irfft` / `num_fft_pad` / `num_conv1d` / `mat_svd2` | Signal + SVD2 + real/complex FFT |
| `csv_*` / `ascii_plot` / `plot_line` / `plot_scatter` / `plot_hist` / `pretty` | Data + viz |
| `ml_pca` / `ml_kmeans` / `ml_logreg_fit` / `ml_logreg_predict` | Klassisk ML |
| `num_root` / `num_minimize` / `num_least_squares` | Optimize |
| `ml_conv2d` / `ml_maxpool2d` / `ml_embedding` / `ml_mha` | NN-lager |
| `num_rk4` / `num_odeint` | ODE |
| `stat_quantile` / `stat_ttest` / `stat_chi2` / `stat_norm_pdf` / `stat_norm_cdf` | Stats++ |
| `gpu_to_device` / `gpu_to_host` / `gpu_linear` / `gpu_conv2d` / `gpu_conv2d_kernel` | GPU train/infer path |
| `gpu_tensor_*` / `gpu_zeros` / `gpu_ones` / `gpu_scale` / `gpu_add` | GPU staging tensors |
| `ml_train_log` | Training progress (rich notebook) |
| `tok_build_vocab` / `tok_encode` / `tok_decode` / `tok_bpe_*` | Tokenizer subset |
| `tf_sinusoidal_pos` / `tf_transformer_forward` / `tf_lm_sgd_step` / `tf_lm_backprop_step` / `tf_stack_forward` / `tf_stack_backprop_step` | Transformer inference + last-layer / multi-layer stack train (`stackForward` / `stackBackpropStep`) |
| `sci_blas_set_num_threads` / `sci_blas_num_threads` / `sci_blas_info` | OpenBLAS/MKL/OMP thread control (`blasSetNumThreads` / `blasInfo`) when the loaded BLAS exposes it |
| `parquet_save` / `parquet_load` | Apache Parquet + KPQT1; nested List (f64/i64/utf8) and flat Struct string fields |
| `sci_bench` / `sci_bench_report` | Science bench harness |
| `ml_stump_fit` / `ml_tree_fit` / `ml_tree_predict` | Decision stump/tree |
| `ml_adamw_update` / `ml_roc_auc` | AdamW + ROC-AUC |
| `gpu_matmul_kernel` / `gpu_available_kernels` / `gpu_conv2d_kernel` | GPU kernel path |
| `num_fir` / `num_moving_average` / `num_iir` / `num_biquad` | FIR/IIR filters |
| `gpu_compute::try_matmul_compute` / `try_conv2d_compute` | WGSL f32 matmul+conv (`--features gpu`) |
| `sci_gemm` / `sci_blas_dgemm` / blocked `nd_matmul` | SC4a GEMM — system OpenBLAS/MKL via `cblas_dgemm` when found (`KABOOTAR_BLAS_LIB`), else `matrixmultiply`; `sci_blas_backend` / `blasBackend()` |
| `job_map_parallel` / `job_map_chunks` | SC4c parallel f64 map + Kab-closure chunks |
| `NdShared` / `nd_slice` views / `a[1:10, :]` | SC0f zero-copy Rc views (no dangling) |
| `nd_take` / `Tensor` / `science/lazy` | Buffer ownership move + GC lazy graphs |
| `nd_gather` / `nd_compress` / `gather` / `compress` | Fancy indexing (real + complex64); mask = logical numel |
| `nd_nonzero` / `nd_fancy_index` / `nd_take_along` / `fancyOuter` | Nonzero; broadcast/outer multi-axis fancy |
| `complex64` / `nd_conj` / `astype(..., "complex64")` | Interleaved `[re,im]` buffer; KND1 tag 6; complex add/mul/sub/div/abs |
| `sci_allreduce_f64` / `allReduce*` / `allReduceRanks` / `allReduceStar|Ring` / `allReduceTcp` / `allReduceTcpRank` | Threaded + mailbox + TCP (in-process + multi-host rank API) (`science/dist`) |
| `science/kab_algo` SC5b | PCA/stump/kmeans + bagging/AdaBoost + **`gbdtFitKab`/`gbdtPredictKab`** |
| `sparse_gather_rows` / `sparse_compress_rows` / `sparse_gather_cols` / `sparse_compress_cols` / `sparse_slice` / `sparse_from_dense_mask` | Sparse fancy row/col/slice views (`science/sparse`) |)

## ML / AI (SC2)

```kabootar
import "science";
import "science/ml";
ml_relu([-1.0, 2.0]);
ml_softmax([1.0, 2.0, 3.0]);
let params = [0.0, 0.0];
params = ml_linreg_step(params, [2.0], 5.0, 0.05);
ml_dense(W, x, b, true);   // relu
job_map([1, 2, 3], double); // P8 API
job_map_parallel([1, 2, 3], "square", 4); // SC4c OS-threads
job_map_chunks([1, 2, 3, 4], double, 2); // Kab-closure chunk plan
sci_blas_dgemm(A, m, k, B, n, 1.0, 0.0, null); // BLAS-style DGEMM
```

Mall: `kabootar mod init science-ai`. Exempel: `examples/science_ai_linreg.kab`.

## Felsökning

| Meddelande | Orsak | Åtgärd |
|------------|-------|--------|
| `expected number` | Fel typ till numerisk funktion | Skicka tal, inte sträng |
| `expected complex number [re, im]` | Fel format till `c_*` | Använd `cplx()` eller array med 2 flyttal |
| `nd_solve: singular matrix` | Ax=b ej lösbar | Kontrollera A |
| `Module not found` | Fel importnamn | `import "science";` |

## Implementation

- Motor (tillfällig hotpath): `src/runtime/science/` — krymper enligt **SC5**; ny produktlogik ska inte växa här
- Kab (produkt-API): `lib/science/*.kab` — **Kab-first** (`fit`, `kab_algo`, `pipeline`, `bootstrap`, `autograd`, `gpu`, …; **SC6:** `preprocess`, `metrics`, `prob`, `graph`, `timeseries`, `rl`, `explain`, `dist`, `domain/*`; **SC7:** `io`, `parallel`, `visualize`, `nd_gpu`; **STEM sandlåda:** `mechanics`)
- Registrering: `science_register` vid `import "science"`
- Tester: `tests/science_sc*.rs` inkl. `science_sc_wave7.rs`
- Demo: `examples/science_freedom_demo.kab` — train + tokenizer + transformer + plot (no Python)

### Policy SC5c (inga nya Rust-produkt-API)

1. **Nya science-features** exporteras via `lib/science/*.kab` (wrappers / Kab-algoritmer).
2. **Rust** får bara nya natives om hotpath kräver det (buffer, SIMD, FFT, GPU kernel) — dokumentera i hotpath-tabellen.
3. **Bootstrap:** `import "science"` (natives) + `import "science/bootstrap"` (Kab-yta).
4. **CI:** wave-tester ska köra Kab-imports, inte bara raw natives, där produkt-API finns.
- IDE-stub: `src/modules/mod.rs` (goto-definition i LSP)
- Ambition & gap: [ROADMAP.md](ROADMAP.md) Våg SC (NumPy / SciPy / Python-AI → Kab-only)

### Hotpath-kontrakt (SC5d)

| Lager | Rust-native (tillfällig) | Kab (`lib/science`) |
|-------|--------------------------|---------------------|
| ndarray bulk | `nd_*`, `nd_from_f64`, matmul, FFT | wrappers, slice/stack, I/O API |
| linalg heavy | QR/SVD/eig, solve | `optimize.kab`, future spline |
| NN hotpath | conv2d, mha, embedding, GPU tensors | `fit.kab`, `ml.kab` training loop |
| tokenizer/transformer | BPE train, transformer forward matmul | `tokenizer.kab`, `transformer.kab` |
| training DX | `ml_train_log` (progress emit) | `fit.kab` — loop, schedulers, early stop |
| GPU compute | WGSL matmul+conv2d (`gpu` feature) via `gpu_compute` | `gpu.kab` wrappers; CPU fallback always |
| GEMM / workers | `matrixmultiply` dgemm; `sci_v*`/`job_map_*` | `nd.kab` gemm/blasDgemm/vadd; `ml.kab` jobMap* |
| Nd views | `NdShared` Rc buffer + strided views | `nd.kab` slice/isView/bufRc/ensureOwned; syntax `a[1:10, :]` |
| Tensor ownership + lazy | `nd_take` unique buffer move; GC graph nodes | `tensor.kab` Tensor/take/meta; `lazy.kab` realize |
| SC7 surface | I/O (KND/CSV/JSONL/KPQT1), local parallel, viz, nd↔GPU | `io.kab`, `parallel.kab`, `visualize.kab`, `nd_gpu.kab` |
| ODE / quad | `num_odeint_adaptive` / `num_quad` | `optimize.kab` |
| signal FIR/IIR | `num_fir`/`num_iir` natives | `signal.kab`; boxcar also in `kab_algo.movingAvgKab` |

Ny **produktlogik** (fit-loop, schedulers, tokenize-pipeline) ska landa i Kab; Rust får bara nya tunna primitives om hotpath kräver det.

