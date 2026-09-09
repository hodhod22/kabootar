"""Build a new, factual 4K Kabootar product-tour video."""
from __future__ import annotations

import asyncio
import shutil
import subprocess
from pathlib import Path

import edge_tts
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(r"C:\after-2026-06-03\new-kabootar-reserv\nova-interpreter")
OUT = ROOT / "assets" / "video" / "kabootar-world-tour-2026"
FFMPEG = Path(r"C:\Users\hodho\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe")
W, H, SECONDS = 3840, 2160, 30
BG, PANEL, INK, MUTED, CYAN, GOLD, GREEN, RED = "#050b16", "#0d1c32", "#f4f8ff", "#a9bad2", "#27c9ff", "#f4bc45", "#73e3ae", "#ff9a98"

SCENES = [
("KABOOTAR", "ONE LANGUAGE. MANY DESTINATIONS.", "NOW",
"""// a Kabootar source file
fn hello(name) {
    return `Hello, ${name}`
}

const message = hello("world")
println(message)""",
"Kabootar is a fullstack programming language. This is not a promise that every layer is already finished. It is a real language and runtime project with one clear direction: keep a single mental model from application code to the systems beneath it. In this tour, every bright label means available today. Every gold label means an active destination, not a completed claim."),
("THE LANGUAGE", "FAMILIAR SHAPE. STRICTER CHOICES.", "NOW",
"""const score = 42

fn welcome(name) {
    return `Welcome, ${name}`
}

// no var; no implicit "1" + 2
println(welcome("Kabootar"))""",
"The surface is intentionally approachable for JavaScript developers. Functions use fn. Values use let and const. Template strings are familiar. But Kabootar removes var and does not make silent string-and-number coercion part of normal programming. The point is not novelty. The point is readable code with fewer accidental surprises as projects grow."),
("MODELING DATA", "CLASSES, STRUCTS, MATCH, RESULT.", "NOW",
"""class Account {
    fn init(name) { this.name = name }
    fn greet() { return `Hi ${this.name}` }
}

let label = match status {
    "ready" => "Launch",
    _ => "Wait"
}""",
"Kabootar supports classes with this, structs, enums, Result, Option, and match. The object model is deliberately not built around JavaScript's prototype chain. Pattern matching and explicit result values make branching visible, while classes keep ordinary application modeling concise. Those are language features you can use now, not a separate framework."),
("FRONTEND", "KML + kDOM.", "NOW / HYBRID",
"""let ui = kml(
  "<div class=\\"app\\">" +
  "<h1>Kabootar</h1>" +
  "<p>One language. Whole stack.</p>" +
  "</div>"
)
let html = kdom_render(ui)
println(html)""",
"For interface structure, Kabootar has KML, an XML-like markup language, and kDOM, its own document model. A program can also work with host browser APIs when that is the right deployment target. This dual-platform design is important: native Kabootar UI is real work in the project, while browser layout and paint still include host-side implementation during the transition."),
("STYLE", "KSS IS THE NATIVE UI DIRECTION.", "NOW / SUBSET",
"""<div class="hero">
  <h1>Build across the stack</h1>
  <button>Explore</button>
</div>

// KML creates kDOM nodes
// KSS provides selectors and themes""",
"KSS, or Kabootar Style Sheets, is the style layer around kDOM. It includes parsing, selectors, themes, and layout-oriented work. It should not be described as a finished replacement for every browser layout engine: layout and paint retain host debt today. The important engineering direction is ownership of markup, style, DOM, and eventually rendering by the platform."),
("VISUAL COMPUTING", "WEBGL 3D + CANVAS 2D.", "NOW",
"""import "game/render"
platform_use("kabootar")

let hero = game_surface_create_3d(1280, 720)
let gl = hero["gl"]
gl.lookAt(0, 0, 3.0, 0, 0, 0, 0, 1, 0)
setColor(gl, 0.20, 0.82, 1.00, 1.00)
drawMesh(createMesh(gl, vertices))
hero.present()""",
"This is Kabootar code from the graphics showcase. One source file opens a three-dimensional surface, positions a camera, creates mesh data, colours it, and draws it. The same program can also use Canvas for interface overlays. That makes visual computing a practical demonstration, rather than only a roadmap slide."),
("CANVAS HUD", "2D INTERFACE IN THE SAME PROGRAM.", "NOW",
"""let hud = canvas_create(1280, 720)
hud.fillStyle = "#09111f"
hud.fillRect(0, 0, 1280, 720)
hud.font = "bold 64px Arial"
hud.fillStyle = "#f3f7ff"
hud.fillText("KABOOTAR", 112, 150)
let pixels = canvas_to_pixels(hud)""",
"Canvas handles crisp text, diagrams, charts, and interaction layers. WebGL handles mesh geometry and GPU-oriented effects. Kabootar exposes both through the same language surface. A mature production game editor is still a roadmap objective, so this video shows the existing APIs and demo rather than claiming a completed Unity replacement."),
("BACKEND", "ROUTES WITHOUT A LANGUAGE SWITCH.", "NOW / HYBRID",
"""import "http"

fn list_users() { return ok("[]") }
fn create_user() { return created(req_body) }

route_get("/api/users", list_users)
route_post("/api/users", create_user)
http_body(request_get("/api/users"))""",
"On the backend, Kabootar provides routes, request handling, response helpers, and asynchronous fetch APIs. This example registers handlers and exercises an in-process request. Network and TLS functionality are being moved toward Kabootar in stages. The API is useful today; calling every underlying network component Kabootar-native would be inaccurate."),
("DATABASE", "SQL IN PROCESS.", "NOW / HYBRID",
"""sql("CREATE TABLE IF NOT EXISTS users " +
    "(id SERIAL PRIMARY KEY, name TEXT NOT NULL)")

sql("INSERT INTO users (name) VALUES ($1)", "Ada")
let rows = sql(
    "SELECT name FROM users WHERE id = $1", 1
)""",
"Kabootar SQL is an in-process database API. It supports tables, parameter values, joins, transactions, indexes, and persistence work including WAL. Parameter values matter: the application passes data separately from the query text. It is PostgreSQL-inspired, not a PostgreSQL server. The current SQL engine still has host implementation during the self-host migration."),
("SECURITY", "CRYPTO IS A LANGUAGE SURFACE.", "NOW / TRANSITION",
"""import "crypto"

// cryptographic APIs are available
// TLS migration is actively in progress

// policy belongs in Kabootar;
// host components remain until replaced""",
"Kabootar exposes cryptographic functionality through its standard-library surface. The roadmap's current wave is moving crypto and TLS behavior deeper into Kabootar. That wording matters. Rustls and other host components have not yet been removed, so the accurate story is a working security surface and an active transition toward a Kab-owned implementation."),
("SCIENCE", "NUMERICS + MACHINE LEARNING SUBSET.", "NOW",
"""import "science"
import "science/nd"
import "science/ml"

let X = [[1.0], [2.0], [3.0], [4.0]]
let Y = [3.0, 5.0, 7.0, 9.0]
let params = linregStep([0.0, 0.0], X[0], Y[0], 0.05)
let x = solve(from([[2.0, 0.0], [0.0, 2.0]]), from([4.0, 6.0]))""",
"Kabootar includes a science surface for math, numerical arrays, and machine-learning-oriented work. Here, the repository's own example imports science modules, performs a linear-regression step, and solves a small system. It is a meaningful subset and an expanding foundation. It is not yet a claim to replace Python's complete scientific and AI ecosystem."),
("DEFAULT MEMORY", "GC FOR PRODUCT CODE.", "NOW / TRANSITION",
"""let app = {
    title: "Kabootar",
    features: ["UI", "HTTP", "SQL"]
}

app.features.push("science")
println(app.title)

// GC is the default memory model""",
"The default memory model is garbage collected, which keeps normal application, UI, collection, and dynamic code comfortable. Presently, parts of that behavior are still host-side reference-counted values with cycle protection. A Kabootar nursery collector is under active development. The dual approach is real, but the all-Kab garbage collector is not yet the finished destination."),
("MANUAL MEMORY", "OWNED, BORROWED, EXPLICIT.", "NOW",
"""@manual

fn peek(b: &Owned) { owned_read(b, 0, 1) }
fn poke(b: &mut Owned) { owned_write(b, 0, [1]) }
fn take(b: Owned) { drop(b) }

let buffer = owned_alloc(8, "packet")
peek(&buffer)
poke(&mut buffer)
take(buffer)""",
"For buffers, operating-system resources, and systems-oriented code, manual mode provides affine Owned values, shared borrows with ampersand, and exclusive mutable borrows with ampersand mut. The checker detects use after move and includes a leak lint. It is intentionally smaller than Rust's full lifetime language, especially around asynchronous borrowing. That focused contract is a design choice."),
("SELF-HOSTING", ".KAB COMPILES .KAB.", "NOW / HYBRID",
""".kab source
    ↓
self_host/compile.kab
    ↓
.kbc bytecode  /  .kbcb packed bytecode
    ↓
self_host/vm.kab
    ↓
Kab-VM execution""",
"Kabootar is self-hosting in a practical, staged sense. Its compiler pipeline includes Kabootar lexer, parser, emitter, serializer, and compile code. Applications compile to readable KBC or packed KBCB bytecode, then execute on Kab-VM. Rust still starts parts of the system, so self-hosting is a real milestone, not a reason to pretend the bootstrap has disappeared."),
("BYTECODE", "PORTABLE EXECUTION.", "NOW",
"""// Source becomes bytecode:
const total = 0
for let i = 0; i < 10; i += 1 {
    total += i
}

// .kbc stays inspectable
// .kbcb packs instructions for loading""",
"Bytecode gives Kabootar a structured middle layer between source and execution. The project supports readable KBC files and packed KBCB artifacts, with Kab-VM as the intended application runtime path. This is how more language features move away from the original tree-walking interpreter and toward an executable representation Kabootar can increasingly own."),
("JIT", "HOT WORK, NATIVE DIRECTION.", "NOW / SUBSET",
"""hot loop
    ↓
Kab intermediate representation
    ↓
optimization experiments
    ↓
native loop templates
    ↓
deoptimization when assumptions fail""",
"A just-in-time compiler makes frequently executed work faster by compiling it while a program runs. Kabootar has a JIT track with loop templates, intermediate representations, optimization, deoptimization, and native execution experiments. There is also an older host JIT. The truthful label is subset: this is substantial work, but not yet a universal production JIT that replaces every host component."),
("AOT", "PREPARE BEFORE STARTUP.", "NOW / SUBSET",
"""build time
    ↓
AOT operations + relocations
    ↓
prepared boot image
    ↓
fast initial path
    ↓
JIT may optimize later""",
"Ahead-of-time work prepares execution before a program starts. Kabootar has AOT image, relocation, profile-guided, and lazy boot-graph work. Combined with a JIT direction, that enables a design where essential startup work is prepared early and hot paths improve later. A standalone all-Kab AOT process remains a tracked goal, not a delivered final binary format."),
("Kv8", "KABOOTAR'S JS-SUBSET — NOT GOOGLE V8.", "NOW / SUBSET",
"""// Kv8 bundle direction
let label = "Kabootar"
let ready = true

// connected to kDOM and KSS
// own lexer, parser, evaluation subset

// Kv8 ≠ Google V8""",
"Kv8 needs a precise explanation. It is Kabootar's own JavaScript-subset environment for KDOM, KSS, and Kv8 bundles. It is not Google V8 under another name, and it is not a complete JavaScript engine. The project includes parser, evaluator, Promise-subset, and fiber-direction work, all framed as an evolving native platform surface."),
("SYSTEM DIRECTION", "kOS + kbrowser.", "NOW / SUBSET",
"""kOS
  policy · virtual filesystem · shell · windows

kbrowser
  navigation · tabs · Kab content handling

host
  system calls · compositor · layout debt""",
"Kabootar also reaches beneath normal applications. kOS contains policy, virtual filesystem, shell, window, and explorer direction. kbrowser contains navigation, tabs, and Kabootar content handling. Neither should be called a finished commercial desktop operating system or Chrome replacement. They are platform layers where Kabootar increasingly owns policy while host system and compositor work remains underneath."),
("DEVELOPER EXPERIENCE", "FORMAT, TEST, DOC, EDITOR.", "NOW / TRANSITION",
"""kabootar fmt main.kab
kabootar test
kabootar doc
kabootar compile main.kab

main.kab → main.kbc / main.kbcb

// VS Code support is included
// test tooling continues to migrate""",
"The project also includes developer tooling: a REPL, formatting, documentation, tests, registry work, compilation commands, and editor support. The long-term aim is a Kabootar-owned command line and test path. Today, parts of the runner and bootstrap remain in Rust. Tooling exists; a zero-Rust distribution is the final gate, not a present-tense slogan."),
("THE ROADMAP", "FROM HYBRID TO SELF-RELIANT.", "GOAL",
"""SH16  no Rust emit for applications
SH6   Kab-VM default execution
SH17  JIT in Kabootar
SH18  GC in Kabootar
SH20–24 stdlib, OS, SQL, crypto, HTTP
SH25–27 tools, science, browser/game
SH28  archive host tree when stable""",
"The roadmap is deliberately ordered. First, make the self-host compiler reliable. Then make Kab-VM the default, deepen JIT and GC in Kabootar, move loader and library layers, and continue through operating system policy, SQL, crypto, HTTP, science, browser, game, and tooling. Only when those paths are stable is the host tree archived. That is the destination."),
("KABOOTAR", "BUILD WHAT THE LANGUAGE CAN OWN.", "NOW + GOAL",
"""// One source language
// UI · graphics · backend · SQL · science
// bytecode · VM · GC · ownership
// platform direction

fn future() {
    return "Kabootar"
}""",
"Kabootar is already a real language with an unusually wide surface: UI, graphics, HTTP, SQL, science, bytecode, Kab-VM, and two memory models. Its boldest claim is also the future-facing one: move more of the stack into the language itself. The honest conclusion is not that every dependency is gone. It is that Kabootar is building the foundation to own them."),
]

def font(size: int, bold: bool = False):
    path = Path(r"C:\Windows\Fonts\segoeuib.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf")
    return ImageFont.truetype(path, size)

def mono(size: int):
    return ImageFont.truetype(r"C:\Windows\Fonts\consola.ttf", size)

def wrap(draw, text, fnt, width):
    lines, line = [], ""
    for word in text.split():
        candidate = f"{line} {word}".strip()
        if draw.textlength(candidate, font=fnt) <= width: line = candidate
        else: lines.append(line); line = word
    return lines + ([line] if line else [])

def card(index, title, subtitle, status, code, narration):
    im = Image.new("RGB", (W, H), BG); d = ImageDraw.Draw(im)
    for x in range(0, W, 160): d.line((x, 0, x, H), fill="#081429", width=2)
    for y in range(0, H, 160): d.line((0, y, W, y), fill="#081429", width=2)
    d.rounded_rectangle((150, 110, 690, 190), 28, fill=CYAN)
    d.text((195, 128), f"KABOOTAR / {index:02}", font=font(32, True), fill=BG)
    color = GOLD if "GOAL" in status else GREEN if "NOW" in status else CYAN
    d.rounded_rectangle((W-670, 110, W-150, 190), 28, outline=color, width=4)
    d.text((W-620, 128), status, font=font(30, True), fill=color)
    d.text((155, 270), title, font=font(110, True), fill=INK)
    d.text((160, 405), subtitle, font=font(38, True), fill=CYAN)
    d.rounded_rectangle((150, 510, 2300, 1660), 40, fill="#071426", outline="#1d4f75", width=4)
    d.text((225, 585), "KABOOTAR", font=font(26, True), fill=GOLD)
    y = 680
    for line in code.splitlines():
        d.text((225, y), line, font=mono(42), fill=INK if not line.strip().startswith("//") else MUTED)
        y += 62
    d.rounded_rectangle((2420, 510, W-150, 1660), 40, fill=PANEL, outline="#1d4f75", width=4)
    d.text((2490, 585), "ON SCREEN", font=font(26, True), fill=GOLD)
    y = 690
    for line in wrap(d, narration, font(42), 1120):
        d.text((2490, y), line, font=font(42), fill=INK)
        y += 65
    d.rectangle((0, 1940, W, H), fill="#030711")
    d.text((155, 2015), "KABOOTAR — ONE LANGUAGE. MANY DESTINATIONS.", font=font(30, True), fill=MUTED)
    d.text((W-1050, 2015), "FACTUAL PRODUCT TOUR · 2026", font=font(30, True), fill=MUTED)
    return im

async def tts(text, out):
    await edge_tts.Communicate(text, "en-US-JennyNeural", rate="-4%").save(str(out))

def main():
    if not FFMPEG.exists(): raise FileNotFoundError(FFMPEG)
    if OUT.exists(): shutil.rmtree(OUT)
    slides, audio, parts = OUT/"slides", OUT/"audio", OUT/"parts"
    for p in slides, audio, parts: p.mkdir(parents=True, exist_ok=True)
    (OUT/"narration-en.txt").write_text("\n\n".join(f"{i+1}. {s[4]}" for i, s in enumerate(SCENES)), encoding="utf-8")
    for i, scene in enumerate(SCENES, 1):
        card(i, *scene).save(slides/f"{i:02}.png")
        asyncio.run(tts(scene[4], audio/f"{i:02}.mp3"))
        subprocess.run([str(FFMPEG), "-y", "-loop", "1", "-i", str(slides/f"{i:02}.png"), "-i", str(audio/f"{i:02}.mp3"),
                        "-t", str(SECONDS), "-vf", "format=yuv420p", "-af", f"apad=pad_dur={SECONDS}",
                        "-c:v", "libx264", "-preset", "veryfast", "-crf", "17", "-maxrate", "12M", "-bufsize", "24M",
                        "-r", "25", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", str(parts/f"{i:02}.mp4")], check=True)
    listing = parts/"concat.txt"
    listing.write_text("\n".join(f"file '{(parts/f'{i:02}.mp4').as_posix()}'" for i in range(1, len(SCENES)+1)), encoding="ascii")
    final = OUT/"Kabootar-World-Tour-2026-4K-English.mp4"
    subprocess.run([str(FFMPEG), "-y", "-f", "concat", "-safe", "0", "-i", str(listing), "-c", "copy", str(final)], check=True)
    print(f"FINAL={final}")

if __name__ == "__main__":
    main()
