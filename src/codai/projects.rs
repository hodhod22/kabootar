//! Project blueprints — folder structure + starter files per project type.

#[derive(Debug, Clone, Copy)]
pub struct ProjectFile {
    pub path: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectBlueprint {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub files: &'static [ProjectFile],
}

pub const BLUEPRINTS: &[ProjectBlueprint] = &[
    ProjectBlueprint {
        id: "web",
        title: "Webbprojekt",
        description: "HTTP-server med startsida och konfig-modul i lib/.",
        tags: &["web", "website", "frontend", "html", "sida"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "Projektkonfiguration (version, entry, port)",
                content: WEB_TOML,
            },
            ProjectFile {
                path: "main.kab",
                description: "Entrypoint — routes och server",
                content: WEB_MAIN,
            },
            ProjectFile {
                path: "index.html",
                description: "Statisk startsida",
                content: WEB_INDEX,
            },
            ProjectFile {
                path: "lib/config.kab",
                description: "Exporterade webbkonstanter",
                content: WEB_LIB_CONFIG,
            },
        ],
    },
    ProjectBlueprint {
        id: "api",
        title: "REST API",
        description: "JSON-API med SQL-databas och uppdelade handlers i lib/.",
        tags: &["api", "rest", "backend", "json", "endpoint"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "API-projektkonfiguration",
                content: API_TOML,
            },
            ProjectFile {
                path: "main.kab",
                description: "Startar DB-schema och registrerar routes",
                content: API_MAIN,
            },
            ProjectFile {
                path: "lib/config.kab",
                description: "API-namn och version",
                content: API_LIB_CONFIG,
            },
            ProjectFile {
                path: "lib/routes.kab",
                description: "HTTP-handlers (list/create)",
                content: API_LIB_ROUTES,
            },
        ],
    },
    ProjectBlueprint {
        id: "api-crud",
        title: "CRUD API",
        description: "Komplett CRUD för users med GET/POST/PUT/DELETE.",
        tags: &["crud", "api", "rest", "users", "database", "sql"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "CRUD-projektkonfiguration",
                content: API_CRUD_TOML,
            },
            ProjectFile {
                path: "main.kab",
                description: "Schema + route-registrering",
                content: API_CRUD_MAIN,
            },
            ProjectFile {
                path: "lib/users.kab",
                description: "User CRUD-handlers",
                content: API_CRUD_USERS,
            },
            ProjectFile {
                path: "lib/db.kab",
                description: "Databas-init",
                content: API_CRUD_DB,
            },
        ],
    },
    ProjectBlueprint {
        id: "science",
        title: "Data & statistik",
        description: "Science-modul med datafil och analys i lib/.",
        tags: &["science", "statistik", "data", "analys", "matris"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "Science-projektkonfiguration",
                content: SCIENCE_TOML,
            },
            ProjectFile {
                path: "main.kab",
                description: "Kör analys och skriver ut resultat",
                content: SCIENCE_MAIN,
            },
            ProjectFile {
                path: "lib/data.kab",
                description: "Exempeldata (arrayer)",
                content: SCIENCE_LIB_DATA,
            },
            ProjectFile {
                path: "lib/analysis.kab",
                description: "Statistik och regression",
                content: SCIENCE_LIB_ANALYSIS,
            },
        ],
    },
    ProjectBlueprint {
        id: "fullstack",
        title: "Fullstack (web + API)",
        description: "Webb + API i samma projekt med delad config.",
        tags: &["fullstack", "web", "api", "hela", "app", "full"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "Fullstack-konfiguration",
                content: FULLSTACK_TOML,
            },
            ProjectFile {
                path: "main.kab",
                description: "Kombinerar webb- och API-routes",
                content: FULLSTACK_MAIN,
            },
            ProjectFile {
                path: "index.html",
                description: "Frontend-startsida",
                content: FULLSTACK_INDEX,
            },
            ProjectFile {
                path: "static/app.css",
                description: "Enkel CSS (redigera fritt)",
                content: FULLSTACK_CSS,
            },
            ProjectFile {
                path: "lib/config.kab",
                description: "Delad app-konfiguration",
                content: FULLSTACK_LIB_CONFIG,
            },
            ProjectFile {
                path: "lib/api.kab",
                description: "API-endpoints",
                content: FULLSTACK_LIB_API,
            },
        ],
    },
    ProjectBlueprint {
        id: "library",
        title: "Bibliotek / modulpaket",
        description: "Återanvändbar lib/ med pub fn — inget HTTP.",
        tags: &["library", "lib", "modul", "package", "bibliotek", "delad"],
        files: &[
            ProjectFile {
                path: "kabootar.toml",
                description: "Paketmanifest",
                content: LIBRARY_TOML,
            },
            ProjectFile {
                path: "demo.kab",
                description: "Exempel som importerar lib-modulen",
                content: LIBRARY_DEMO,
            },
            ProjectFile {
                path: "lib/greet.kab",
                description: "Exporterad modul",
                content: LIBRARY_GREET,
            },
            ProjectFile {
                path: "lib/utils.kab",
                description: "Hjälpfunktioner",
                content: LIBRARY_UTILS,
            },
        ],
    },
];

const WEB_TOML: &str = r#"version = "0.1.0"
template = "web"
entry = "main.kab"
port = 8080

[dependencies]
"#;

const WEB_MAIN: &str = r#"@version "0.1.0"
import "http"
import "config"

http_route("GET", "/", home)
http_route("GET", "/health", health)

pub fn home() {
    return http_response(200, SITE_TITLE + " — redigera main.kab och lib/")
}

pub fn health() {
    return http_response(200, "ok")
}
"#;

const WEB_INDEX: &str = r#"<!DOCTYPE html>
<html lang="sv">
<head>
  <meta charset="UTF-8" />
  <title>Kabootar Web</title>
</head>
<body>
  <h1>Kabootar</h1>
  <p>Backend: <code>kabootar serve --watch main.kab</code></p>
  <p>Redigera denna fil eller ta bort den — din app, dina regler.</p>
</body>
</html>
"#;

const WEB_LIB_CONFIG: &str = r#"@version "1.0.0"
pub let SITE_TITLE = "Kabootar Web"
pub let SITE_VERSION = "0.1.0"
"#;

const API_TOML: &str = r#"version = "0.1.0"
template = "api"
entry = "main.kab"
port = 8080

[dependencies]
"#;

const API_MAIN: &str = r#"@version "0.1.0"
import "http"
import "config"
import "routes"

sql("CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)")

http_route("GET", "/health", health)
http_route("GET", "/api/items", list_items)
http_route("POST", "/api/items", create_item)

pub fn health() {
    return http_response(200, API_NAME)
}
"#;

const API_LIB_CONFIG: &str = r#"@version "1.0.0"
pub let API_NAME = "Kabootar API"
pub let API_VERSION = "0.1.0"
"#;

const API_LIB_ROUTES: &str = r#"@version "1.0.0"

pub fn list_items() {
    let rows = sql("SELECT id, name FROM items")
    return http_response(200, rows)
}

pub fn create_item() {
    sql("INSERT INTO items (name) VALUES ($1)", "new-item")
    return http_response(201, "created")
}
"#;

const API_CRUD_TOML: &str = r#"version = "0.1.0"
template = "api-crud"
entry = "main.kab"
port = 8080

[dependencies]
"#;

const API_CRUD_MAIN: &str = r#"@version "0.1.0"
import "http"
import "db"
import "users"

init_db()

http_route("GET", "/health", health)
http_route("GET", "/api/users", users_list)
http_route("POST", "/api/users", users_create)
http_route("PUT", "/api/users/:id", users_update)
http_route("DELETE", "/api/users/:id", users_delete)

pub fn health() {
    return http_response(200, "ok")
}
"#;

const API_CRUD_DB: &str = r#"@version "1.0.0"

pub fn init_db() {
    sql("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
}
"#;

const API_CRUD_USERS: &str = r#"@version "1.0.0"

pub fn users_list() {
    return http_response(200, sql("SELECT id, name, email FROM users"))
}

pub fn users_create() {
    sql("INSERT INTO users (name, email) VALUES ($1, $2)", "User", "user@example.com")
    return http_response(201, "created")
}

pub fn users_update() {
    sql("UPDATE users SET name = $1 WHERE id = $2", "Updated", 1)
    return http_response(200, "updated")
}

pub fn users_delete() {
    sql("DELETE FROM users WHERE id = $1", 1)
    return http_response(200, "deleted")
}
"#;

const SCIENCE_TOML: &str = r#"version = "0.1.0"
template = "science"
entry = "main.kab"

[dependencies]
"#;

const SCIENCE_MAIN: &str = r#"@version "0.1.0"
import "science"
import "data"
import "analysis"

let xs = SAMPLE_X
let ys = SAMPLE_Y
let report = analyze(xs, ys)

println(report)
"#;

const SCIENCE_LIB_DATA: &str = r#"@version "1.0.0"

pub let SAMPLE_X = [1, 2, 3, 4, 5]
pub let SAMPLE_Y = [2, 4, 5, 4, 5]
"#;

const SCIENCE_LIB_ANALYSIS: &str = r#"@version "1.0.0"
import "science"

pub fn analyze(xs, ys) {
    let mean_x = stat_mean(xs)
    let mean_y = stat_mean(ys)
    let fit = stat_linreg(xs, ys)
    return "mean_x=" + mean_x + " mean_y=" + mean_y + " linreg=" + fit
}
"#;

const FULLSTACK_TOML: &str = r#"version = "0.1.0"
template = "fullstack"
entry = "main.kab"
port = 8080

[dependencies]
"#;

const FULLSTACK_MAIN: &str = r#"@version "0.1.0"
import "http"
import "config"
import "api"

http_route("GET", "/", home)
http_route("GET", "/health", health)
register_api_routes()

pub fn home() {
    return http_response(200, APP_NAME + " — fullstack starter")
}

pub fn health() {
    return http_response(200, "ok")
}
"#;

const FULLSTACK_INDEX: &str = r#"<!DOCTYPE html>
<html lang="sv">
<head>
  <meta charset="UTF-8" />
  <link rel="stylesheet" href="/static/app.css" />
  <title>Kabootar Fullstack</title>
</head>
<body>
  <h1>Kabootar Fullstack</h1>
  <p>API: <code>GET /api/status</code></p>
</body>
</html>
"#;

const FULLSTACK_CSS: &str = r#"body { font-family: system-ui, sans-serif; margin: 2rem; }
h1 { color: #2563eb; }
"#;

const FULLSTACK_LIB_CONFIG: &str = r#"@version "1.0.0"
pub let APP_NAME = "Kabootar Fullstack"
pub let APP_VERSION = "0.1.0"
"#;

const FULLSTACK_LIB_API: &str = r#"@version "1.0.0"

pub fn register_api_routes() {
    http_route("GET", "/api/status", api_status)
}

pub fn api_status() {
    return http_response(200, "{\"status\":\"ready\"}")
}
"#;

const LIBRARY_TOML: &str = r#"version = "0.1.0"
template = "library"
entry = "demo.kab"

[dependencies]
"#;

const LIBRARY_DEMO: &str = r#"import "greet"
import "utils"

println(greet("world"))
println(double(21))
"#;

const LIBRARY_GREET: &str = r#"@version "1.0.0"

pub fn greet(name) {
    return "Hello, " + name
}
"#;

const LIBRARY_UTILS: &str = r#"@version "1.0.0"

pub fn double(n) {
    return n * 2
}
"#;

pub fn blueprint_by_id(id: &str) -> Option<&'static ProjectBlueprint> {
    let key = id.to_lowercase().replace('.', "-").replace('_', "-");
    BLUEPRINTS.iter().find(|b| b.id == key)
}

pub fn all_project_ids() -> Vec<&'static str> {
    BLUEPRINTS.iter().map(|b| b.id).collect()
}
