# SH28 — TODO: Kabootar fri från Rust

Status-snapshot för SH28 ("Arkivera produkt-Rust", se `docs/ROADMAP.md` rad 610 och
`self_host/README.md` rad 726). Varje punkt visar **nuvarande** värde på grinden i
`lib/kab/noll/` (källan är auktoritativ: en grön test betyder bara att testet som
*läser* grinden passerar — de flesta `noll*`-tester asserterar att grinden fortfarande
är `false`).

- [ ] = grinden är `false` (arbete kvar) · [x] = grinden är `true`

## 0. Testkörning som listan bygger på

Körningsmiljö (samma som `.github/workflows/self-host.yml`):

```bash
RUST_MIN_STACK=67108864 KABOOTAR_COMPILE=self-host \
  cargo test --test sh_wave -- --test-threads=1
```

- Testmålet är `tests/sh_wave.rs` (autodetekterat av Cargo, inget `[[test]]`, inget
  `--manifest-path`, inga feature-flaggor). Workflowen kör bara delmängden
  `cargo test --test sh_wave sh16_ -- --test-threads=1`; hela vågen körs enligt
  `self_host/README.md`.
- Workflowen sätter **inte** `KAB_TLS_PEERS=1`. Den sätts bara internt av
  `sh23_crypto_host_delete_probe` / `sh23_crypto_host_delete_tail_probe` i
  `tests/sh23_tls_peer.rs` (separat testmål, ingår inte i `sh_wave`).

### Resultat

**Körning A — originalkoden (`main`).** Exakt kommando i en process kraschar redan på test 7 (`f10_aot_apply_reloc_base_reject_exec`: `memory allocation of 128297923241120 bytes failed`, SIGABRT), så libtest skriver aldrig någon sammanfattning. Sviten kördes därför via ett drivskript som kör samma binär med `--test-threads=1 --exact` i 8 delar och noterar kraschat test som ABORT och fortsätter.

**Körning B — med fixen i denna PR** (`merge_object_fields`: hoppa över när `from` och `into` är samma `Rc`), samma drivskript (från ~2240 tester startas processen om var 60:e test för att undvika OOM). **Delresultat: 3723/4603 tester körda när filen skrevs; körningen pågår.**

| | Totalt | ok | FAILED | ABORT (krasch) | ignored |
|---|---|---|---|---|---|
| A (original) | 4603 | 3296 | 29 | 1273 | 5 |
| B (med fix) | 3723 | 3687 | 28 | 5 | 3 |

- `noll*`-tester, körning A: 385 st — 382 ok, 0 FAILED, 3 ABORT: `f10_aot_pgo_fp_cold_ss_noll_aot_ready_dual_bind_arm64_exec`, `f10_aot_pgo_fp_cold_ss_noll_aot_ready_dual_bind_exec`, `f10_aot_pgo_fp_cold_ss_noll_aot_ready_dual_bind_reject_exec`
- `noll*`-tester, körning B: 147 st — 147 ok, 0 FAILED, 0 ABORT

`sh*`-smokes per våg (ok / FAILED / ABORT / ignored):

| Våg | A (original) | B (med fix) |
|---|---|---|
| sh0 | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 |
| sh1 | 3 / 0 / 0 / 1 | 3 / 0 / 0 / 1 |
| sh2 | 1 / 1 / 1 / 0 | 2 / 1 / 0 / 0 |
| sh3a | 1 / 0 / 0 / 1 | 1 / 0 / 0 / 1 |
| sh3b | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 |
| sh3c | 0 / 0 / 1 / 0 | 1 / 0 / 0 / 0 |
| sh4 | 1 / 1 / 0 / 0 | 1 / 1 / 0 / 0 |
| sh6 | 24 / 1 / 516 / 0 | 84 / 0 / 0 / 0 |
| sh7 | 1 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| sh7b | 1 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| sh8 | 1 / 0 / 3 / 0 | 0 / 0 / 0 / 0 |
| sh9 | 2 / 4 / 1 / 0 | 0 / 0 / 0 / 0 |
| sh10 | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 |
| sh12 | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 |
| sh15 | 1 / 1 / 0 / 0 | 1 / 1 / 0 / 0 |
| sh16 | 5 / 0 / 1 / 0 | 6 / 0 / 0 / 0 |
| sh17 | 174 / 0 / 33 / 0 | 207 / 0 / 0 / 0 |
| sh18 | 63 / 1 / 32 / 0 | 95 / 0 / 0 / 0 |
| sh19 | 53 / 0 / 4 / 0 | 50 / 0 / 0 / 0 |
| sh20 | 244 / 0 / 2 / 0 | 233 / 0 / 0 / 0 |
| sh21 | 25 / 0 / 2 / 0 | 27 / 0 / 0 / 0 |
| sh22 | 28 / 0 / 1 / 0 | 29 / 0 / 0 / 0 |
| sh23 | 25 / 1 / 17 / 0 | 37 / 1 / 0 / 0 |
| sh24 | 18 / 1 / 1 / 0 | 14 / 1 / 0 / 0 |
| sh25 | 55 / 0 / 1 / 0 | 36 / 0 / 0 / 0 |
| sh26 | 65 / 0 / 1 / 0 | 41 / 0 / 0 / 0 |
| sh27 | 922 / 0 / 9 / 0 | 860 / 1 / 0 / 0 |
| sh28 | 380 / 0 / 22 / 3 | 149 / 0 / 1 / 1 |
| sh29 | 0 / 0 / 1 / 0 | 0 / 0 / 0 / 0 |
| sh30 | 0 / 0 / 1 / 0 | 0 / 1 / 0 / 0 |
| sh31 | 0 / 0 / 1 / 0 | 0 / 1 / 0 / 0 |
| sh32 | 0 / 0 / 1 / 0 | 0 / 1 / 0 / 0 |
| sh33 | 0 / 0 / 1 / 0 | 0 / 1 / 0 / 0 |

Röda/ignorerade tester i körning B:

- `f10_aot_apply_reloc_dual_bind_load_verify_ship_at40_arm64_exec` — ABORT (OOM-kill: testprocessen hade vuxit till >10 GB efter ~1000 tester; inte testets fel)
- `f10_aot_apply_reloc_dual_bind_rodata_hex_loaded_arm64_exec` — ABORT (OOM-kill: testprocessen hade vuxit till >10 GB efter ~1000 tester; inte testets fel)
- `f10_aot_jg_smokes_in_kab` — FAILED (läser borttagna `examples/f10_aot_verify_jg_*.kab`)
- `f10_aot_pgo_fp_cold_ss_entry_load_verify_ship_arm64_exec` — ABORT (OOM-kill: testprocessen hade vuxit till >10 GB efter ~1000 tester; inte testets fel)
- `f10_aot_pgo_fp_cold_ss_sym_reloc_seal_dual_bind_reject_exec` — ABORT (OOM-kill: testprocessen hade vuxit till >10 GB efter ~1000 tester; inte testets fel)
- `f14_io_aread_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_async_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_await_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_del_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_get_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_has_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_list_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_mkdir_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_put_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f14_io_stat_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f20_jit_spec_deopt_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f20_jit_spec_f64_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f20_jit_spec_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f21_aot_lazy_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f21_aot_lazy_jit_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f24_jit_tco_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `f24_jit_tco_self_in_kab` — FAILED (läser borttagen `lib/kab/<modul>.kab` — NotFound)
- `sh15_ca_kbcb_mmap_hit_skips_text` — FAILED (CA-cache `cache/ca/v*_*.kbcb` skapas inte)
- `sh1_warm_full_compile_dag` — ignored
- `sh23_crypto_tls_ecdsa_eval_smoke` — FAILED (`tlsCertEcdsaEvalOk()` returnerar inte `true`)
- `sh24_http_host_in_kab` — FAILED (läser gammal sökväg `lib/kab/http_host.kab`; filen ligger nu i `lib/kab/http/`)
- `sh27_krender_display_list_smoke` — FAILED (`KABOOTAR_COMPILE=self-host`: "SH16: rust compile disabled for app")
- `sh28_aot_native_exec_smoke` — ABORT (stack overflow i tråden `sh28-aot-native-exec`)
- `sh28_self_host_seed_write_smoke` — ignored
- `sh2_parser_emit_exec_are_per_call_session` — FAILED (källtext-assert: `pMakeSession()`/`tramp(sess)` finns inte längre i parsern)
- `sh30_aot_calln_src_smoke` — FAILED (FAILED i sviten; >15 min timeout isolerat)
- `sh31_aot_bitops_src_smoke` — FAILED (FAILED i sviten)
- `sh32_aot_pow_src_smoke` — FAILED (FAILED i sviten)
- `sh33_aot_link_src_smoke` — FAILED (FAILED i sviten)
- `sh3a_self_host_push_len_nested` — ignored
- `sh4_kbcb_v2_faster_than_text` — FAILED (läser borttagen `self_host/seed/emit_impl.kab.kbc`)

## 1. Aggregatgrind

- [ ] `nollHostDeleteOk` = **false** (`lib/kab/noll/noll_host.kab`) — måste bli `true`.
  Blir det först när alla punkter nedan är klara.
  - [x] `nollPendingHostGatesClosed` = true (inga SH19–SH27-grindar väntar längre)
  - [ ] `nollAllHostGatesClosed` = false (beräknad; returnerar `false` eftersom
    `loadMainDeleteOk()` redan är `true` — semantiken "alla host-grindar stängda"
    är föråldrad och bör städas/inverteras när SH28 stängs)

## 2. Ersätt produkt-Rust med Kab (`noll*IsKab` = false)

| Klar | Modul | Grind | Värde |
|---|---|---|---|
| [ ] | `lib/kab/noll/noll_jit.kab` | `nollJitIsKab` | false |
| [ ] | `lib/kab/noll/noll_std.kab` | `nollStdIsKab` | false |
| [ ] | `lib/kab/noll/noll_sha.kab` | `nollShaIsKab` | false |
| [ ] | `lib/kab/noll/noll_hqc.kab` | `nollHqcIsKab` | false |
| [ ] | `lib/kab/noll/noll_slhdsa.kab` | `nollSlhdsaIsKab` | false |
| [ ] | `lib/kab/noll/noll_noise.kab` | `nollNoiseIsKab` | false |
| [ ] | `lib/kab/noll/noll_shuffle.kab` | `nollShuffleIsKab` | false |
| [ ] | `lib/kab/noll/noll_hybridanalysis.kab` | `nollHybridanalysisIsKab` | false |
| [ ] | `lib/kab/noll/noll_lock.kab` | `nollLockIsKab` | false |
| [ ] | `lib/kab/noll/noll_watch.kab` | `nollWatchIsKab` | false |
| [ ] | `lib/kab/noll/noll_bench.kab` | `nollBenchIsKab` | false |
| [ ] | `lib/kab/noll/noll_fake.kab` | `nollFakeIsKab` | false |

Övriga `*IsKab`-grindar som också är `false` finns i bilagan (§9).

## 3. Rustc-fri process-start

- [ ] `nollAotReady` = false (`noll_aot.kab`)
- [ ] `nollImageIsProcess` = false (`noll_imgproc.kab`)
- [ ] `nollMmapExecProcess` = false (`noll_mmexec.kab`)
- [ ] `nollStubIsProcess` = false (`noll_stubproc.kab`)
- [x] `nollAotProcess` = true (`noll_aotproc.kab`) — redan klar
- [x] `nollBootstrapFromKabOk` = true (`noll_bootk.kab`) — redan klar

## 4. Syscalls i Kab

- [ ] `nollSyscallIsKab` = false (`noll_syskab.kab`)
- [ ] `nollHostSyscallGone` = false (`noll_syscall.kab`)

## 5. Ta bort rustc som host/CI-beroende

- [ ] `nollRustcNotHost` = false (`noll_rchost.kab`)
- [ ] `nollRustcNotProcess` = false (`noll_rustc.kab`)
- [ ] `nollRustcCiGone` = false (`noll_ci.kab`) — alla fyra workflows
  (`kabootar-artifact.yml`, `kabtest.yml`, `kv8-smoke.yml`, `self-host.yml`) kör
  fortfarande `cargo`
- [ ] `nollKabtestProductCi` = false (`noll_kabtest.kab`)

## 6. Ta bort Cargo-runtime

- [ ] `nollCargoNotRuntime` = false (`noll_cargo.kab`)
- [ ] `nollCargoTomlGone` = false (`noll_toml.kab`)

## 7. SH19 / SH23

- [x] `loadMainDeleteOk` = **true** (`lib/kab/load/load_main.kab`, SH19).
  Obs: `self_host/README.md` (punkt 16) säger fortfarande "`loadMainDeleteOk` still
  false" — dokumentationen behöver uppdateras.
- [ ] SH23 TLS 1.3 live-handshake (`cryptoHostDeleteOk` → `cryptoTlsPeerEvidenceOk()`
  i `lib/kab/crypto/crypto_host.kab`) = **false** i `sh_wave`/CI: grinden returnerar
  bara `true` när `KAB_TLS_PEERS=1` är satt och `cryptoTls13AllOk()` lyckas mot en
  levande rustls-peer. Ingen workflow sätter `KAB_TLS_PEERS`.
  - Live-beviset finns bara i `tests/sh23_tls_peer.rs`
    (`sh23_crypto_host_delete_probe`, `sh23_crypto_host_delete_tail_probe`; peers på
    `:28291` TLS 1.2 och `:28296` TLS 1.3). Lokalt (debug-build, med fixen) blev
    `sh23_crypto_host_delete_probe` inte klar inom 74 min (CPU-bunden Kab-kryptografi i debug-build; avbruten), så live-handshaken är **inte** verifierad i den här körningen.
  - Kvar: kör probe-testerna i CI (eget jobb med `KAB_TLS_PEERS=1`, release-build),
    och låt `nollPendingHostGatesClosed` bero på faktiskt CI-resultat i stället för
    en hårdkodad `true`.

## 8. Tester och arkivering av `src/` + `Cargo.toml`

- [ ] Migrera `tests/*.rs` → `tests/*_test.kab`: idag **177** `tests/*.rs` mot
  **17** `tests/*_test.kab`.
- [ ] `nollDropSrc` = false (`noll_drop.kab`)
- [ ] `nollProductSrcGone` = false (`noll_srcprod.kab`) — idag **314** `src/**/*.rs`
  (~135 500 rader)
- [ ] `nollUserNoRustc` = false (`noll_user.kab`)
- [x] `nollKeepSrc` = true, `nollNoNewRs` = true (policy tills ovan är klart)


## 9. Bilaga — övriga `noll*`-grindar som är `false`

Utöver punkterna ovan returnerar dessa 154 grindar i `lib/kab/noll/` literal `false` (totalt 182 av 192 publika grindar; 6 är `true`, 4 är beräknade). Var och en motsvarar en del av produkt-Rust som ska ersättas av Kab eller tas bort:

- [ ] `nollAddIsKab` (`noll_add.kab`)
- [ ] `nollAesIsKab` (`noll_aes.kab`)
- [ ] `nollAgeIsKab` (`noll_age.kab`)
- [ ] `nollAnyrunIsKab` (`noll_anyrun.kab`)
- [ ] `nollArgonIsKab` (`noll_argon.kab`)
- [ ] `nollAttackIsKab` (`noll_attack.kab`)
- [ ] `nollAuthIsKab` (`noll_auth.kab`)
- [ ] `nollBcryptIsKab` (`noll_bcrypt.kab`)
- [ ] `nollBikeIsKab` (`noll_bike.kab`)
- [ ] `nollCapecIsKab` (`noll_capec.kab`)
- [ ] `nollCapeIsKab` (`noll_cape.kab`)
- [ ] `nollCatIsKab` (`noll_cat.kab`)
- [ ] `nollChachaIsKab` (`noll_chacha.kab`)
- [ ] `nollCisaIsKab` (`noll_cisa.kab`)
- [ ] `nollClamavIsKab` (`noll_clamav.kab`)
- [ ] `nollCleanIsKab` (`noll_clean.kab`)
- [ ] `nollCliIsKab` (`noll_cli.kab`)
- [ ] `nollClockIsKab` (`noll_clock.kab`)
- [ ] `nollCompileIsKab` (`noll_compile.kab`)
- [ ] `nollCortexIsKab` (`noll_cortex.kab`)
- [ ] `nollCosignIsKab` (`noll_cosign.kab`)
- [ ] `nollCovIsKab` (`noll_cov.kab`)
- [ ] `nollCpeIsKab` (`noll_cpe.kab`)
- [ ] `nollCraneliftGone` (`noll_cranelift.kab`)
- [ ] `nollCuckooIsKab` (`noll_cuckoo.kab`)
- [ ] `nollCveIsKab` (`noll_cve.kab`)
- [ ] `nollCvssIsKab` (`noll_cvss.kab`)
- [ ] `nollCweIsKab` (`noll_cwe.kab`)
- [ ] `nollCyclonedxIsKab` (`noll_cyclonedx.kab`)
- [ ] `nollDbgIsKab` (`noll_dbg.kab`)
- [ ] `nollDhcpIsKab` (`noll_dhcp.kab`)
- [ ] `nollDilithiumIsKab` (`noll_dilithium.kab`)
- [ ] `nollDnsIsKab` (`noll_dns.kab`)
- [ ] `nollDocIsKab` (`noll_doc.kab`)
- [ ] `nollEd25519IsKab` (`noll_ed25519.kab`)
- [ ] `nollEmitIsKab` (`noll_emit.kab`)
- [ ] `nollFalconIsKab` (`noll_falcon.kab`)
- [ ] `nollFixIsKab` (`noll_fix.kab`)
- [ ] `nollFmtIsKab` (`noll_fmt.kab`)
- [ ] `nollFndsaIsKab` (`noll_fndsa.kab`)
- [ ] `nollFrodoIsKab` (`noll_frodo.kab`)
- [ ] `nollFtpIsKab` (`noll_ftp.kab`)
- [ ] `nollFulcioIsKab` (`noll_fulcio.kab`)
- [ ] `nollFuzzIsKab` (`noll_fuzz.kab`)
- [ ] `nollGcIsKab` (`noll_gc.kab`)
- [ ] `nollGemssIsKab` (`noll_gemss.kab`)
- [ ] `nollGhsaIsKab` (`noll_ghsa.kab`)
- [ ] `nollGrpcIsKab` (`noll_grpc.kab`)
- [ ] `nollHkdfIsKab` (`noll_hkdf.kab`)
- [ ] `nollHmacIsKab` (`noll_hmac.kab`)
- [ ] `nollHostOptional` (`noll_hopt.kab`)
- [ ] `nollHotpIsKab` (`noll_hotp.kab`)
- [ ] `nollHpkeIsKab` (`noll_hpke.kab`)
- [ ] `nollHttpIsKab` (`noll_http.kab`)
- [ ] `nollHybridIsKab` (`noll_hybrid.kab`)
- [ ] `nollIcmpIsKab` (`noll_icmp.kab`)
- [ ] `nollImapIsKab` (`noll_imap.kab`)
- [ ] `nollInitIsKab` (`noll_init.kab`)
- [ ] `nollIntotoIsKab` (`noll_intoto.kab`)
- [ ] `nollJoeIsKab` (`noll_joe.kab`)
- [ ] `nollJwksIsKab` (`noll_jwks.kab`)
- [ ] `nollJwtIsKab` (`noll_jwt.kab`)
- [ ] `nollKerberosIsKab` (`noll_kerberos.kab`)
- [ ] `nollKevIsKab` (`noll_kev.kab`)
- [ ] `nollKyberIsKab` (`noll_kyber.kab`)
- [ ] `nollLdapIsKab` (`noll_ldap.kab`)
- [ ] `nollLexIsKab` (`noll_lex.kab`)
- [ ] `nollLinkIsKab` (`noll_link.kab`)
- [ ] `nollLmsIsKab` (`noll_lms.kab`)
- [ ] `nollLogIsKab` (`noll_log.kab`)
- [ ] `nollLsIsKab` (`noll_ls.kab`)
- [ ] `nollLspIsKab` (`noll_lsp.kab`)
- [ ] `nollMcelieceIsKab` (`noll_mceliece.kab`)
- [ ] `nollMerkleIsKab` (`noll_merkle.kab`)
- [ ] `nollMinisignIsKab` (`noll_minisign.kab`)
- [ ] `nollMispIsKab` (`noll_misp.kab`)
- [ ] `nollMitreIsKab` (`noll_mitre.kab`)
- [ ] `nollMldsaIsKab` (`noll_mldsa.kab`)
- [ ] `nollMlkemIsKab` (`noll_mlkem.kab`)
- [ ] `nollMmapStub` (`noll_mmap.kab`)
- [ ] `nollMockIsKab` (`noll_mock.kab`)
- [ ] `nollModIsKab` (`noll_mod.kab`)
- [ ] `nollMqttIsKab` (`noll_mqtt.kab`)
- [ ] `nollNetIsKab` (`noll_net.kab`)
- [ ] `nollNewIsKab` (`noll_new.kab`)
- [ ] `nollNotaryIsKab` (`noll_notary.kab`)
- [ ] `nollNtpIsKab` (`noll_ntp.kab`)
- [ ] `nollNtruIsKab` (`noll_ntru.kab`)
- [ ] `nollNvdIsKab` (`noll_nvd.kab`)
- [ ] `nollOauthIsKab` (`noll_oauth.kab`)
- [ ] `nollOidcIsKab` (`noll_oidc.kab`)
- [ ] `nollOpenctiIsKab` (`noll_opencti.kab`)
- [ ] `nollOptIsKab` (`noll_opt.kab`)
- [ ] `nollOsqueryIsKab` (`noll_osquery.kab`)
- [ ] `nollOsvIsKab` (`noll_osv.kab`)
- [ ] `nollParseIsKab` (`noll_parse.kab`)
- [ ] `nollPbkdfIsKab` (`noll_pbkdf.kab`)
- [ ] `nollPgpIsKab` (`noll_pgp.kab`)
- [ ] `nollPicnicIsKab` (`noll_picnic.kab`)
- [ ] `nollPkgIsKab` (`noll_pkg.kab`)
- [ ] `nollPolyIsKab` (`noll_poly.kab`)
- [ ] `nollPopIsKab` (`noll_pop.kab`)
- [ ] `nollProcessIsKab` (`noll_proc.kab`)
- [ ] `nollProfIsKab` (`noll_prof.kab`)
- [ ] `nollPubIsKab` (`noll_pub.kab`)
- [ ] `nollQuicIsKab` (`noll_quic.kab`)
- [ ] `nollRadiusIsKab` (`noll_radius.kab`)
- [ ] `nollRainbowIsKab` (`noll_rainbow.kab`)
- [ ] `nollRandIsKab` (`noll_rand.kab`)
- [ ] `nollRegIsKab` (`noll_reg.kab`)
- [ ] `nollRekorIsKab` (`noll_rekor.kab`)
- [ ] `nollReplIsKab` (`noll_repl.kab`)
- [ ] `nollRmIsKab` (`noll_rm.kab`)
- [ ] `nollSamlIsKab` (`noll_saml.kab`)
- [ ] `nollSanIsKab` (`noll_san.kab`)
- [ ] `nollSbomIsKab` (`noll_sbom.kab`)
- [ ] `nollScryptIsKab` (`noll_scrypt.kab`)
- [ ] `nollSctpIsKab` (`noll_sctp.kab`)
- [ ] `nollSigmaIsKab` (`noll_sigma.kab`)
- [ ] `nollSignifyIsKab` (`noll_signify.kab`)
- [ ] `nollSigstoreIsKab` (`noll_sigstore.kab`)
- [ ] `nollSikeIsKab` (`noll_sike.kab`)
- [ ] `nollSlsaIsKab` (`noll_slsa.kab`)
- [ ] `nollSmtpIsKab` (`noll_smtp.kab`)
- [ ] `nollSnapIsKab` (`noll_snap.kab`)
- [ ] `nollSnmpIsKab` (`noll_snmp.kab`)
- [ ] `nollSnortIsKab` (`noll_snort.kab`)
- [ ] `nollSpdxIsKab` (`noll_spdx.kab`)
- [ ] `nollSphincsIsKab` (`noll_sphincs.kab`)
- [ ] `nollSpyIsKab` (`noll_spy.kab`)
- [ ] `nollSshIsKab` (`noll_ssh.kab`)
- [ ] `nollStixIsKab` (`noll_stix.kab`)
- [ ] `nollStubFrozen` (`noll_stub.kab`)
- [ ] `nollSuricataIsKab` (`noll_suricata.kab`)
- [ ] `nollTaxiiIsKab` (`noll_taxii.kab`)
- [ ] `nollTftpIsKab` (`noll_tftp.kab`)
- [ ] `nollThehiveIsKab` (`noll_thehive.kab`)
- [ ] `nollTlsIsKab` (`noll_tls.kab`)
- [ ] `nollTotpIsKab` (`noll_totp.kab`)
- [ ] `nollTraceIsKab` (`noll_trace.kab`)
- [ ] `nollTypeIsKab` (`noll_type.kab`)
- [ ] `nollUdpIsKab` (`noll_udp.kab`)
- [ ] `nollVexIsKab` (`noll_vex.kab`)
- [ ] `nollVirustotalIsKab` (`noll_virustotal.kab`)
- [ ] `nollVmIsKab` (`noll_vm.kab`)
- [ ] `nollWazuhIsKab` (`noll_wazuh.kab`)
- [ ] `nollWebauthnIsKab` (`noll_webauthn.kab`)
- [ ] `nollWotsIsKab` (`noll_wots.kab`)
- [ ] `nollWsIsKab` (`noll_ws.kab`)
- [ ] `nollX25519IsKab` (`noll_x25519.kab`)
- [ ] `nollXmssIsKab` (`noll_xmss.kab`)
- [ ] `nollXwingIsKab` (`noll_xwing.kab`)
- [ ] `nollYaraIsKab` (`noll_yara.kab`)
- [ ] `nollZeekIsKab` (`noll_zeek.kab`)
