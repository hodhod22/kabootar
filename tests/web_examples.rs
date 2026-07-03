use kabootar_lib::evaluate;

#[test]
fn while_loop_increments_to_five() {
    let result = evaluate("let i = 0; while i < 5 { i = i + 1 }; i");
    assert_eq!(result, "5");
}

#[test]
fn while_with_break_stops_at_three() {
    let result = evaluate("let i = 0; while i < 10 { if i == 3 { break } else { i = i + 1 } }; i");
    assert_eq!(result, "3");
}

#[test]
fn null_and_undefined_are_distinct() {
    assert_eq!(evaluate("null == null"), "true");
    assert_eq!(evaluate("undefined == undefined"), "true");
    assert_eq!(evaluate("null == undefined"), "false");
}

#[test]
fn is_null_and_is_undefined() {
    assert_eq!(evaluate("is_null(null)"), "true");
    assert_eq!(evaluate("is_undefined(undefined)"), "true");
    assert_eq!(evaluate("is_null(undefined)"), "false");
}

#[test]
fn sql_select_one() {
    assert_eq!(evaluate("sql(\"SELECT 1\")"), "1");
}

#[test]
fn sql_persistent_insert_and_select() {
    let code = r#"
        sql("CREATE TABLE users (id INTEGER, name TEXT)");
        sql("INSERT INTO users (id, name) VALUES (1, 'Ada')");
        sql("INSERT INTO users (id, name) VALUES (2, 'Bob')");
        sql("SELECT name FROM users WHERE id = $1", 1)
    "#;
    assert_eq!(evaluate(code), "Ada");
}

#[test]
fn sql_select_all_rows() {
    let code = r#"
        sql("CREATE TABLE nums (id INTEGER)");
        sql("INSERT INTO nums (id) VALUES (1)");
        sql("INSERT INTO nums (id) VALUES (2)");
        sql("SELECT id FROM nums")
    "#;
    assert_eq!(evaluate(code), "[1, 2]");
}

#[test]
fn sql_parameterized_where() {
    let code = r#"
        sql("CREATE TABLE items (id INTEGER, price INTEGER)");
        sql("INSERT INTO items (id, price) VALUES (1, 50)");
        sql("INSERT INTO items (id, price) VALUES (2, 150)");
        sql("SELECT id FROM items WHERE price > $1", 100)
    "#;
    assert_eq!(evaluate(code), "2");
}

#[test]
fn sql_join() {
    let code = r#"
        sql("CREATE TABLE users (id INTEGER, name TEXT)");
        sql("CREATE TABLE orders (id INTEGER, user_id INTEGER, total INTEGER)");
        sql("INSERT INTO users (id, name) VALUES (1, 'Ada')");
        sql("INSERT INTO orders (id, user_id, total) VALUES (1, 1, 100)");
        sql("SELECT users.name, orders.total FROM users JOIN orders ON users.id = orders.user_id")
    "#;
    assert_eq!(evaluate(code), "[[Ada, 100]]");
}

#[test]
fn class_field_assignment() {
    let code = r#"
        class Person { name: string; }
        let p = Person();
        p.name = "Ada";
        p.name
    "#;
    assert_eq!(evaluate(code), "Ada");
}

#[test]
fn class_method_with_self() {
    let code = r#"
        class Person { name: string; fn greet() { return "Hi " + self.name } }
        let p = Person();
        p.name = "Ada";
        p.greet()
    "#;
    assert_eq!(evaluate(code), "Hi Ada");
}

#[test]
fn class_field_default_value() {
    let code = r#"
        class Counter { count: number = 0; }
        let c = Counter();
        c.count
    "#;
    assert_eq!(evaluate(code), "0");
}

#[test]
fn float_literal() {
    assert_eq!(evaluate("3.14"), "3.14");
}

#[test]
fn float_arithmetic() {
    assert_eq!(evaluate("1.5 + 2.5"), "4");
    assert_eq!(evaluate("10.0 / 4.0"), "2.5");
}

#[test]
fn is_nan_with_float() {
    assert_eq!(evaluate("is_nan(NaN)"), "true");
    assert_eq!(evaluate("is_nan(3.14)"), "false");
}

#[test]
fn kml_parse_and_render() {
    assert_eq!(
        evaluate(r#"kdom_render(kml("<p>Hi</p>"))"#),
        "<p>Hi</p>"
    );
}

#[test]
fn kml_nested_with_attributes() {
    assert_eq!(
        evaluate(r#"kdom_render(kml("<div class='main'><h1>Title</h1></div>"))"#),
        r#"<div class="main"><h1>Title</h1></div>"#
    );
}

#[test]
fn os_write_read_and_exists() {
    assert_eq!(
        evaluate("os_write(\"/hello.txt\", \"Kabootar\"); os_read(\"/hello.txt\")"),
        "Kabootar"
    );
    assert_eq!(
        evaluate("os_write(\"/x.txt\", \"x\"); os_exists(\"/x.txt\")"),
        "true"
    );
}

#[test]
fn os_list_and_delete() {
    assert_eq!(
        evaluate(
            r#"os_mkdir("/data/listtest"); os_write("/data/listtest/a.txt", "1"); os_write("/data/listtest/b.txt", "2"); os_list("/data/listtest")"#,
        ),
        "[a.txt, b.txt]"
    );
    assert_eq!(
        evaluate(
            r#"os_mkdir("/data/listtest2"); os_write("/data/listtest2/a.txt", "1"); os_write("/data/listtest2/b.txt", "2"); os_delete("/data/listtest2/a.txt"); os_list("/data/listtest2")"#,
        ),
        "[b.txt]"
    );
}

#[test]
fn http_route_and_request() {
    let code = r#"
        fn hello() { return http_response(200, "Hello") }
        http_route("GET", "/hello", hello);
        http_body(http_request("GET", "/hello"))
    "#;
    assert_eq!(evaluate(code), "Hello");
}

#[test]
fn http_echo_request_body() {
    let code = r#"
        fn echo() { return http_response(200, req_body) }
        http_route("POST", "/echo", echo);
        http_body(http_request("POST", "/echo", "ping"))
    "#;
    assert_eq!(evaluate(code), "ping");
}

#[test]
fn http_status_and_not_found() {
    assert_eq!(
        evaluate("http_status(http_request(\"GET\", \"/missing\"))"),
        "404"
    );
}

#[test]
fn http_process_raw_request() {
    let code = r#"
        fn ping() { return http_response(200, "pong") }
        http_route("GET", "/ping", ping);
        http_process("GET /ping HTTP/1.1\r\n\r\n")
    "#;
    let result = evaluate(code);
    assert!(result.contains("200 OK"));
    assert!(result.contains("pong"));
}

#[test]
fn os_caps_lists_kernel_features() {
    let caps = evaluate("os_caps()");
    assert!(caps.contains("vfs"));
    assert!(caps.contains("permissions"));
    assert!(caps.contains("hotplug"));
}

#[test]
fn os_mkdir_and_stat() {
    let code = r#"
        os_mkdir("/apps");
        os_write("/apps/note.txt", "Hi");
        os_stat("/apps/note.txt")
    "#;
    let out = evaluate(code);
    assert!(out.starts_with("[file, 2,"), "unexpected os_stat: {out}");
}

#[test]
fn import_math_module() {
    assert_eq!(evaluate(r#"import "math"; add(2, 3)"#), "5");
}

#[test]
fn import_http_helpers() {
    let code = r#"
        import "http";
        fn home() { return ok("Kabootar") }
        http_route("GET", "/", home);
        http_body(http_request("GET", "/"))
    "#;
    assert_eq!(evaluate(code), "Kabootar");
}

#[test]
fn import_http_all_verbs() {
    let code = r#"
        import "http";
        fn users_list() { return ok("[]") }
        fn create() { return created(req_body) }
        fn update() { return ok(req_body) }
        fn delete_user() { return no_content() }
        route_get("/api/users", users_list)
        route_post("/api/users", create)
        route_put("/api/users/1", update)
        route_patch("/api/users/1", update)
        route_delete("/api/users/1", delete_user)
        let a = http_body(request_get("/api/users"))
        let b = http_status(request_post("/api/users", "{}"))
        let c = http_body(request_put("/api/users/1", "x"))
        let d = http_status(request_delete("/api/users/1"))
        [a, b, c, d]
    "#;
    assert_eq!(evaluate(code), "[[], 201, x, 204]");
}

#[test]
fn sql_update_and_order_by() {
    let code = r#"
        sql("CREATE TABLE items (id INTEGER, score INTEGER)");
        sql("INSERT INTO items (id, score) VALUES (1, 10)");
        sql("INSERT INTO items (id, score) VALUES (2, 30)");
        sql("UPDATE items SET score = 20 WHERE id = 1");
        sql("SELECT id FROM items ORDER BY score DESC LIMIT 1")
    "#;
    assert_eq!(evaluate(code), "2");
}

#[test]
fn sql_count_and_delete() {
    let code = r#"
        sql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)");
        sql("INSERT INTO users (id, name) VALUES (1, 'Ada')");
        sql("INSERT INTO users (id, name) VALUES (2, 'Bob')");
        sql("SELECT COUNT(*) FROM users");
    "#;
    assert_eq!(evaluate(code), "2");
    let delete_code = r#"
        sql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)");
        sql("INSERT INTO users (id, name) VALUES (1, 'Ada')");
        sql("INSERT INTO users (id, name) VALUES (2, 'Bob')");
        sql("DELETE FROM users WHERE id = 2");
        sql("SELECT COUNT(*) FROM users")
    "#;
    assert_eq!(evaluate(delete_code), "1");
}

#[test]
fn sql_is_null() {
    let code = r#"
        sql("CREATE TABLE t (id INTEGER, note TEXT)");
        sql("INSERT INTO t (id, note) VALUES (1, NULL)");
        sql("INSERT INTO t (id, note) VALUES (2, 'ok')");
        sql("SELECT COUNT(*) FROM t WHERE note IS NULL")
    "#;
    assert_eq!(evaluate(code), "1");
}

#[test]
fn security_list_providers() {
    let result = evaluate("security_list_providers()");
    assert!(result.contains("software"));
    assert!(result.contains("tpm-stub"));
}

#[test]
fn security_device_list() {
    let result = evaluate("device_list()");
    assert!(result.contains("usb-0"));
    assert!(result.contains("tpm-0"));
}

#[test]
#[cfg(feature = "crypto")]
fn crypto_sha3_and_secure_wipe() {
    let hash = evaluate("crypto_sha3_256(\"password\")");
    assert!(!hash.is_empty());
    assert!(!hash.starts_with("Lexer error"));
    assert!(!hash.starts_with("Parse error"));
    let wiped = evaluate(r#"
        let key = crypto_secure(crypto_random(16));
        crypto_wipe(key);
        crypto_is_secure(key)
    "#);
    assert_eq!(wiped, "true");
}

#[test]
#[cfg(feature = "crypto")]
fn crypto_aes_roundtrip() {
    let code = r#"
        let key = crypto_random(32);
        let nonce = crypto_random(12);
        let enc = crypto_aes256_encrypt(key, nonce, "secret");
        crypto_aes256_decrypt(key, nonce, enc)
    "#;
    let result = evaluate(code);
    assert!(result.contains("115"));
}

#[test]
fn science_complex_and_physics() {
    let code = r#"
        import "science";
        c_abs(cplx(3, 4))
    "#;
    assert_eq!(evaluate(code), "5");
}

#[test]
fn science_quadratic_equation() {
    let code = r#"
        import "science";
        kinetic_energy(2, 3)
    "#;
    assert_eq!(evaluate(code), "9");
}

#[test]
fn science_chemistry_ph() {
    let code = r#"
        import "science";
        ph(0.001)
    "#;
    assert_eq!(evaluate(code), "3");
}

#[test]
fn science_economics_compound() {
    let code = r#"
        import "science";
        compound(1000, 0.05, 2)
    "#;
    let result = evaluate(code);
    assert!(result.starts_with("1102"));
}

#[test]
fn science_digital_hex() {
    let code = r#"
        import "science";
        hex("FF")
    "#;
    assert_eq!(evaluate(code), "255");
}

#[test]
fn science_ohms_law() {
    let code = r#"
        import "science";
        ohms_v(10, 2)
    "#;
    assert_eq!(evaluate(code), "20");
}

#[test]
fn science_stat_mean() {
    let code = r#"
        import "science";
        stat_mean([2, 4, 4, 4, 5, 5, 7, 9])
    "#;
    assert_eq!(evaluate(code), "5");
}

#[test]
fn docai_answers_science_question() {
    let code = r#"
        import "docai";
        doc_ask("hur importerar jag science")
    "#;
    let result = evaluate(code);
    assert!(result.contains("science") || result.contains("import") || result.contains("SCIENCE"));
}

#[test]
fn science_statistics() {
    let code = r#"
        import "science";
        stat_mean([2, 4, 4, 4, 5, 5, 7, 9])
    "#;
    assert_eq!(evaluate(code), "5");
}

#[test]
fn science_matrix_det() {
    let code = r#"
        import "science";
        mat_det([[1, 2], [3, 4]])
    "#;
    assert_eq!(evaluate(code), "-2");
}

#[test]
fn science_matrix_mul() {
    let code = r#"
        import "science";
        mat_mul([[1, 2], [3, 4]], [[5, 6], [7, 8]])
    "#;
    let result = evaluate(code);
    assert!(result.contains("19") && result.contains("22"));
}

#[test]
fn science_numerics_trapz() {
    let code = r#"
        import "science";
        num_trapz([1, 1], 1)
    "#;
    assert_eq!(evaluate(code), "1");
}

#[test]
fn science_linreg() {
    let code = r#"
        import "science";
        stat_linreg([1, 2, 3], [2, 4, 6])
    "#;
    let result = evaluate(code);
    assert!(result.contains("2"));
}
