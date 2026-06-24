//! Utility-first code snippets — Tailwind-style building blocks for Kabootar.

#[derive(Debug, Clone, Copy)]
pub struct CodeUtil {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub code: &'static str,
    pub tags: &'static [&'static str],
}

pub const UTILS: &[CodeUtil] = &[
    CodeUtil {
        id: "http-route-get",
        category: "http",
        title: "GET route",
        description: "Registrera en GET-handler och returnera JSON/text.",
        code: r#"import "http";

fn handler() {
    return ok("Hello from Kabootar")
}

http_route("GET", "/api/hello", handler);
"#,
        tags: &["http", "get", "route", "api", "rest"],
    },
    CodeUtil {
        id: "http-route-post",
        category: "http",
        title: "POST route",
        description: "POST-endpoint som läser request-body.",
        code: r#"import "http";

fn create_item() {
    let body = http_body(http_request("POST", "/api/items"));
    return ok(body)
}

http_route("POST", "/api/items", create_item);
"#,
        tags: &["http", "post", "route", "api", "rest", "body"],
    },
    CodeUtil {
        id: "http-health",
        category: "http",
        title: "Health check",
        description: "Enkel /health-endpoint för load balancers.",
        code: r#"import "http";

fn health() {
    return ok("ok")
}

http_route("GET", "/health", health);
"#,
        tags: &["http", "health", "ping", "monitor"],
    },
    CodeUtil {
        id: "http-serve",
        category: "http",
        title: "Starta server",
        description: "Starta HTTP-server på port 8080.",
        code: r#"import "http";

http_serve(8080);
"#,
        tags: &["http", "serve", "server", "listen", "port"],
    },
    CodeUtil {
        id: "http-json-ok",
        category: "http",
        title: "JSON-svar",
        description: "Returnera strukturerat JSON-svar.",
        code: r#"import "http";

fn api_status() {
    return ok("{\"status\":\"ready\",\"version\":\"1.0\"}")
}

http_route("GET", "/api/status", api_status);
"#,
        tags: &["http", "json", "api", "response"],
    },
    CodeUtil {
        id: "sql-create-table",
        category: "sql",
        title: "CREATE TABLE",
        description: "Skapa tabell med id och namn.",
        code: r#"import "sql";

sql_exec("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)");
"#,
        tags: &["sql", "create", "table", "schema", "database"],
    },
    CodeUtil {
        id: "sql-insert",
        category: "sql",
        title: "INSERT",
        description: "Infoga rad i tabell.",
        code: r#"import "sql";

sql_exec("INSERT INTO users (name) VALUES ('Alice')");
"#,
        tags: &["sql", "insert", "create", "database"],
    },
    CodeUtil {
        id: "sql-select",
        category: "sql",
        title: "SELECT WHERE",
        description: "Hämta rader med filter.",
        code: r#"import "sql";

let rows = sql_query("SELECT id, name FROM users WHERE id = 1");
"#,
        tags: &["sql", "select", "query", "where", "read"],
    },
    CodeUtil {
        id: "sql-update",
        category: "sql",
        title: "UPDATE",
        description: "Uppdatera befintlig rad.",
        code: r#"import "sql";

sql_exec("UPDATE users SET name = 'Bob' WHERE id = 1");
"#,
        tags: &["sql", "update", "database"],
    },
    CodeUtil {
        id: "mod-pub-fn",
        category: "mod",
        title: "Exportera funktion",
        description: "pub fn för filmoduler.",
        code: r#"pub fn greet(name) {
    return "Hello, " + name
}
"#,
        tags: &["module", "export", "pub", "fn", "library"],
    },
    CodeUtil {
        id: "mod-pub-let",
        category: "mod",
        title: "Exportera konstant",
        description: "pub let för delade konstanter.",
        code: r#"pub let VERSION = "1.0.0";
pub let MAX_ITEMS = 100;
"#,
        tags: &["module", "export", "pub", "let", "constant"],
    },
    CodeUtil {
        id: "mod-import-file",
        category: "mod",
        title: "Importera filmodul",
        description: "import från lib/ eller projektmapp.",
        code: r#"import "greet";

greet("world");
"#,
        tags: &["import", "module", "file", "lib"],
    },
    CodeUtil {
        id: "mod-versioned",
        category: "mod",
        title: "Versionerad import",
        description: "import med @version (v2.1).",
        code: r#"import "config@1.0";

let host = CONFIG_HOST;
"#,
        tags: &["import", "version", "dependency", "package"],
    },
    CodeUtil {
        id: "class-basic",
        category: "class",
        title: "Klass med fält",
        description: "Enkel klass med konstruktor och metod.",
        code: r#"class Point {
    fn init(x, y) {
        self.x = x;
        self.y = y;
    }
    fn sum() {
        return self.x + self.y;
    }
}

let p = Point(3, 4);
"#,
        tags: &["class", "oop", "object", "method"],
    },
    CodeUtil {
        id: "science-stats",
        category: "science",
        title: "Statistik — medel & std",
        description: "Beräkna medelvärde och standardavvikelse.",
        code: r#"import "science";

let data = [2, 4, 4, 4, 5, 5, 7, 9];
let mean = stat_mean(data);
let std = stat_std(data);
"#,
        tags: &["science", "stats", "mean", "std", "statistics"],
    },
    CodeUtil {
        id: "science-linreg",
        category: "science",
        title: "Linjär regression",
        description: "stat_linreg för x/y-serier.",
        code: r#"import "science";

let xs = [1, 2, 3, 4, 5];
let ys = [2, 4, 5, 4, 5];
let fit = stat_linreg(xs, ys);
"#,
        tags: &["science", "stats", "regression", "linear"],
    },
    CodeUtil {
        id: "science-matrix",
        category: "science",
        title: "Matris-multiplikation",
        description: "mat_mul för 2D-matriser.",
        code: r#"import "science";

let a = [[1, 2], [3, 4]];
let b = [[5, 6], [7, 8]];
let c = mat_mul(a, b);
"#,
        tags: &["science", "matrix", "math", "linear-algebra"],
    },
    CodeUtil {
        id: "project-main",
        category: "project",
        title: "Projekt main.kab",
        description: "Minimal entrypoint med HTTP.",
        code: r#"import "http";

fn index() {
    return ok("Kabootar project")
}

http_route("GET", "/", index);
http_serve(8080);
"#,
        tags: &["project", "main", "entry", "web", "starter"],
    },
    CodeUtil {
        id: "project-api",
        category: "project",
        title: "API-projekt",
        description: "REST API med health + items.",
        code: r#"import "http";

fn health() {
    return ok("ok")
}

fn list_items() {
    return ok("[\"item1\",\"item2\"]")
}

http_route("GET", "/health", health);
http_route("GET", "/api/items", list_items);
http_serve(8080);
"#,
        tags: &["project", "api", "rest", "starter"],
    },
    CodeUtil {
        id: "array-literal",
        category: "lang",
        title: "Array-literal",
        description: "Skapa och iterera array.",
        code: r#"let items = [1, 2, 3];
let first = items[0];
"#,
        tags: &["array", "literal", "collection", "lang"],
    },
    CodeUtil {
        id: "closure-fn",
        category: "lang",
        title: "Closure / callback",
        description: "Funktion som argument.",
        code: r#"fn apply(fn, x) {
    return fn(x)
}

fn double(n) {
    return n * 2
}

apply(double, 5);
"#,
        tags: &["closure", "callback", "function", "higher-order"],
    },
    CodeUtil {
        id: "crypto-hash",
        category: "crypto",
        title: "SHA-256 hash",
        description: "Hasha sträng med crypto-modulen.",
        code: r#"import "crypto";

let digest = sha256("secret data");
"#,
        tags: &["crypto", "hash", "sha256", "security"],
    },
];

pub fn util_by_id(id: &str) -> Option<&'static CodeUtil> {
    let key = id.to_lowercase().replace('.', "-").replace('_', "-");
    UTILS.iter().find(|u| u.id == key || u.id.replace('-', "") == key.replace('-', ""))
}

pub fn all_ids() -> Vec<&'static str> {
    UTILS.iter().map(|u| u.id).collect()
}

pub fn categories() -> Vec<&'static str> {
    let mut cats: Vec<&str> = UTILS.iter().map(|u| u.category).collect();
    cats.sort();
    cats.dedup();
    cats
}
