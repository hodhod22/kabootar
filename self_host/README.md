# Self-hosted Kabootar compiler

**Slutmål (nolltolerans):** hela stacken är `.kab`. Rust är skuld tills [SH28](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust) **arkiverar** den när Kab är stabil. Plan: [docs/ROADMAP.md — Kabootar på egna fötter](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust).

Produktkompilatorn är `self_host/compile.kab`. Plan: **[docs/ROADMAP.md — Våg SH](../docs/ROADMAP.md)**. SH6: Kab-VM **kab-only default**; packed `.kbcb` run; **`in`**; **`await`**; **`array_slice_from`**; **`new_instance_from_array`**; self-host **`let [a, ...rest]`** / **`let { a, ...rest }`** / **`let { x: [a, b] }`** / **`[1, ...xs]`** / **`{ ...obj }`** / **`{ a }`** / **`{ foo() {} }`** / **`{ [k]: v }`** / **`is`/`is not`** / **`fn f(a, b = 3)`** / **`fn f(a, ...xs)`** / **`(a, b = 3) =>`** / **`(a, ...xs) =>`** / **`async (n) =>`** / **`class C { fn add(a, b = 3) }`** / **`fn rest(a, ...xs)`** / **`{ add(a, b = 3) {} }`** / **`{ rest(a, ...xs) {} }`** / **`trait T { fn add(a, b = 3) }`** / **`fn rest(a, ...xs)`** / **`o?.x`** / **`xs?.[0]`** / **`f?.()`** / **`delete o.z`** / **`delete o.a.b`** / **`delete xs[0].x`** / **`delete o[k]`** / **`delete o.items[0].x`** / **`delete xs[0][0].x`** / **`delete this.z`** / **`delete o.items[0][0].x`** / **`delete o.a.b.c`** / **`delete this.a.b`** / **`delete o[k].x`** / **`delete super.z`** / **`delete this[k]`** / **`delete super[k]`** / **`delete super.a.b`** / **`delete o[k][j]`** / **`delete super.a[k]`** / **`delete this[k].x`** / **`delete super[k].x`** / **`delete this.a[k]`** / **`delete o.a[k]`** / **`delete this[k][j]`** / **`delete super[k][j]`** / **`delete o.items[0][k]`** / **`delete this.a.b[k]`** / **`delete o.a.b[k]`** / **`delete xs[0][0][k]`** / **`delete super.a.b[k]`** / **`delete this.items[0][k]`** / **`delete super.items[0][k]`** / **`delete this.items[0][0][k]`** / **`delete o.items[0][0][k]`** / **`delete super.items[0][0][k]`** / **`n &= 3`** / **`n |= 2`** / **`n ^= 3`** / **`o.x &= 3`** / **`xs[0] |= 2`** / **`o.x ^= 3`** / **`this.n &= 3`** / **`xs[0] ^= 3`** / **`super.n |= 2`** / **`o.x |= 2`** / **`xs[0] &= 3`** / **`this.n |= 2`** / **`this.n ^= 3`** / **`super.n &= 3`** / **`super.n ^= 3`** / **`o.a.b &= 3`** / **`o.a.b |= 2`** / **`o.a.b ^= 3`** / **`xs[0].x &= 3`** / **`xs[0].x |= 2`** / **`xs[0].x ^= 3`** / **`o.items[0] &= 3`** / **`o.items[0] |= 2`** / **`o.items[0] ^= 3`** / **`o.items[0][0] &= 3`** / **`o.items[0][0] |= 2`** / **`o.items[0][0] ^= 3`** / **`xs[0][0].x &= 3`** / **`xs[0][0].x |= 2`** / **`xs[0][0].x ^= 3`** / **`xs[0][0] &= 3`** / **`xs[0][0] |= 2`** / **`xs[0][0] ^= 3`** / **`o.items[0][0].x &= 3`** / **`o.items[0][0].x |= 2`** / **`o.items[0][0].x ^= 3`** / **`xs[0][0][0] &= 3`** / **`xs[0][0][0] |= 2`** / **`xs[0][0][0] ^= 3`** / **`n <<= 1`** / **`n >>= 1`** / **`n >>>= 1`** / **`o.x <<= 1`** / **`o.x >>= 1`** / **`o.x >>>= 1`** / **`xs[0] <<= 1`** / **`xs[0] >>= 1`** / **`xs[0] >>>= 1`** / **`this.n <<= 1`** / **`this.n >>= 1`** / **`this.n >>>= 1`** / **`super.n <<= 1`** / **`super.n >>= 1`** / **`super.n >>>= 1`** / **`o.a.b <<= 1`** / **`o.a.b >>= 1`** / **`o.a.b >>>= 1`** / **`xs[0].x <<= 1`** / **`xs[0].x >>= 1`** / **`xs[0].x >>>= 1`** / **`o.items[0] <<= 1`** / **`o.items[0] >>= 1`** / **`o.items[0] >>>= 1`** / **`o.items[0][0] <<= 1`** / **`o.items[0][0] >>= 1`** / **`o.items[0][0] >>>= 1`** / **`xs[0][0].x <<= 1`** / **`xs[0][0].x >>= 1`** / **`xs[0][0].x >>>= 1`** / **`o.items[0][0].x <<= 1`** / **`o.items[0][0].x >>= 1`** / **`o.items[0][0].x >>>= 1`** / **`xs[0][0] <<= 1`** / **`xs[0][0] >>= 1`** / **`xs[0][0] >>>= 1`** / **`xs[0][0][0] <<= 1`** / **`xs[0][0][0] >>= 1`** / **`xs[0][0][0] >>>= 1`** / **`n **= 2`** / **`o.x **= 2`** / **`xs[0] **= 2`** / **`this.n **= 2`** / **`super.n **= 2`** / **`o.a.b **= 2`** / **`xs[0].x **= 2`** / **`o.items[0] **= 2`** / **`o.items[0][0] **= 2`** / **`xs[0][0].x **= 2`** / **`o.items[0][0].x **= 2`** / **`xs[0][0] **= 2`** / **`xs[0][0][0] **= 2`** / **`n %= 7`** / **`o.x %= 7`** / **`xs[0] %= 7`** / **`this.n %= 7`** / **`super.n %= 7`** / **`o.a.b %= 7`** / **`xs[0].x %= 7`** / **`o.items[0] %= 7`** / **`o.items[0][0] %= 7`** / **`xs[0][0].x %= 7`** / **`o.items[0][0].x %= 7`** / **`xs[0][0] %= 7`** / **`xs[0][0][0] %= 7`** / **`n -= 2`** / **`o.x -= 2`** / **`xs[0] -= 2`** / **`this.n -= 2`** / **`super.n -= 2`** / **`o.a.b -= 2`** / **`xs[0].x -= 2`** / **`o.items[0] -= 2`** / **`o.items[0][0] -= 2`** / **`xs[0][0].x -= 2`** / **`o.items[0][0].x -= 2`** / **`xs[0][0] -= 2`** / **`xs[0][0][0] -= 2`** / **`n *= 3`** / **`n /= 2`** / **`switch`** / **`fallthrough`** / **`do while`** / **`xs[0] +=`** / **`step()?`/`bad()?`** / **`? :`** / **`import.meta`** / **`` `n=${n}` ``** / **`||=` `&&=` `??=`** / **`??`** / **`for let i = 0`** / **`for x of`** / **`for k in`** / **`match 1 { 1 => 2, _ => 0 }`** / **`match [x, y]`** / **`match { p, q }`** / **`if let Some(x)`** / **`while let Ok(v)`** / **`match 1..=5`** / **`n @ 1..=5`** / **`1 | 2 | 3`** / **`..5`** / **`5..`** / **`[h, ...t]`** / **`{ k, ...s }`** / **`[h, ...mid, last]`** / **`n @ 1..=5 if n != 3`** / **`Color.Red`** / **`Msg.Move(p)`** / **`xs @ [p, q]`** / **`wrap @ { k, ...s }`** / **`{ k: n @ 1..=5 }`** / **`[n @ 1, ...r]`** / **`Ok(n @ 1..=5)`** / **`Some(n @ 1..=5)`** / **`if let 1 | 2`** / **`while let 1 | 2`** / **`(1 | 2)`** / **`Option.Some(n)`** / **`Option.Some("x")`** / **`Option<Number>.None`** / **`1.0..=2.0`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** / **`if let n @ Some(x)`** / **`if let 1.. = x`** / **`while let 1.. = r`** / **`if let ..5 = x`** / **`while let ..5 = r`** / **`n @ 1 | 2`** / **`v @ Msg.Move(x)`** / **`struct Box<T>`** / **`Box$String`** / **`echo$Number`** / **`echo$String`** / **`Box<String>("hi")`** / **`Child$Number`** / **`id<Number>(42)`** / **`id$String`** / **`id(id(42))`** / **`pair$Number_String`** / **`id$Box`** / **`pair(x, s)`** / **`len(wrap(1))`** / **`super.init`** / **`super.count = 1`** / **`super.n += 2`** / **`let m = super.tag`** / **`this.run(super.f)`** / **`Show<Number>`** / **`type Item = Number`** / **`where T: Show`** / **`show_it<Shown>`** / **`Box().show_it<Shown>`** / **`show_it<Nope>`** / **`Box().show_it<Nope>`** / **`Box<Nope>`** / **`where T: Show, T: Named`** / **`both_it<OnlyShow>`** / **`where A: Show, B: Named`** / **`pair_it<Shown, Nope>`** / **`PairBox<Shown, Labeled>`** / **`PairBox<Shown, Nope>`** / **`Box().join_ab<Shown, Labeled>`** / **`Box().join_ab<Shown, Nope>`** / **`Box().both_it<Shown>`** / **`Box().both_it<OnlyShow>`** / **`BothBox<Shown>`** / **`BothBox<OnlyShow>`** / **`WBox<Shown>`** / **`WBox<Nope>`** / **`Thing().id()`** / **`id() { return 42 }`** / **`Show<T> default`** / **`Show<T> default override`** / **`is(obj, "Class")`** / **`pass`/`assert`/`not`** / **`raise`** / **`o.x +=`** / **`o.a.b +=`** / **`o.items[0] +=`** / **`o.items[0][0] +=`** / **`xs[0].x +=`** / **`xs[0][0].x +=`** / **`o.items[0][0].x +=`** / **`xs[0][0] +=`** / **`xs[0][0][0] +=`** / **`o.x ||= `** / **`o.x &&=`** / **`o.x ??=`** / **`xs[0] ||= `** / **`xs[0] ??=`** / **`o.a.b ??=`** / **`o.items[0] ||= `** / **`xs[0].x ??=`** / **`xs[0][0] ||= `** / **`this.n ||= `** / **`o.items[0][0] ||= `** / **`xs[0][0].x ??=`** / **`super.n ||= `** / **`o.items[0][0].x ??=`** / **`xs[0][0][0] ||= `** / **`Child<T> super.n ||= `**. 100 loop / 200 unrolled. 1k+ loop är nested-interpreter. Text-`maxKbc` oförändrad. Inte `noll_*`. Språkparitet i `.kab`, inte `src/`.

Språkparitet: [docs/LANGUAGE.md](../docs/LANGUAGE.md) (inte den här filen). **Dok efter varje deepen:** uppdatera [docs/ROADMAP.md](../docs/ROADMAP.md) (status + **Nästa**) och den här filen (nuläge + milstolpar + tester) i samma pass. Språkparitet: även [docs/LANGUAGE.md](../docs/LANGUAGE.md).

**Nolltolerans:** [docs/ROADMAP.md — strikt arbetsregel](../docs/ROADMAP.md#nolltolerans--strikt-arbetsregel). **Just nu:** [docs/ROADMAP.md — Just nu](../docs/ROADMAP.md#just-nu). Bekräfta SH-/F-/GP-/SC-steg innan kod. Ingen ny `src/**/*.rs`. Hoppa inte över SH5 → SH16 → SH6 → SH17 → … → SH28. Radera inte Rust förrän Kab är stabil; när Kab-yta finns, använd den.

**Akutundantag:** Bara för en dokumenterad blockerare för säkerhet, korrekthet eller körbarhet; kräv minsta ändring, riktat test och ett uttalat steg tillbaka till Kab-spåret. Det ersätter inte den normala arbetsordningen. Se [ROADMAP.md — Akutundantag](../docs/ROADMAP.md#akutundantag-snävsäkerhetsventil).

**SH6 = KLAR** (+ `vm_run_tag_array.kab` fast tag→family array, `sh6_vm_tag_array_smoke`). **SH17** i64-loop-subset stängt. **SH18** nursery på `new_instance`. **SH19** packed `.kbcb`. **SH20** JSON codec. **SH21** OS write/read. **SH22:** `sqlExecOk` (`sh22_sql_exec_smoke`). **SH23:** 🟡 Kab TLS application-data/AES-GCM unwrap med recordsäker raw-byte TCP-loopback + query Location (`sh23_crypto_http_fetch_eval_smoke`), verifierad typ-22 TCP-loopback för ClientHello/ServerHello/Certificate och X25519-ServerKeyExchange med längd-/algoritmkontroller och transcript (`sh23_crypto_tls_hs_eval_smoke`), deterministisk TLS 1.2 fixture-handshake (`sh23_crypto_tls_fixture_eval_smoke`) och strikt PEM/Base64→X.509-DER-gate för `TBSCertificate`/`signatureAlgorithm`/`signatureValue`, exakt `sha256WithRSAEncryption`-AlgorithmIdentifier, v3-prefix, issuer/subject-RDN och `rsaEncryption`-SPKI med kanoniska modulus/exponent. SHA-256/PKCS#1-v1.5 kontrolleras även med en avgränsad 512-bitars RSA/e=3-fixture plus signerad leaf←issuer-kedja (`sh23_crypto_root_eval_smoke`, `cryptoRootSignedChainEvalOk`). Live TLS 1.2 ECDHE-handskakning härleder premaster med X25519 från SKE+CKE och verifierar AES-GCM Finished (`sh23_crypto_tls_live_eval_smoke`). `httpsLoopFetch` / `tlsLiveGetEvalOk` gör GET application-data efter den handskakningen (`sh23_crypto_http_tls_eval_smoke`, `https://127.0.0.1:28199/v?q=1`; premaster `x25519` på SKE; ClientHello med X25519 `supported_groups`). Produkt-`httpFetch` dirigerar den URL:en till `httpsLoopFetch` (inte rustls). Kab-klient mot rustls TLS 1.2-peer: ServerHello 0xC02B genom encrypted Finished och GET `/v?q=1`; `httpFetch` mot `https://127.0.0.1:28291/v?q=1` via `import "kab/crypto/crypto_http_fetch_peer"` (inga default-parametrar; `sh23_crypto_tls_peer_get_eval_smoke`, `sh23_crypto_http_fetch_peer_eval_smoke`). rustls-host kvar. `httpFetch` mot samma loop-URL via `import "kab/crypto/crypto_http_fetch_loop"` (`sh23_crypto_http_fetch_loop_eval_smoke`). `httpFetch` dirigerar båda via `import "kab/crypto/crypto_http_fetch_route"` (`sh23_crypto_http_fetch_route_eval_smoke`). Wrapper-`httpFetch` är kopplad via `import "kab/crypto/crypto_http_fetch_bind"` (`sh23_crypto_http_fetch_bind_eval_smoke`). Cheap smokes importerar inte crypto. `await httpFetch` mot wrapper-URL:erna är grön via `promise_resolve` (`sh23_crypto_http_fetch_await_eval_smoke`). `http_status`/`http_body` på wrapper-Kab-TLS är gröna via `http_response` (`sh23_crypto_http_fetch_body_eval_smoke`). `http_header`/`http_headers` läser wire-headers från Kab-TLS (`sh23_crypto_http_fetch_hdr_eval_smoke`). Valfri sökväg på `:28199`/`:28291` går via Kab-TLS (`sh23_crypto_http_fetch_url_eval_smoke`). `localhost` är loopback-alias (`sh23_crypto_http_fetch_localhost_eval_smoke`). POST via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_post_eval_smoke`). PUT via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_put_eval_smoke`). PATCH via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_patch_eval_smoke`). DELETE via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_delete_eval_smoke`). HEAD via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_head_eval_smoke`). OPTIONS via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_options_eval_smoke`). TRACE via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_trace_eval_smoke`). CONNECT via Kab-TLS mot loop/peer med authority-form (`sh23_crypto_http_fetch_connect_eval_smoke`). GET med Authorization via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_auth_eval_smoke`). GET med Cookie via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_cookie_eval_smoke`). GET med Proxy-Authorization via Kab-TLS mot loop/peer (`sh23_crypto_http_fetch_proxy_eval_smoke`). Live TLS 1.3 ClientHello/ServerHello på loopback (`sh23_crypto_tls13_hs_eval_smoke`, `0x1301`/`0x0304`/X25519). TLS 1.3 HKDF + EncryptedExtensions/Finished på loopback (`sh23_crypto_tls13_fin_eval_smoke`). TLS 1.3 AES-GCM handshake-record 0x17 för EE/Finished (`sh23_crypto_tls13_gcm_eval_smoke`). TLS 1.3 application-data GET efter GCM-Finished (`sh23_crypto_tls13_app_eval_smoke`). Wrapper-`httpFetch` GET via Kab TLS 1.3 mot `:28198` (`sh23_crypto_http_fetch_tls13_eval_smoke`). TLS 1.3 `httpFetch` GET mot `localhost:28198` (`sh23_crypto_http_fetch_tls13_localhost_eval_smoke`). TLS 1.3 `httpFetch` GET annan sökväg på `:28198` (`sh23_crypto_http_fetch_tls13_url_eval_smoke`). TLS 1.3 `httpFetch` POST mot `:28198` (`sh23_crypto_http_fetch_tls13_post_eval_smoke`). TLS 1.3 `httpFetch` PUT mot `:28198` (`sh23_crypto_http_fetch_tls13_put_eval_smoke`). TLS 1.3 `httpFetch` PATCH mot `:28198` (`sh23_crypto_http_fetch_tls13_patch_eval_smoke`). TLS 1.3 `httpFetch` DELETE mot `:28198` (`sh23_crypto_http_fetch_tls13_delete_eval_smoke`). TLS 1.3 `httpFetch` HEAD mot `:28198` (`sh23_crypto_http_fetch_tls13_head_eval_smoke`). TLS 1.3 `httpFetch` OPTIONS mot `:28198` (`sh23_crypto_http_fetch_tls13_options_eval_smoke`). TLS 1.3 `httpFetch` TRACE mot `:28198` (`sh23_crypto_http_fetch_tls13_trace_eval_smoke`). TLS 1.3 `httpFetch` CONNECT mot `:28198` (`sh23_crypto_http_fetch_tls13_connect_eval_smoke`). TLS 1.3 `httpFetch` GET med Authorization mot `:28198` (`sh23_crypto_http_fetch_tls13_auth_eval_smoke`). TLS 1.3 `httpFetch` GET med Cookie mot `:28198` (`sh23_crypto_http_fetch_tls13_cookie_eval_smoke`). TLS 1.3 `httpFetch` GET med Proxy-Authorization mot `:28198` (`sh23_crypto_http_fetch_tls13_proxy_eval_smoke`). Kab TLS 1.3 ClientHello/ServerHello mot rustls-peer `:28293` (`sh23_crypto_tls13_peer_eval_smoke`). Kab TLS 1.3 EncryptedExtensions/Finished mot rustls-peer `:28294` (`sh23_crypto_tls13_peer_fin_eval_smoke`). Kab TLS 1.3 application-data GET mot rustls-peer `:28295` (`sh23_crypto_tls13_peer_get_eval_smoke`). Wrapper-`httpFetch` GET via Kab TLS 1.3 mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_eval_smoke`). TLS 1.3 `httpFetch` GET mot `localhost:28296` (`sh23_crypto_http_fetch_tls13_peer_localhost_eval_smoke`). TLS 1.3 `httpFetch` GET annan sökväg mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_url_eval_smoke`). TLS 1.3 `httpFetch` POST mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_post_eval_smoke`). TLS 1.3 `httpFetch` PUT mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_put_eval_smoke`). TLS 1.3 `httpFetch` PATCH mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_patch_eval_smoke`). TLS 1.3 `httpFetch` DELETE mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_delete_eval_smoke`). TLS 1.3 `httpFetch` HEAD mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_head_eval_smoke`). TLS 1.3 `httpFetch` OPTIONS mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_options_eval_smoke`). TLS 1.3 `httpFetch` TRACE mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_trace_eval_smoke`). TLS 1.3 `httpFetch` CONNECT mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_connect_eval_smoke`). TLS 1.3 `httpFetch` GET med Authorization mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_auth_eval_smoke`). TLS 1.3 `httpFetch` GET med Cookie mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_cookie_eval_smoke`). TLS 1.3 `httpFetch` GET med Proxy-Authorization mot rustls-peer `:28296` (`sh23_crypto_http_fetch_tls13_peer_proxy_eval_smoke`). Kab TLS 1.3 Certificate/CertificateVerify mot rustls-peer `:28297` (`sh23_crypto_tls13_peer_cv_eval_smoke`). Kab TLS 1.3 ECDSA-SHA256-verify av CertificateVerify mot rustls-peer `:28299` (`sh23_crypto_tls13_peer_ecdsa_eval_smoke`, scheme `0x0403`). Kab TLS 1.3 RSA-PSS SHA-256-verify av CertificateVerify mot rustls RSA-peer `:28300` (`sh23_crypto_tls13_peer_pss_eval_smoke`, scheme `0x0804`). Kab TLS 1.3 PKCS#1 v1.5 SHA-256-verify av leaf-TBS mot rustls RSA-peer `:28301` (`sh23_crypto_tls13_peer_tbs_eval_smoke`). Kab TLS 1.3 ECDSA-SHA256-verify av leaf-TBS mot rustls P-256-peer `:28302` (`sh23_crypto_tls13_peer_ecdsa_tbs_eval_smoke`). Kab TLS 1.3 leaf-SAN/iPAddress `127.0.0.1` mot rustls P-256-peer `:28303` (`sh23_crypto_tls13_peer_san_eval_smoke`). Kab TLS 1.3 leaf-validity `notBefore`/`notAfter` mot rustls P-256-peer `:28304` (`sh23_crypto_tls13_peer_time_eval_smoke`). Kab TLS 1.3 leaf-keyUsage `digitalSignature` mot rustls P-256-peer `:28305` (`sh23_crypto_tls13_peer_ku_eval_smoke`). Kab TLS 1.3 leaf-EKU `serverAuth` mot rustls P-256-peer `:28306` (`sh23_crypto_tls13_peer_eku_eval_smoke`). Kab TLS 1.3 leaf-basicConstraints `cA=FALSE` mot rustls P-256-peer `:28307` (`sh23_crypto_tls13_peer_bc_eval_smoke`). Kab TLS 1.3 leaf issuer=subject mot rustls P-256-peer `:28308` (`sh23_crypto_tls13_peer_iss_eval_smoke`). Kab TLS 1.3 leaf-serial mot rustls P-256-peer `:28309` (`sh23_crypto_tls13_peer_sn_eval_smoke`). Kab TLS 1.3 leaf-version v3 mot rustls P-256-peer `:28310` (`sh23_crypto_tls13_peer_ver_eval_smoke`). Kab TLS 1.3 leaf-SKI mot rustls P-256-peer `:28311` (`sh23_crypto_tls13_peer_ski_eval_smoke`). Kab TLS 1.3 leaf-AKI mot rustls P-256-peer `:28312` (`sh23_crypto_tls13_peer_aki_eval_smoke`). Kab TLS 1.3 leaf-SPKI `id-ecPublicKey` mot rustls P-256-peer `:28313` (`sh23_crypto_tls13_peer_spki_eval_smoke`). Kab TLS 1.3 leaf-SPKI `secp256r1` mot rustls P-256-peer `:28314` (`sh23_crypto_tls13_peer_p256_eval_smoke`). Kab TLS 1.3 leaf-SPKI uncompressed point mot rustls P-256-peer `:28315` (`sh23_crypto_tls13_peer_pt_eval_smoke`). Kab TLS 1.3 leaf-SPKI nonzero X/Y mot rustls P-256-peer `:28316` (`sh23_crypto_tls13_peer_xy_eval_smoke`). Kab TLS 1.3 leaf-CN mot rustls P-256-peer `:28317` (`sh23_crypto_tls13_peer_cn_eval_smoke`). Kab TLS 1.3 leaf-CN UTF8String mot rustls P-256-peer `:28318` (`sh23_crypto_tls13_peer_utf8_eval_smoke`). Kab TLS 1.3 SAN iPAddress 127.0.0.1 mot rustls P-256-peer `:28398` (`sh23_crypto_tls13_peer_lip_eval_smoke`). Kab TLS 1.3 SAN iPAddress not multicast mot rustls P-256-peer `:28399` (`sh23_crypto_tls13_peer_nmc_eval_smoke`). Kab TLS 1.3 SAN iPAddress not unspecified mot rustls P-256-peer `:28400` (`sh23_crypto_tls13_peer_nus_eval_smoke`). Kab TLS 1.3 SAN iPAddress not broadcast mot rustls P-256-peer `:28401` (`sh23_crypto_tls13_peer_nbc_eval_smoke`). Kab TLS 1.3 SAN iPAddress not link-local mot rustls P-256-peer `:28402` (`sh23_crypto_tls13_peer_nll_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC1918 10/8 mot rustls P-256-peer `:28403` (`sh23_crypto_tls13_peer_n10_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC1918 172.16/12 mot rustls P-256-peer `:28404` (`sh23_crypto_tls13_peer_n172_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC1918 192.168/16 mot rustls P-256-peer `:28405` (`sh23_crypto_tls13_peer_n192_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6598 100.64/10 mot rustls P-256-peer `:28406` (`sh23_crypto_tls13_peer_n100_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-1 192.0.2/24 mot rustls P-256-peer `:28407` (`sh23_crypto_tls13_peer_n202_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-2 198.51.100/24 mot rustls P-256-peer `:28408` (`sh23_crypto_tls13_peer_n198_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-3 203.0.113/24 mot rustls P-256-peer `:28409` (`sh23_crypto_tls13_peer_n203_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC2544 198.18/15 mot rustls P-256-peer `:28410` (`sh23_crypto_tls13_peer_n218_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 IETF 192.0.0/24 mot rustls P-256-peer `:28411` (`sh23_crypto_tls13_peer_n200_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC7526 192.88.99/24 mot rustls P-256-peer `:28412` (`sh23_crypto_tls13_peer_n288_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 reserved 240/4 mot rustls P-256-peer `:28413` (`sh23_crypto_tls13_peer_n240_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 this-network 0/8 mot rustls P-256-peer `:28414` (`sh23_crypto_tls13_peer_n008_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 AS112 192.31.196/24 mot rustls P-256-peer `:28415` (`sh23_crypto_tls13_peer_n231_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 AMT 192.52.193/24 mot rustls P-256-peer `:28426` (`sh23_crypto_tls13_peer_n252_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC7535 AS112 192.175.48/24 mot rustls P-256-peer `:28427` (`sh23_crypto_tls13_peer_n175_eval_smoke`). Kab TLS 1.3 SAN iPAddress not RFC6890 255/8 mot rustls P-256-peer `:28438` (`sh23_crypto_tls13_peer_n255_eval_smoke`). Kab TLS 1.3 SAN otherName (tag 160) rejected mot rustls P-256-peer `:28439` (`sh23_crypto_tls13_peer_n160_eval_smoke`). Kab TLS 1.3 SAN rfc822Name (tag 129) rejected mot rustls P-256-peer `:28440` (`sh23_crypto_tls13_peer_n129_eval_smoke`). Kab TLS 1.3 SAN dNSName (tag 130) rejected mot rustls P-256-peer `:28441` (`sh23_crypto_tls13_peer_n130_eval_smoke`). Kab TLS 1.3 SAN x400Address (tag 131) rejected mot rustls P-256-peer `:28442` (`sh23_crypto_tls13_peer_n131_eval_smoke`). Kab TLS 1.3 SAN directoryName (tag 132) rejected mot rustls P-256-peer `:28443` (`sh23_crypto_tls13_peer_n132_eval_smoke`). Kab TLS 1.3 SAN ediPartyName (tag 133) rejected mot rustls P-256-peer `:28444` (`sh23_crypto_tls13_peer_n133_eval_smoke`). Kab TLS 1.3 SAN uniformResourceIdentifier (tag 134) rejected mot rustls P-256-peer `:28445` (`sh23_crypto_tls13_peer_n134_eval_smoke`). Kab TLS 1.3 SAN registeredID (tag 136) rejected mot rustls P-256-peer `:28446` (`sh23_crypto_tls13_peer_n136_eval_smoke`). Kab TLS 1.3 SAN iPAddress IPv6 (16 byte) rejected mot rustls P-256-peer `:28447` (`sh23_crypto_tls13_peer_n16_eval_smoke`). Kab TLS 1.3 SAN iPAddress empty (len 0) rejected mot rustls P-256-peer `:28448` (`sh23_crypto_tls13_peer_n0_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 1 rejected mot rustls P-256-peer `:28449` (`sh23_crypto_tls13_peer_n1_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 2 rejected mot rustls P-256-peer `:28450` (`sh23_crypto_tls13_peer_n2_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 3 rejected mot rustls P-256-peer `:28451` (`sh23_crypto_tls13_peer_n3_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 5 rejected mot rustls P-256-peer `:28452` (`sh23_crypto_tls13_peer_n5_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 6 rejected mot rustls P-256-peer `:28453` (`sh23_crypto_tls13_peer_n6_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 7 rejected mot rustls P-256-peer `:28454` (`sh23_crypto_tls13_peer_n7_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 8 rejected mot rustls P-256-peer `:28455` (`sh23_crypto_tls13_peer_n8_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 9 rejected mot rustls P-256-peer `:28456` (`sh23_crypto_tls13_peer_n9_eval_smoke`). Kab TLS 1.3 SAN iPAddress length 11 rejected mot rustls P-256-peer `:28457` (`sh23_crypto_tls13_peer_n11_eval_smoke`). **Nästa:** Kab TLS 1.3 SAN iPAddress length 12 rejected mot rustls-peer. Inte fler crypto-flag-kloner eller sqlIs*-kloner.

## Kedja

```
source text
    → tokenizeExec / parseTokensExec   AST
    → emitMainExec                     opcode IR
    → serSerializeBc                   .kbc text
    → compile.kab                      source → .kbc
    → seed/compiler.kbcb               SH1 packed image
```

Default CLI: `kabootar compile` → self-host först. App-`.kab` har **ingen** Rust-fallback (SH16); `KABOOTAR_COMPILE=rust` / `--rust` **felar** för appar. `self_host/` DAG får rust-seeds. `bootPolicy("prefer")` = `self-host-only`.

## Nuläge (inte den gamla shard-listan)

| Yta | Status |
|-----|--------|
| Skip-list | **tom** (`attempt-all`, P6b) |
| Compile-DAG | **12** `.kab` (SH5 platå); `vm_*` **&lt; 40** (SH6) |
| Image | `self_host/seed/compiler.kbcb` + `seed/dag/*.kbc` (SH1) |
| Facader | `pub let` alias, inte wrapping `pub fn` (SH3b) |
| Lexer | **SH12:** `gLxSess` + in-place tokens; skip/ident/number cache `src`/`pos` |
| Parser/emit | **SH2/SH13:** återanvänd `gSess`/`gE` + `pResetSession`/`eResetSession`; tramp 0-arg. `pCondStack` på sess |
| **`struct` `&self`** | ✅ R4: `fn get(&self)` / `fn set(&mut self, n)` parse + compile-run (samma `self`-lokal som `fn sum(self)`) |
| **`match`** | ✅ produktkompilatorn: const/`_`/var/`Some`/`None`/`Ok`/`Err`/`NaN` + guards + **array/objekt** + **`...rest`** / **`[a, ...mid, b]`** + **or-mönster** (`1 \| 2`) + **range** (`1..5` / `1..=5` / **`..5` / `1..` / `..=5`**) + **`n @ pat`** + **`(pat)`** + **enum** (unit + payload-ctors) + **`if let`/`while let`**. Lexer: `..` / `..=` / `@`. Text-`.kbc` enum-sektion; host + Kab-VM |
| Dirty seeds | `compile_dirty_dag_seeds()` loggar `dirty=N` (SH7) |
| Produktträd | `compile_dirty_product_tree(entry)` (SH7b) |
| Tiny parse | `sh8_tiny_parse_via_compiler_image` i CI; full `compile("return 1")` ignored i debug |
| Cache | SH15 content-addressed `cache/ca/v{image}_{fp}.kbcb` + mmap |

Tunga `_*probe*` / `_bisect*` är **inte** produkt. Regenerera image: `KABOOTAR_SH1_WARM=1 cargo test --test sh_wave sh1_warm -- --ignored`.

## Filer (ingångar)

| Fil | Roll |
|-----|------|
| `compile.kab` | `compile(source)` / `compileIr` |
| `parse.kab` | `parse` = tokenizeExec + parseTokensExec |
| `lexer.kab` / `parser.kab` / `emit.kab` / `serialize.kab` | tunna `pub let`-facader |
| `parser_exec.kab` / `emit_exec.kab` | per-call session + tramp |
| `ownership.kab` | O5 `@manual` |
| `vm.kab` | kab-only VM (alias till `vm_run_exec_core`) |
| `deserialize.kab` / `deserialize_kbcb.kab` | text `.kbc` / packed kbcb v2 (Number array eller Uint8Array) → `runModule` IR |
| `seed/compiler.kbcb` | packed compile-DAG |

## Tester

```bash
cargo test --test sh_wave -- --test-threads=1
cargo test --test self_host self_host_parser_suite -- --test-threads=1
cargo test --test self_host self_host_if_let_some_compile_run -- --test-threads=1
cargo test --test self_host self_host_while_let_ok_compile_run -- --test-threads=1
cargo test --test self_host self_host_match_enum_pattern_compile_run -- --test-threads=1
cargo test --test self_host self_host_match_enum_payload -- --test-threads=1
cargo test --test self_host self_host_match_array_rest -- --test-threads=1
cargo test --test self_host self_host_match_object_rest -- --test-threads=1
cargo test --test self_host self_host_match_array_mid_rest -- --test-threads=1
cargo test --test self_host self_host_match_or_pattern -- --test-threads=1
cargo test --test self_host self_host_if_let_or -- --test-threads=1
cargo test --test self_host self_host_match_range -- --test-threads=1
cargo test --test self_host self_host_match_open_range -- --test-threads=1
cargo test --test self_host self_host_match_at_bind -- --test-threads=1
cargo test --test self_host self_host_if_let_at_bind -- --test-threads=1
cargo test --test self_host self_host_match_array_at_rest -- --test-threads=1
cargo test --test self_host self_host_match_ok_nested_at -- --test-threads=1
cargo test --test self_host self_host_while_let_at_ok -- --test-threads=1
cargo test --test self_host self_host_match_object_field_at -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_array -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_object -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_guard -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_object_rest -- --test-threads=1
cargo test --test self_host self_host_struct_ref_self -- --test-threads=1
cargo test --test self_host self_host_class_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_init_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_member_assign_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_bound_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_compound_assign_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_member_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_mixed_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_super_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_super_callback_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_generic_super_method_ok_kab_only -- --test-threads=1
cargo test --test sh_wave sh6_default_eval -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_in_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_await_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_array_slice_from_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_new_instance_from_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_array_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_object_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_array_literal_spread_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_literal_spread_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_is_is_not_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_shorthand_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_computed_key_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_fn_default_param_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_fn_rest_param_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_arrow_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_class_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_iface_default_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_optional_chain_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_switch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_do_while_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_switch_fallthrough_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_result_question_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_result_question_err_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_ternary_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_import_meta_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_template_literal_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_classic_for_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_in_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_const_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_array_object_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_while_let_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_mid_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_guard_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_at_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_field_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_elem_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_payload_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_while_let_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_paren_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_option_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_float_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_result_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_at_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_payload_at_bind_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_option_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_struct_box_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_struct_box_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_method_echo_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_method_echo_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_box_explicit_string_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_class_extends_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_explicit_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_pair_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_box_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_pair_from_lets_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_len_wrap_call_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_init_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_count_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_n_add_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bound_tag_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_as_callback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_trait_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_class_assoc_item_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_struct_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_struct_show_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_id_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_generic_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_generic_show_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_while_let_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_open_to_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_while_let_open_to_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_is_class_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_pass_assert_not_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_raise_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mixed_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_nullish_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mixed_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_bitwise_or_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mul_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_mul_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_div_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_div_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_compound_assign_eval_once_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_method_this_writeback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_using_class_close_writeback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_with_bind_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_bound_tag_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_dynamic_import_math_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_async_fn_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_await_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_break_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_continue_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_generator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_method_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_next_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_expr_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_throw_outer_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_array_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_array_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_custom_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_custom_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_custom_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_custom_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_custom_return_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_symbol_iterator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_symbol_iterator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_custom_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_symbol_iterator_wellknown_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_obj_method_and_generator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_obj_method_iter_this_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_obj_method_this_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_nested_obj_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_obj_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_return_this_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_throw_this_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_throw_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_throw_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_return_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_try_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_return_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_member_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_send_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_inner_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_inner_send_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_yield_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_flag_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_again_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_null_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_method_return_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_method_return_null_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_null_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_into_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_throw_into_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_throw_into_try_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_return_into_try_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_return_into_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_outer_finally_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_outer_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_return_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_throw_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_throw_inner_finally_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_return_inner_finally_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_outer_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_inner_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_inner_try_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_try_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_try_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_try_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_finally_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_finally_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_inner_finally_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_outer_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_throw_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_return_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_nested_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_nested_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_nested_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_nested_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_outer_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_throw_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_array_return_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_outer_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_method_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_method_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_send_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_outer_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_into_outer_finally_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_method_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_method_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_rethrows_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_rethrows_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_throws_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_throws_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_outer_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_outer_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_outer_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_outer_finally_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_outer_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_inner_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_inner_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_inner_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_inner_finally_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_inner_finally_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_inner_finally_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_inner_finally_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_inner_finally_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_inner_finally_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_inner_finally_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_inner_finally_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_inner_finally_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_inner_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_inner_finally_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_inner_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_inner_finally_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_inner_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_inner_finally_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_inner_finally_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_inner_finally_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_false_continue_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_false_continue_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_false_continue_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_false_continue_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_return_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_return_method_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_return_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_return_method_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_return_method_next6_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_return_method_next6_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_false_continue_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_return_done_false_continue_send_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_false_continue_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_done_false_continue_send_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_throw_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_next6_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_next6_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_next7_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_next7_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_throw_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_throw_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_return_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_return_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_throw_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_throw_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_return_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_return_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_throw_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_throw_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_return_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_return_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_return_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_return_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_throw_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_throw_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_next_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_next_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_next_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_custom_throw_done_false_continue_send_return_method_send_done_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_done_false_continue_send_return_method_send_done_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_try_finally_no_catch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_return_finally_no_catch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_finally_no_catch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_throw_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_send_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_nested_expr_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_oversize_string_const_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_loop_100_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_unrolled_200_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_file_mmap_eval_ok -- --test-threads=1
cargo test --test sh_wave sh6_eval_file_cached_mmap_ok -- --test-threads=1
cargo test --test sh_wave sh6_kab_only_eval_ok -- --test-threads=1
cargo test --test sh_wave sh6_eval_file_cached_kbcb_image_ok -- --test-threads=1
cargo test --test self_host self_host_generic_struct_box -- --test-threads=1
cargo test --test self_host self_host_generic_method_on_specialized -- --test-threads=1
cargo test --test self_host self_host_generic_enum -- --test-threads=1
cargo test --test self_host self_host_trait_default_method -- --test-threads=1
cargo test --test self_host self_host_class_assoc_type -- --test-threads=1
cargo test --test self_host self_host_generic_trait -- --test-threads=1
cargo test --test self_host self_host_where_bound -- --test-threads=1
cargo test --test self_host self_host_where_method_bound -- --test-threads=1
cargo test --test self_host self_host_where_class_bound -- --test-threads=1
kabootar self_host/test_tiny.kab
kabootar compile self_host/sample.kab
```

## Designregler

**SH2:** parser/emit-cursors (`pPos`, `eOps`, …) ligger på **session-objektet**, inte nya modul-globaler. Trampolin: `sess["tramp"](sess)` så rekursion inte fångar en modul-`sess`. Nested `if`/`while` använder `pCondStack` / `eIfJmpStack` **på sess**. Nested named `fn` i en funktion: `emitNestedNamedFn` (save/restore `eFnOps`, `MakeArrow` + lokal).

1. **Fn-lokaler** — bytecode speglar lokaler på *aktuell* aktiveringsram. Captures: `local_captures`. **Lexer-ident:** `let cd`/`ok`/`start` i samma fn som loopen (`lxScanIdent`) — saknad `let` blir bytecode-global (`Undefined variable: cd`).
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`.
4. **Bracket-access för AST-nycklar** — `node["sym"]` där `.then`/`.value` krockar.
5. **Radbrytning** — använd `CHAR_NL`, inte `"\n"` i serializer (SH3c).
6. **Assign: peek före bump** — `let tok = peek(); bump();`.
7. **Modulskala (L2)** — ≥40 top-level `fn` per modul OK. Densify till 5-radersfiler är **föråldrat** (ökar import-evals).
8. **Exporterade fn** — `pub let X = Ximpl` på facader (SH3b); wrapping `pub fn` ger extra Kab-VM-ram.
9. **Nested import** — `import "self_host/compile"` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` tillsammans med `parse.kab`.
10. **CALL-args** — undvik var+literal i samma 2-arg `CALL` i heta fn.
11. **Windows stack** — `build.rs` sätter 16 MiB för `kabootar`-bin.
12–51. Nested if/while/call/clobber-workarounds (`pCondStack`, `eIfJmpStack`, `eCalleeStack`, …) är **session-fält**, inte nya modul-globaler. Full lista historiskt nedan; nya `let pSave*` / `let pPos` i facader är **förbjudna** (SH10).

## Historiska clobber-regler (session-fält, inte nya globals)

12. **Compare-parse** — spara lhs före rhs; inte `parsePostfix()` för compare-rhs.
13. **Emitter while** — loop-head i `eWhileHead`; jump-args relativa: `target - jmpIndex - 1`.
14. **Bytecode-cache** — `.kabootar/cache/*.kbc` + fingerprint (content + import-mtimes). SH7b kompilerar bara dirty.
15. **Serialize från `.kbc`** — `CHAR_NL`; inte privata fn från exporterade `serialize_bc`.
16. **Array literal** — `AST_ARRAY` + `make_array`.
17. **Emitter scratch** — `eBxL`/`eBxR` / `eList` / `eBodyStmts` på sess.
18. **Throw** — `AST_THROW` + `throw` opcode.
19. **Nested if/while** — `eIfJmpStack`/`eIfSkipStack`; trimma `eBreakIdxs` efter inner loop.
20. **Parser sym snapshot** — `pFnSym`/`pFnPub` på sess före rekursiv `parseStmt`.
21. **while/if cond** — `pCondStack` på sess.
22. **let/assign sym** — `pBindSym` (inte `pSaveSym`).
23. **assign lookahead** — `pNextTok = pToks[pPos+1]` med EOF-fallback.
24. **bracket index** — fn-lokal `indexObj`.
25. **compare rhs** — `pInAddSub`.
26. **`+`/`-` rhs** — `pAddLeftStack` + rekursiv `parseCompare`.
27. **&& expr** — `pExprLeft`.
28. **binary op** — fn-lokaler `binOp`/`binRight`.
29. **pub exports** — `isPub` / `eExports` / `exports=`.
30. **let/member** — `eStoreSym` / `eMemberFldStack` / `eAssignSym` / `eExprStmt`.
31. **module globals in fn** — `eFnLocals` först, sedan `eGlobals`.
32. **fn snapshot** — `snapArr(eFnOps)` vid push till `eFunctions`.
33. **block loop** — `eBlockIStack`/`eBlockNStack`.
34. **expr-loops** — `eObjIStack`, `eArrIStack`, `eCallArgIStack`.
35. **parseTokens EOF** — `while pDone == 0`.
36. **binary `+` i fn** — spara rhs före `emitExpr(left)`.
37. **let sym** — `pLetSym` före `bump()`.
38. **undefined literal** — `TOKEN_UNDEFINED` → `LIT_UNDEF`.
39. **postfix chains** — interleaved `()`, `.`, `[]`.
40. **`null` vs `undefined`** — `null == undefined` är `false`.
41. **Program body** — block-stack + `OP_HALT`.
42. **index assign** — `AST_INDEX_ASSIGN` + `OP_INDEX_SET`.
43. **emit index assign** — spara `eBxRhs` före `emitExpr`.
44. **popStack** — native `pop(stack)`.
45. **import emit vs compile(emit.kab)** — häng via `.kbc` → serialize/compile, inte bara emit-logik.
46. **SH3a** — nested call args on locals (`callArgs`), not `sess["pArgs"]`; `len(expr)` → `get_length`; argv N-path. Gate: `self_host_len_of_call_expr_*` / `self_host_emit_nested_call_argn_*`.
47. **CHAR_NL** i serialize (SH3c).
48. **nested call emit** — `eCalleeStack`.
49. **nested call parse** — fn-lokaler `savedCallee`/`savedTypeArgs`.
50. **generic call type args** — `savedTypeArgs` med call.
51. **generic emit** — `eGenericTemplates`; ingen extra import från `emit.kab`.

## Nästa milstolpar (Våg SH)

Historiska 1–14 (roundtrip, facader, bootstrap, generics) är klara. **Inte nästa:** fler `_probe`-filer.

**~~SH17–SH27~~ ✅ subset** nedan är **flag-gates i `.kab`**, inte att Rust är raderad. Produktmålet är [noll Rust / SH28](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust): `src/` finns, rustc krävs, **SH28 är inte stängd**.

Kort ordning:

1. ~~SH0/SH1~~ ✅ · **SH2** nested named `fn` + sess ✅
2. ~~SH3–SH7b~~ ✅ · ~~SH5 densify~~ ✅ (serialize_sections+out+ir_line+acc, parser_expr→exec, parser_hooks/lexer_defs/emit_defs→ast_defs, lexer_tokenize→scan, emit_fn_scope/hooks/arr_util→sym, emit_sym_index→sym, emit_tramp/main_fn→exec, parser_main/tramp/type_args/session→exec, parser_block→hooks)
3. ~~**SH16**~~ ✅ appar: ingen rust-emit (`eval_file_cached` / `compile --rust`); toolchain `self_host/` får rust
4. **SH5 platå** — compile-DAG **12**; `ownership` får **inte** `pub import compile` (suiten laddar hela pipelinen). Inte `parser_stmt`/`postfix`/`emit_*_body` förrän leaf ≤10 s / ~550 rader. **`match`**: enum unit + payload + `if let`/`while let`.
5. ~~**SH17/SH18**~~ ✅ subset + deepen (`jitMmapOk` mmap/exec dual-bind; loop8/loopN/arith-imm/bit-ops/shifts/unary/eq/ne/lt/gt/le/ge/test/je/jne/jmp/jl/jle/jg/jge/nop `os_mm_call`; `gcHostDeleteOk` host-GC dual-bind)
6. ~~**SH19**~~ ✅ subset + deepen (`loadEvalKbcbRoundtripOk` packed `.kbcb` v2 eval; `loadEvalKabRoundtripOk` `.kab`-källa via `load_src.kab`; `loadArgvKind` argv-dispatch via SH25-gates i `load_argv.kab`; `loadKbcbHeaderOk`/`loadKbcbFixtureOk` i `load_validate.kab`; `loadMainDeleteOk` still false)
7. ~~**SH20**~~ ✅ subset (JSON/datum/regex + math + objekt + collections + colget + collen + colpush + colpop + colfirst + collast + colrest + colempty + colconcat + colrev + colcontains + colindex + colcount + coltake + coldrop + colzip + colunzip + colflat + colunique + coleq + colclone + colrepeat + colfill + colrange + colsum + colmax + colmin + colprod + colavg + colmed + colmode + colsort + coldesc + colfind + colrfind + colrix + colslice + colwin + colchunk + colrot + colpad + colilv + coltr + coldiag + colident + coltrc + colrow + colcol + colshape + colrshp + coldot + colmv + colmm + colout + colcrs + coldet + colnorm + colunit + colproj + colrej + coldist + collerp + colscale + coladd + colsub + colmul + coldiv + colneg + colabs + colsign + colclamp + colmod + colpow + colsqrt + colsqr + colcub + colfloor + colceil + colround + coltrunc + collog + collog2 + collog10 + colexp + colsin + colcos + coltan + colasin + colacos + colatan + colatan2 + colhypot + colcbrt + colimul + colclz32 + colfround + colf16round + colsumprec + collog1p + colexpm1 + colsinh + colcosh + coltanh + colasinh + colacosh + colatanh + colfmod + colrandom + colpi + cole + colln2 + colln10 + collog2e + collog10e + colsqrt2 + colsqrt12 leaves); radera natives deepen
8. ~~**SH21**~~ ✅ subset (`kabOsIsFile` + `kabOsArgvOk` + `kabOsEnvOk` + `kabOsCwdOk` + `kabOsIsDir` + `kabOsJoin` + `kabOsBase` + `kabOsExt` + `kabOsDirname` + `kabOsNorm` + `kabOsAbs` + `kabOsRel`); radera `runtime/os` deepen
9. ~~**SH22**~~ ✅ subset (`sqlIsWhere` + `sqlStoreOk` + `sqlIsLimit` + `sqlIsOrder` + `sqlIsInsert` + `sqlIsUpdate` + `sqlIsDelete` + `sqlIsCreate` + `sqlIsJoin` + `sqlIsGroup` + `sqlIsHaving` + `sqlIsDistinct` + `sqlIsUnion`); radera `src/sql` deepen
10. **SH23** ✅ Kab TLS 1.3 SAN iPAddress length 11–15 rejected rustls P-256-peer `:28457` (`crypto_tls13_peer_n11..n15.kab`); längdsvepning 0–20 via `crypto_tls13_peer_nlen.kab`; leaf `validity` notBefore/notAfter (`crypto_tls13_peer_dates.kab`); chain-count (`crypto_tls13_peer_chain.kab`), chain-all (`crypto_tls13_peer_chainall.kab`), serialNumber (`crypto_tls13_peer_serial.kab`), signatureAlgorithm (`crypto_tls13_peer_sigalg.kab`), cert-pin (`crypto_tls13_peer_pin.kab`), sammansatt trust-gate (`crypto_tls13_peer_trust.kab`), X.509-struktur (`crypto_tls13_peer_x509.kab`), AIA/OCSP (`crypto_tls13_peer_ocsp.kab`), full handshake (`crypto_tls13_peer_full.kab`), TLS 1.2 high-level client (`crypto_tls12_client.kab`/`crypto_tls12_client_fetch.kab`), TLS 1.3 high-level client (`crypto_tls13_client.kab`/`crypto_tls13_client_get.kab`/`crypto_tls13_client_close.kab`/`crypto_tls13_client_fetch.kab`/`crypto_tls13_client_reconnect.kab`/`crypto_tls13_client_config.kab`/`crypto_tls13_client_post.kab`/`crypto_tls13_client_timeout.kab`/`crypto_tls13_client_generic.kab`/`crypto_tls13_all.kab`) och scheme-match (`crypto_tls13_peer_scheme.kab`) och CRLDP (`crypto_tls13_peer_crl.kab`) och alert (`crypto_tls13_peer_alert.kab`) och certreq (`crypto_tls13_peer_certreq.kab`) och resume (`crypto_tls13_peer_resume.kab`) och no-rustls loopback (`crypto_tls13_client_loop.kab`); `httpFetch` kan nå rustls-peer :28296 via Kab-klient (`crypto_tls13_client_fetch.kab`/`crypto_tls13_client_generic.kab`); length-4-adressvalidering via `crypto_tls13_peer_n4.kab`; generaliserad SAN-validator (`crypto_tls13_peer_san_gen.kab`) + konsoliderad ext-scanner (`crypto_tls13_hs_opt.kab`) minskar duplicering; `cryptoHostDeleteOk` drivs av `crypto_tls13_all.kab`. **Nästa:** [Just nu](../docs/ROADMAP.md#just-nu).
11. ~~**SH24**~~ ✅ subset (Kab-VM owner-scheduler: `await httpFetch`, `await_all`, timeout, OS-read, blandad I/O och importerad `httpFetch` över lokalt betrodd HTTPS med godkänd/avvisad pin, trust-reset, absoluta/relativa redirects, bevarade credentials same-origin och borttagna Authorization/Cookie/Proxy-Authorization cross-origin, POST→301/302/303→GET, POST→307/308→POST med body och rejection för 3xx utan Location eller över redirectgränsen på 10 hopp, `sh6_http_fetch_wrapper_local_await_smoke`, `sh6_http_fetch_wrapper_local_await_all_smoke`, `sh6_http_fetch_wrapper_local_timeout_smoke`, `sh6_os_read_async_owner_scheduler_smoke`, `sh6_mixed_io_owner_scheduler_smoke`, `kab_vm_imported_http_fetch_uses_outer_tls_trust`, `kab_vm_imported_http_fetch_rejects_wrong_tls_pin`, `kab_vm_tls_reset_restores_default_trust`, `kab_vm_imported_http_fetch_follows_tls_redirect_with_ca_and_pin`, `kab_vm_imported_http_fetch_preserves_headers_across_tls_redirect`, `kab_vm_imported_http_fetch_follows_relative_tls_redirect_with_ca_and_pin`, `kab_vm_imported_http_fetch_rejects_redirect_without_location`, `kab_vm_imported_http_fetch_rejects_too_many_redirects`, `kab_vm_imported_http_fetch_post_301_redirect_changes_to_get`, `kab_vm_imported_http_fetch_post_302_redirect_changes_to_get`, `kab_vm_imported_http_fetch_post_303_redirect_changes_to_get`, `kab_vm_imported_http_fetch_post_307_redirect_preserves_method_and_body`, `kab_vm_imported_http_fetch_post_308_redirect_preserves_method_and_body`, `cross_origin_redirect_removes_credentials`, `same_origin_redirect_keeps_credentials` + `httpIsPost` + `httpIsJson` + `httpIsPut` + `httpIsPatch` + `httpIsHead` + `httpIsDelete` + `httpIsOptions` + `httpIsTrace` + `httpIsConnect`); TCP/rustls host kvar tills Kab kan äga handskakningen
12. ~~**SH25**~~ ✅ subset (`cliIsCompile` + `cliIsFmt` + `cliIsCheck` + `cliIsLint` + `cliIsVersion` + `cliIsHelp` + `cliIsDoc` + `cliIsBench` + `cliIsNew` + `cliIsInit` + `cliIsWatch` + `cliIsClean` + `cliIsAdd` + `cliIsRm` + `cliIsMod` + `cliIsLs` + `cliIsCat` + `cliIsServe` + `cliIsShell` + `cliIsNotebook`/`nb` + `cliIsRegistry` + `cliIsInstall` + `cliIsPublish` + `cliIsVersionFlag`/`cliIsHelpFlag` + `cliIsRunPath`/`cliIsNbPath` + `cliIsModInit` + `cliIsModRun`; kabtest KT8 `ktCliIsCoverage` + `ktCliHostDeleteOk`); radera `src/cli` deepen
13. ~~**SH26**~~ ✅ subset (`sciNdLenOk` + `sciFftPow2` + `sciSub` + `sciDiv` + `sciNeg` + `sciAbs` + `sciMax` + `sciMin` + `sciClamp` + `sciPow` + `sciSqr` + `sciCub` + `sciSign`); GPU kernel deepen
14. ~~**SH27**~~ ✅ subset (`uiIsCanvas` + `uiFpsOk` + `uiIsSpan` + `uiIsButton` + `uiIsInput` + `uiIsImg` + `uiIsP` + `uiIsA` + `uiIsUl` + `uiIsLi` + `uiIsOl` + `uiIsH1` + `uiIsH2` + `uiIsH3` + `uiIsH4` + `uiIsH5` + `uiIsH6` + `uiIsForm` + `uiIsLabel` + `uiIsTextarea` + `uiIsSelect` + `uiIsOption` + `uiIsTable` + `uiIsTr` + `uiIsTh` + `uiIsTd` + `uiIsThead` + `uiIsTbody` + `uiIsTfoot` + `uiIsNav` + `uiIsHeader` + `uiIsFooter` + `uiIsMain` + `uiIsSection` + `uiIsArticle` + `uiIsAside` + `uiIsFigure` + `uiIsFigcaption` + `uiIsDetails` + `uiIsSummary` + `uiIsDialog` + `uiIsPre` + `uiIsCode` + `uiIsBlockquote` + `uiIsVideo` + `uiIsAudio` + `uiIsSource` + `uiIsTrack` + `uiIsIframe` + `uiIsFieldset` + `uiIsLegend` + `uiIsHr` + `uiIsBr` + `uiIsKbd` + `uiIsSamp` + `uiIsVar` + `uiIsAbbr` + `uiIsCite` + `uiIsMark` + `uiIsSmall` + `uiIsStrong` + `uiIsEm` + `uiIsSub` + `uiIsSup` + `uiIsTime` + `uiIsQ` + `uiIsB` + `uiIsI` + `uiIsU` + `uiIsS` + `uiIsDel` + `uiIsIns` + `uiIsWbr` + `uiIsRuby` + `uiIsRt` + `uiIsRp` + `uiIsBdi` + `uiIsBdo` + `uiIsData` + `uiIsDfn` + `uiIsMeter` + `uiIsProgress` + `uiIsOutput` + `uiIsDatalist` + `uiIsOptgroup` + `uiIsPicture` + `uiIsMap` + `uiIsArea` + `uiIsEmbed` + `uiIsObject` + `uiIsParam` + `uiIsColgroup` + `uiIsCol` + `uiIsCaption` + `uiIsTemplate` + `uiIsSlot` + `uiIsNoscript` + `uiIsScript` + `uiIsStyle` + `uiIsLink` + `uiIsMeta` + `uiIsTitle` + `uiIsBase` + `uiIsHead` + `uiIsBody` + `uiIsHtml` + `uiIsHgroup` + `uiIsAddress` + `uiIsDl` + `uiIsDt` + `uiIsDd` + `uiIsMenu` + `uiIsSearch` + `uiIsPortal` + `uiIsSvg` + `uiIsMath` + `uiIsSelectedcontent` + `uiIsFencedframe` + `uiIsFrameset` + `uiIsFrame` + `uiIsNoframes` + `uiIsMarquee` + `uiIsFont` + `uiIsCenter` + `uiIsNobr` + `uiIsDir` + `uiIsBlink` + `uiIsApplet` + `uiIsBasefont` + `uiIsIsindex` + `uiIsKeygen` + `uiIsListing` + `uiIsXmp` + `uiIsPlaintext` + `uiIsMenuitem` + `uiIsNoembed` + `uiIsSpacer` + `uiIsBgsound` + `uiIsAcronym` + `uiIsBig` + `uiIsTt` + `uiIsStrike` + `uiIsRb` + `uiIsRtc` + `uiIsRbc` + `uiIsShadow` + `uiIsContent` + `uiIsElement` + `uiIsNextid` + `uiIsLayer` + `uiIsIlayer` + `uiIsNolayer` + `uiIsMulticol` + `uiIsComment` + `uiIsXml` + `uiIsImage` + `uiIsServer` + `uiIsDiv` + `uiIsRect` + `uiIsCircle` + `uiIsEllipse` + `uiIsLine` + `uiIsPolyline` + `uiIsPolygon` + `uiIsPath` + `uiIsG` + `uiIsUse` + `uiIsDefs` + `uiIsSymbol` + `uiIsMarker` + `uiIsClipPath` + `uiIsMask` + `uiIsPattern` + `uiIsLinearGradient` + `uiIsRadialGradient` + `uiIsStop` + `uiIsText` + `uiIsTspan` + `uiIsTextPath` + `uiIsForeignObject` + `uiIsSwitch` + `uiIsFilter` + `uiIsFeGaussianBlur` + `uiIsFeBlend` + `uiIsFeColorMatrix` + `uiIsFeComponentTransfer` + `uiIsFeComposite` + `uiIsFeConvolveMatrix` + `uiIsFeDiffuseLighting` + `uiIsFeDisplacementMap` + `uiIsFeFlood` + `uiIsFeFuncA` + `uiIsFeFuncB` + `uiIsFeFuncG` + `uiIsFeFuncR` + `uiIsFeImage` + `uiIsFeMerge` + `uiIsFeMergeNode` + `uiIsFeMorphology` + `uiIsFeOffset` + `uiIsFePointLight` + `uiIsFeSpecularLighting` + `uiIsFeSpotLight` + `uiIsFeTile` + `uiIsFeTurbulence` + `uiIsFeDistantLight` + `uiIsFeDropShadow` + `uiIsAnimate` + `uiIsAnimateMotion` + `uiIsAnimateTransform` + `uiIsSet` + `uiIsMpath` + `uiIsView` + `uiIsMetadata` + `uiIsDesc` + `uiIsHatch` + `uiIsHatchpath` + `uiIsSolidcolor` + `uiIsCursor` + `uiIsTref` + `uiIsAltGlyph` + `uiIsAltGlyphDef` + `uiIsAltGlyphItem` + `uiIsGlyphRef` + `uiIsGlyph` + `uiIsMissingGlyph` + `uiIsFontFace` + `uiIsFontFaceSrc` + `uiIsFontFaceUri` + `uiIsFontFaceFormat` + `uiIsFontFaceName` + `uiIsHkern` + `uiIsVkern` + `uiIsMeshgradient` + `uiIsMeshrow` + `uiIsMeshpatch` + `uiIsDiscard` + `uiIsUnknown` + `uiIsMrow` + `uiIsMi` + `uiIsMn` + `uiIsMo` + `uiIsMtext` + `uiIsMs` + `uiIsMspace` + `uiIsMfrac` + `uiIsMsqrt` + `uiIsMroot` + `uiIsMsub` + `uiIsMsup` + `uiIsMsubsup` + `uiIsMunder` + `uiIsMover` + `uiIsMunderover` + `uiIsMmultiscripts` + `uiIsMprescripts` + `uiIsNone` + `uiIsMtable` + `uiIsMtr` + `uiIsMtd` + `uiIsMth` + `uiIsMlabeledtr` + `uiIsMaligngroup` + `uiIsMalignmark` + `uiIsMstyle` + `uiIsMerror` + `uiIsMpadded` + `uiIsMphantom` + `uiIsMfenced` + `uiIsMenclose` + `uiIsSemantics` + `uiIsAnnotation` + `uiIsAnnotationXml` + `uiIsMaction` + `uiIsMlongdiv` + `uiIsMstack` + `uiIsMsrow` + `uiIsMscarries` + `uiIsMscarry` + `uiIsMsline` + `uiIsMglyph` + `uiIsCi` + `uiIsCn` + `uiIsCsymbol` + `uiIsApply` + `uiIsBind` + `uiIsBvar` + `uiIsShare` + `uiIsCondition` + `uiIsPiecewise` + `uiIsPiece` + `uiIsOtherwise` + `uiIsLambda` + `uiIsReln` + `uiIsFn` + `uiIsInterval` + `uiIsList` + `uiIsVector` + `uiIsMatrix` + `uiIsMatrixrow` + `uiIsSelector` + `uiIsDomain` + `uiIsCodomain` + `uiIsDomainof` + `uiIsIdent` + `uiIsCompose` + `uiIsInverse` + `uiIsPlus` + `uiIsMinus` + `uiIsTimes` + `uiIsDivide` + `uiIsPower` + `uiIsRoot` + `uiIsGcd` + `uiIsAnd` + `uiIsOr` + `uiIsXor` + `uiIsNot` + `uiIsImplies` + `uiIsForall` + `uiIsExists` + `uiIsEquivalent` + `uiIsApprox` + `uiIsFactorof` + `uiIsTendsto` + `uiIsInt` + `uiIsDiff` + `uiIsPartialdiff` + `uiIsLowlimit` + `uiIsUplimit` + `uiIsDegree` + `uiIsLogbase` + `uiIsLog` + `uiIsLn` + `uiIsExp` + `uiIsSin` + `uiIsCos` + `uiIsTan` + `uiIsSec` + `uiIsCsc` + `uiIsCot` + `uiIsSinh` + `uiIsCosh` + `uiIsTanh` + `uiIsSech` + `uiIsCsch` + `uiIsCoth` + `uiIsArcsin` + `uiIsArccos` + `uiIsArctan` + `uiIsArccosh` + `uiIsArccot` + `uiIsArccoth` + `uiIsArccsc` + `uiIsArccsch` + `uiIsArcsec` + `uiIsArcsech` + `uiIsArcsinh` + `uiIsArctanh` + `uiIsAbs` + `uiIsConjugate` + `uiIsArg` + `uiIsReal` + `uiIsImaginary` + `uiIsFloor` + `uiIsCeiling` + `uiIsMin` + `uiIsMax` + `uiIsLcm` + `uiIsMean` + `uiIsSdev` + `uiIsVariance` + `uiIsMedian` + `uiIsMode` + `uiIsMoment` + `uiIsMomentabout` + `uiIsCartesianproduct` + `uiIsVectorproduct` + `uiIsScalarproduct` + `uiIsOuterproduct` + `uiIsTranspose` + `uiIsDeterminant` + `uiIsUnion` + `uiIsIntersect` + `uiIsIn` + `uiIsNotin` + `uiIsSubset` + `uiIsPrsubset` + `uiIsNotsubset` + `uiIsNotprsubset` + `uiIsSetdiff` + `uiIsCard` + `uiIsSum` + `uiIsProduct` + `uiIsLimit` + `uiIsCurl` + `uiIsDivergence` + `uiIsGrad` + `uiIsLaplacian` + `uiIsEmptyset` + `uiIsIntegers` + `uiIsRationals` + `uiIsReals` + `uiIsComplexes` + `uiIsNaturalnumbers` + `uiIsPrimes` + `uiIsExponentiale` + `uiIsImaginaryi` + `uiIsPi` + `uiIsEulergamma` + `uiIsInfinity` + `uiIsNotanumber` + `uiIsTrue` + `uiIsFalse` + `uiIsDomainofapplication` + `uiIsSep` + `uiIsDeclare` + `uiIsCerror` + `uiIsCs` + `uiIsCbytes` + `uiIsMsgroup` + `uiIsNeq` + `uiIsLt` + `uiIsGt` + `uiIsLeq` + `uiIsGeq` + `uiIsRem` + `uiIsQuotient` + `uiIsFactorial` + `uiIsEq` + `uiIsAnimateColor` + `uiIsColorProfile` + `uiIsDefinitionSrc` + `uiIsPrefetch` + `uiIsHandler` + `uiIsListener` + `uiIsAnimation` + `uiIsTbreak` + `uiIsTextArea` + `uiIsFlowRoot` + `uiIsFlowRegion` + `uiIsFlowDiv` + `uiIsFlowPara` + `uiIsFlowSpan` + `uiIsFlowLine` + `uiIsFlowTref` + `uiIsFlowRegionExclude` + `uiIsFlowImage` + `uiIsSolidColor` + `uiIsMesh`); kbrowser deepen
15. **SH28 inte klar** — `src/` är produkt-skuld (`nollDropSrc=false`, `nollUserNoRustc=false`); flag-log (`nollSrcGoalZero=0` + `nollBootstrapFromKabOk=true` + `nollAllGatesClosedOk=true` + `nollAotReady=false` + `nollAotProcess=false` + `nollImageIsProcess=false` + `nollMmapExecProcess=false` + `nollStubIsProcess=false` + `nollSyscallIsKab=false` + `nollRustcNotHost=false` + `nollHostOptional=false` + `nollNoNewRs=true` + `nollKeepSrc` + `nollDropSrc=false` + `nollProcessIsKab=false` + `nollBootstrapImage` + `nollCargoNotRuntime=false` + `nollRustcNotProcess=false` + `nollMmapStub=false` + `nollStubFrozen=false` + `nollHostSyscallGone=false` + `nollProductSrcGone=false` + `nollCargoTomlGone=false` + `nollRustcCiGone=false` + `nollKabtestProductCi=false` + `nollUserNoRustc=false` + `nollCraneliftGone=false` + `nollJitIsKab=false` + `nollVmIsKab=false` + `nollGcIsKab=false` + `nollCompileIsKab=false` + `nollParseIsKab=false` + `nollLexIsKab=false` + `nollTypeIsKab=false` + `nollEmitIsKab=false` + `nollOptIsKab=false` + `nollLinkIsKab=false` + `nollStdIsKab=false` + `nollCliIsKab=false` + `nollReplIsKab=false` + `nollFmtIsKab=false` + `nollLspIsKab=false` + `nollDocIsKab=false` + `nollBenchIsKab=false` + `nollNewIsKab=false` + `nollInitIsKab=false` + `nollWatchIsKab=false` + `nollCleanIsKab=false` + `nollAddIsKab=false` + `nollRmIsKab=false` + `nollModIsKab=false` + `nollLsIsKab=false` + `nollCatIsKab=false` + `nollPkgIsKab=false` + `nollLockIsKab=false` + `nollPubIsKab=false` + `nollRegIsKab=false` + `nollAuthIsKab=false` + `nollLogIsKab=false` + `nollDbgIsKab=false` + `nollProfIsKab=false` + `nollTraceIsKab=false` + `nollCovIsKab=false` + `nollFuzzIsKab=false` + `nollSanIsKab=false` + `nollSnapIsKab=false` + `nollMockIsKab=false` + `nollFixIsKab=false` + `nollSpyIsKab=false` + `nollFakeIsKab=false` + `nollClockIsKab=false` + `nollRandIsKab=false` + `nollNetIsKab=false` + `nollDnsIsKab=false` + `nollTlsIsKab=false` + `nollHttpIsKab=false` + `nollWsIsKab=false` + `nollUdpIsKab=false` + `nollQuicIsKab=false` + `nollIcmpIsKab=false` + `nollSctpIsKab=false` + `nollGrpcIsKab=false` + `nollMqttIsKab=false` + `nollSmtpIsKab=false` + `nollImapIsKab=false` + `nollPopIsKab=false` + `nollFtpIsKab=false` + `nollSshIsKab=false` + `nollLdapIsKab=false` + `nollNtpIsKab=false` + `nollSnmpIsKab=false` + `nollDhcpIsKab=false` + `nollTftpIsKab=false` + `nollRadiusIsKab=false` + `nollKerberosIsKab=false` + `nollOauthIsKab=false` + `nollOidcIsKab=false` + `nollSamlIsKab=false` + `nollJwtIsKab=false` + `nollJwksIsKab=false` + `nollWebauthnIsKab=false` + `nollTotpIsKab=false` + `nollHotpIsKab=false` + `nollArgonIsKab=false` + `nollScryptIsKab=false` + `nollBcryptIsKab=false` + `nollPbkdfIsKab=false` + `nollHkdfIsKab=false` + `nollHmacIsKab=false` + `nollShaIsKab=false` + `nollAesIsKab=false` + `nollChachaIsKab=false` + `nollPolyIsKab=false` + `nollX25519IsKab=false` + `nollEd25519IsKab=false` + `nollKyberIsKab=false` + `nollDilithiumIsKab=false` + `nollSphincsIsKab=false` + `nollFalconIsKab=false` + `nollNtruIsKab=false` + `nollMcelieceIsKab=false` + `nollBikeIsKab=false` + `nollHqcIsKab=false` + `nollFrodoIsKab=false` + `nollSikeIsKab=false` + `nollRainbowIsKab=false` + `nollGemssIsKab=false` + `nollPicnicIsKab=false` + `nollXmssIsKab=false` + `nollLmsIsKab=false` + `nollWotsIsKab=false` + `nollMerkleIsKab=false` + `nollSlhdsaIsKab=false` + `nollMlkemIsKab=false` + `nollMldsaIsKab=false` + `nollFndsaIsKab=false` + `nollHybridIsKab=false` + `nollXwingIsKab=false` + `nollHpkeIsKab=false` + `nollNoiseIsKab=false` + `nollAgeIsKab=false` + `nollPgpIsKab=false` + `nollMinisignIsKab=false` + `nollSignifyIsKab=false` + `nollCosignIsKab=false` + `nollNotaryIsKab=false` + `nollRekorIsKab=false` + `nollFulcioIsKab=false` + `nollSigstoreIsKab=false` + `nollIntotoIsKab=false` + `nollSlsaIsKab=false` + `nollSpdxIsKab=false` + `nollCyclonedxIsKab=false` + `nollSbomIsKab=false` + `nollVexIsKab=false` + `nollCveIsKab=false` + `nollCweIsKab=false` + `nollCpeIsKab=false` + `nollCvssIsKab=false` + `nollOsvIsKab=false` + `nollGhsaIsKab=false` + `nollNvdIsKab=false` + `nollKevIsKab=false` + `nollCisaIsKab=false` + `nollMitreIsKab=false` + `nollAttackIsKab=false` + `nollCapecIsKab=false + nollStixIsKab=false + nollTaxiiIsKab=false + nollMispIsKab=false + nollOpenctiIsKab=false + nollThehiveIsKab=false + nollCortexIsKab=false + nollShuffleIsKab=false + nollWazuhIsKab=false + nollOsqueryIsKab=false + nollSuricataIsKab=false + nollZeekIsKab=false + nollSnortIsKab=false + nollYaraIsKab=false + nollSigmaIsKab=false + nollClamavIsKab=false + nollVirustotalIsKab=false + nollHybridanalysisIsKab=false + nollAnyrunIsKab=false + nollCuckooIsKab=false + nollJoeIsKab=false + nollCapeIsKab=false`); **radera inte `src/`**
16. ~~**F10 AOT native-image policy**~~ ✅ (ret-stub + sym/reloc + `nollAotReady` dual-bind); ~~**SH17–SH19 deepen**~~ ✅ (`jitMmapOk`, loopN arith-imm + bit-ops/shifts/unary/eq/ne/lt/gt/le/ge/test/je/jne/jmp/jl/jle/jg/jge/nop exec, `gcHostDeleteOk`, `loadMainDeleteOk` still false)
17. ~~**F18 App-CI**~~ ✅ subset (`appCiHttpOk` ≤100 ms + `appCiFpsOk` ≥60 + `appCiNdOk` ≤50 ms); live-app deepen — harness i `.kab`, inte ny `src/`-profiler
18. ~~**F19 zero-copy**~~ ✅ subset (`zcSqlViewOk` + `zcNdViewOk` 0 extra copies); live buffer/histogram deepen
19. ~~**F23 mmap-bulk**~~ ✅ subset (`mmF64Ok` f64 stride 8 + `mmU8Ok` u8 stride 1); GPU-pekare deepen
20. ~~**F22 TLAB**~~ ✅ subset (`tlabOk` ≥2 workers + `tlabPromoteOk` full→old dual-bind `gcHostDeleteOk`); pause-histogram deepen
21. ~~**F20 typspec**~~ ✅ subset (`jitSpecOk` i64 + `jitSpecF64Ok` f64 + `jitSpecDeoptOk` ny typ dual-bind `icIsMono`); native clone deepen
22. ~~**F24 TCO**~~ ✅ subset (`jitTcoOk` `@tail` + `jitTcoSelfOk` self-recursion dual-bind `jitSsaOk`); frame-reuse deepen
23. ~~**F21 lazy AOT**~~ ✅ subset (`aotLazyOk` ≤100 ms + `aotLazyJitOk` remainder JIT dual-bind `aotPgoOk`); live boot-DAG deepen

## Historisk bootstrap-logg

1. ~~`.kbc` roundtrip: `deserialize(serialize_bc(emit(ast)))` i Rust~~ ✅
2. ~~`fn`-anrop: `OP_CALL` mot self-hosted `functions[]`~~ ✅ (Rust `run_module`)
3. ~~`parse.kab`-facaden (nested `tokenize`)~~ ✅
4. ~~Full pipeline: `compile(source)` entrypoint~~ ✅
5. ~~Self-host bootstrap: `compile.kab` cache + `compile(sample)` -> Rust `run_module`~~ ✅
6. ~~Utöka self-hosted språksubset (obj, &&, compares, index)~~ ✅
7. ~~Lexer-like compile (`char_at`-loop, `!=`, `continue`/`break`/`undefined`)~~ ✅
8. ~~Self-host hela `lexer.kab` via `compile()`~~ ✅ — `self_host_lexer_full_compile_and_run` (~2.5 h)

9. ~~Self-host `parser.kab` / `emit.kab` (större moduler, fler opcodes).~~ ✅
   - **parser.kab** (~960 rader, 9 fn): generics (`<T>` på fn/class/enum, type args på call/member), `self_host_parser_suite` via Rust bytecode-preload (undviker Windows OOM). `self_host_parser_full_compile_and_run` (~2.5 h) verifierar `compile(parser.kab)` → `parseTokens(tokenize("let x = 1"))`.
   - **emit.kab** (~850 rader, 8 fn): redo för vidare opcode-stöd om parsern utökas (`||`, unary `!`, `*`, assign till index, etc.).
    - **Verifiering:** snabb: `self_host_emit_suite` (3 subprocess-chunks: core / generics / calls — undviker Windows OOM), `self_host_parser_full_compile_smoke`, `self_host_emit_full_compile_smoke`; långsam: `self_host_parser_full_compile_and_run` (ignored, ~2.5 h).

10. ~~Self-host hela `emit.kab` via `compile()` → kör `emit(parse("let x = 1"))` i bytecode~~ ✅
    - Snabb smoke: `self_host_emit_full_compile_smoke`.
    - Långsam CI: `self_host_emit_full_compile_and_run` (ignored, ~2–3 h).
    - Run-only: `self_host_emit_kbc_run_only` (kräver `_emit_full_out.kbc`).

11. ~~Self-host hela `serialize.kab` via `compile()` → kör `serialize_bc(emit(parse(...)))` + roundtrip~~ ✅
    - Snabb smoke: `self_host_serialize_full_compile_smoke`.
    - Långsam CI: `self_host_serialize_full_compile_and_run` (ignored, ~40 min).
    - Run-only: `self_host_serialize_kbc_run_only` (kräver `_serialize_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile serialize.kab`
    - **Kör tunga tester med `--test-threads=1`** (parallella serialize-tester kan OOM:a på Windows).

12. ~~True bootstrap — `compile(compile.kab)` körs som self-hosted bytecode och kan `compile(sample)`~~ ✅
    - Snabb smoke: `self_host_compile_full_compile_smoke` (subprocess — djup pipeline overflowar test-stack).
    - Långsam CI: `self_host_compile_full_compile_and_run` (ignored, ~3 min för compile.kab).
    - Run-only: `self_host_compile_kbc_run_only` (kräver `_compile_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile compile.kab`

13. ~~**Generics (språk):** Rust v1 + self-host G4~~ ✅ — `fn id<T>`, monomorphisering, `tests/generics.rs`, `test_parser.kab` / `test_emit.kab`. Design: [docs/GENERICS.md](../docs/GENERICS.md). **Struct** ✅ — `self` / `&self` / `&mut self` (self-host parse+emit); **`struct Box<T>`** med fälttyp `T` → `Number`; **G8.1** `b.echo(1)` → `echo$Number`; **`class Child<T> extends Base<T>`** → `Child$Number` extends `Base$Number`; **`super.tag()`** / **`super.init(...)`** → `get_super_method`; **`super.count = 1`** / **`super.n += 2`**; **`||=` `&&=` `??=`**; **`?.`**; **`? :`**; **`step()?`**; **`switch`+`fallthrough`**; **`do`/`while`**; **`this.run(super.f)`**; **`xs[0] += 3`**; **template `` `n=${n}` ``**; **`is(obj, "Class")`**; **`pass`/`raise`/`assert`/`not`**; **`with`/`is`/`is not`**; **`using x = expr`**; **`import.meta`**; **`delete o.x`**; **`for let i = 0; …`**; **G9** `Option.Some(42)` → `Option$Number`; **`match Option.Some(n)`** / **`Option<Number>.None`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** kab-only; **G10** två `Option`/`Box`/`echo`/`id`-specialiseringar; **`pair$Number_String`**; **`id(id(42))`**; **`Result<Number, String>.Ok`**; **`Box<String>(…)`** explicit; **`id(b)`** → `id$Box`. **T5** ✅ — trait default-metoder emit+inject på `implements`; **`type Item = Number`** på klass (`class_assoc_types`); **`where T: Trait`** på generiska fn, metoder och klasser (`emitCheckWhere`); **`trait Show<T>`** (`interface_type_params`, `implements Show$Number`). Kvarvarande self-host-arbete: **P6b** leaf-budget ([seed/README.md](seed/README.md)). Semikolon förblir valfria.

14. **Generics fas 2 (G6–G11):** ~~G6 inferens~~ ✅, ~~G7 klassmetoder~~ ✅, ~~G8 klasser~~ ✅, ~~G9 enum~~ ✅, ~~G10 self-host~~ ✅, ~~G11 LSP~~ ✅. Plan: [docs/GENERICS.md#fas-2--g6-planering](../docs/GENERICS.md#fas-2--g6-planering), roadmap **Våg F** i [docs/ROADMAP.md](../docs/ROADMAP.md).

## Profilering (compile-tid)

Efter grön `emit` full compile — hitta flaskhalsar innan M11/M12.

```bash
# Fas-tid: parse / emit / serialize (emit.kab, kan ta timmar)
python scripts/profile_emit_compile.py phases emit.kab

# P6b leaf (minsta skip-listade källan) — samma pipeline
python scripts/profile_emit_compile.py phases self_host/serialize_body.kab

# Wall-time compile() end-to-end
python scripts/profile_emit_compile.py compile emit.kab

# Prefix-skala: vilka radintervall dominerar
python scripts/profile_emit_compile.py bisect emit

# Jämför lexer / parser / emit
python scripts/profile_emit_compile.py compare

# Run-fas (kräver _emit_full_out.kbc)
CARGO_TARGET_DIR=target-alt3 cargo test --test self_host self_host_emit_profile_run_phases -- --ignored --nocapture
```

Output-rader `PROFILE ...` är maskinläsbara. `popStack()` och stack-trim-loopar använder nu native `pop()` (kräver ny `compile(emit.kab)` för `.kbc`).

Snabb smoke: `cargo test --test self_host self_host_profile_phases_smoke`.

### P6b (skip-list → tom lista)

Se [seed/README.md](seed/README.md) för policy, playbook, **fas-profil** och baslinjer.

- Produktpath = committed seeds; **töm inte** listan förrän alla fem löv
  `compile_source_self_host` < 10 s (`P6_SELF_HOST_LEAF_CI_FAST_MS`).
- Fas-profil (mid AccAdd): parse ≈ 37% | emit ≈ 48% | serialize ≈ 15%. Landade cuts:
  maps/`emitSym`, iterative compare, **`eIfDepth`/`eMemberDepth`/`eIndexDepth`**,
  CallArg/obj/arr + callee/block depth, early `IDENT=`, `eOpsN` patches, IR + AccAdd densify.
- Leaf densify plateau → host-VM **`Rc` Array/Object** (COW + cycle reject) + Len/IndexGet.
  `serialize_body` **~144 s** debug — still ≫ 10 s, **skip-list stays**.
- Efter `emit_impl` / `parser_impl` / `serialize_body`-ändring: regenerera motsvarande `self_host/seed/*.kbc`.
