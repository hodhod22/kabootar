//! v2.8 — parallell async IO

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn os_read_async_returns_content() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn load() {
            return await os_read_async("/data.txt")
        }
        os_write("/data.txt", "hello")
        await load()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "hello"));
}

#[test]
fn parallel_os_reads_via_await_all() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        os_write("/a.txt", "A")
        os_write("/b.txt", "B")
        let p1 = os_read_async("/a.txt")
        let p2 = os_read_async("/b.txt")
        let xs = await_all([p1, p2])
        xs[0] + xs[1]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "AB"));
}

#[test]
fn http_request_async() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn ping() {
            return http_response(200, "pong")
        }
        http_route("GET", "/ping", ping)

        async fn fetch() {
            let res = await http_request_async("GET", "/ping", "")
            return http_body(res)
        }
        await fetch()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "pong"));
}

#[test]
fn sql_async_query() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        sql("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        sql("INSERT INTO items (id, name) VALUES (1, 'alpha')")

        async fn load() {
            return await sql_async("SELECT name FROM items WHERE id = 1")
        }
        await load()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "alpha"));
}

#[test]
fn async_io_interleaves_with_sleep() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        os_write("/x.txt", "X")
        async fn slow() {
            await sleep_ticks(1)
            return await os_read_async("/x.txt")
        }
        async fn fast() {
            return "Y"
        }
        let p1 = slow()
        let p2 = fast()
        await p2 + await p1
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "YX"));
}
