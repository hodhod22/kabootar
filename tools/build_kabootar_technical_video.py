from __future__ import annotations

import asyncio
import re
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont
import edge_tts

ROOT = Path(r"C:\after-2026-06-03\new-kabootar-reserv\nova-interpreter")
OUT = ROOT / "assets" / "video" / "technical-rebuild"
FFMPEG = Path(r"C:\Users\hodho\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe")
W, H = 1920, 1080
BG, PANEL, INK, MUTED, GOLD, CYAN, GREEN, RED = "#09111f", "#12213a", "#f3f7ff", "#b8c4d8", "#f2ba4c", "#5bd6e8", "#6ed19c", "#ef8b86"

SCENES = [
("Kabootar: One Language, Whole Stack", "NOW + GOAL", ["A technical tour of the language, runtime, and journey toward self-hosting."],
"Kabootar is a fullstack programming language. Its ambition is simple to say and difficult to build: use one language, with one mental model, from user interface to server, database, operating system, and browser. This video is not a history lesson alone. It is a guided technical tour of how Kabootar is built, what works today, and which pieces are still on the road to becoming completely self-hosted."),
("The design direction", "DESIGN", ["JavaScript familiarity", "Rust-inspired safety", "C-sharp-inspired classes", "No prototype maze"],
"Kabootar starts from a practical question. Why should a developer need a different language, runtime, and programming model for each layer of a product? Its syntax deliberately feels familiar to JavaScript developers. But it removes several sources of surprise: there is no var, no prototype chain as the core object model, no eval as a language escape hatch, and no silent coercion as a design goal. Classes, structs, Result, Option, match, and explicit ownership tools are all part of the direction."),
("A small Kabootar program", "LANGUAGE", ["Functions use fn", "Blocks need no parentheses", "SQL is a first-class capability"],
'The basic language is compact. A function begins with fn. Blocks can be written without ceremony. The important point is not that the syntax is unusual; it is that the same source language can express business logic, call a database query, and build an interface. Kabootar aims to make the boundary between those tasks a library and runtime boundary, rather than a language boundary.'),
("From Nova to Kabootar", "TIMELINE", ["Nova: Rust-hosted interpreter", "28 June 2026: Kabootar created", "The goal became a full language platform"],
"The project began as Nova, a Rust-hosted interpreter. That early host was useful: it supplied a lexer, parser, evaluator, tests, and a place to experiment with language semantics. On June twenty-eighth, twenty twenty-six, the repository records the creation of the Kabootar computer language. The rename mattered because the project stopped being only an interpreter experiment. It became a platform project, with its own source extension, its own product runtime, and a plan to eventually build itself."),
("The product pipeline", "NOW", ["Kabootar source: .kab", "Self-host compiler", "Bytecode: .kbc or packed .kbcb", "Kab-VM executes the result"],
"The product path is the center of the architecture. A program written in dot kab is tokenized, parsed, and emitted by the self-hosted compiler. It becomes bytecode, either readable dot kbc or packed dot kbcb. The Kab virtual machine then executes it. This pipeline is important because it moves the language from interpreting a host data structure to owning an executable representation. The compiler and virtual machine increasingly live in Kabootar source themselves."),
("Bootstrap is not the destination", "HONEST STATUS", ["Rust host code remains for bootstrap", "Apps do not need Rust emit", "Goal: users run without rustc"],
"There is an important distinction between bootstrap and destination. Rust code in the source tree still helps start parts of the system and still contains host implementations. It is not presented as the final architecture. For applications, the project has a gate against emitting Rust. The longer-term goal is stronger: a user should build and run Kabootar without rustc. That last no-Rust distribution step is not complete today, so this video marks it as a goal rather than claiming it is already finished."),
("Self-hosting: compiler in .kab", "NOW / SUBSET", ["lexer.kab", "parser.kab", "emit.kab", "serialize.kab", "compile.kab"],
"Self-hosting means the language can understand and transform its own source. Kabootar has a compiler pipeline in the self host directory: lexer, parser, emitter, serializer, and compile facade are Kabootar files. It can cache and pack compiler artifacts too. This is a major milestone, but self-hosting is not a magic switch. The project carefully keeps its Rust bootstrap while the Kab compiler becomes stable and covers more of the language. That staged approach reduces the chance of losing a working build path."),
("Bytecode: the portable middle layer", "NOW", ["Stack-based instructions", "Cached compilation", "Text .kbc compatibility", "Packed KBCB v2 for efficient loading"],
"Bytecode gives Kabootar a portable middle layer. It is simpler than native machine code, but faster and more structured than repeatedly walking an abstract syntax tree. The virtual machine can run loops, objects, methods, functions, async work, destructuring, and increasingly rich language constructs from bytecode. The packed KBCB format avoids treating executable instructions only as lines of text. Caches make repeated work cheaper, while readable KBC files remain useful for inspection and compatibility."),
("Kab-VM: the runtime path", "NOW / SUBSET", ["Kab-only default execution", "Generators and async coverage", "Packed bytecode execution", "Host VM is transition debt"],
"The Kab-VM is the intended runtime path. The roadmap calls it kab-only by default: applications should execute through the Kab virtual machine rather than through the old Rust evaluator or host VM path. The project has deepened support for generators, yield, try and finally behavior, destructuring, optional access, and compound assignments on this route. Some host runtime machinery remains during the transition. The architectural rule is clear: when a Kab implementation exists, the product should use it instead of adding more host code."),
("JIT: compiling hot work", "NOW / SUBSET", ["Kab-JIT: native loop templates", "SSA, optimization, deoptimization experiments", "Cranelift host JIT remains bootstrap debt"],
"A just-in-time compiler, or JIT, takes frequently executed work and turns it into faster native operations. Kabootar has a Kab-JIT track for loop templates, intermediate representations, optimization experiments, deoptimization, tail calls, and memory-backed execution. There is also an older Cranelift-based host JIT. These are not the same thing. The host JIT is useful bootstrap debt; the product direction is to move JIT behavior into Kabootar itself. The honest label here is subset: the direction exists and is tested, while the migration continues."),
("AOT: ahead of time images", "NOW / SUBSET", ["AOT operation and relocation experiments", "Lazy boot graph: AOT first, JIT later", "Standalone Kab AOT process is still a goal"],
"Ahead-of-time compilation is the other performance path. Instead of waiting for a hot loop at runtime, AOT prepares code before the program starts. Kabootar has AOT image work for operations, relocations, profile-guided decisions, and lazy boot graphs, where the essential startup path is prepared early and other work can arrive later through JIT. The project does not claim that the whole process is already a standalone Kab AOT binary. That final capability is explicitly tracked as future work."),
("Memory model: two choices", "NOW / SUBSET", ["Default: garbage collection", "Manual mode: Owned, &, &mut", "Choose safety model for the workload"],
"Kabootar does not force every program into one memory discipline. The default programming model is garbage collected. That makes ordinary application, UI, and dynamic programming convenient. For systems-oriented work, the language also provides manual mode. In an at manual function, values can be Owned, borrowed immutably with ampersand, or borrowed mutably with ampersand mut. The point is not to imitate Rust line for line. It is to make a deliberate choice available when buffers, operating-system resources, or high-performance code need stricter lifetime control."),
("Default GC", "NOW / TRANSITION", ["Automatic memory for objects and UI", "Cycle guards and shared values today", "Kab nursery GC migration in progress"],
"The garbage-collected side is the ergonomic default. It is designed for code where allocation and sharing should not dominate every line: web-style objects, collections, user interfaces, and dynamic modules. Today, parts of that behavior still rely on host-side reference-counted values and cycle protection. In parallel, the project is implementing a nursery collector in Kabootar itself, including allocation, promotion, marking, sweeping, write barriers, thread-local allocation buffers, and concurrent-marking experiments. That is progress, but the host collector has not been deleted yet."),
("Manual ownership", "NOW / SUBSET", ["Affine Owned values", "Compile-time move checks", "Borrow and mutable borrow", "Leak lint; not full Rust lifetimes"],
"Manual ownership gives a different contract. An Owned resource has one responsible owner. Passing it can move it. A shared borrow can inspect it, while a mutable borrow is for controlled modification. The checker detects use after move and can warn about leaks. It intentionally does not attempt to be every part of Rust's lifetime system: borrowing across async and a full lifetime language are not the target. This smaller, explicit model is especially relevant to kOS buffers, network resources, and code that wants predictable resource release."),
("One language for UI, HTTP, and SQL", "NOW / HYBRID", ["KML and kDOM for UI", "HTTP routes and fetch", "In-process SQL with parameters", "No language boundary between layers"],
"The fullstack promise becomes concrete when the same Kabootar program can build a view, answer an HTTP route, and execute a parameterized SQL query. KML creates Kabootar markup. The HTTP layer provides routes and requests. The SQL layer offers a PostgreSQL-inspired in-process database API, including tables, filters, joins, transactions, and persistence work. That does not mean Kabootar is a PostgreSQL server. It means application code can use one language and one process for these concerns, avoiding a language switch simply to cross an architectural boundary."),
("Backend and database", "NOW / HYBRID", ["http_route()", "http_fetch_async()", "sql(\"SELECT ... $1\", values)", "WAL and MVCC exist in host engine"],
"On the backend, Kabootar supports routes as well as asynchronous fetch. The SQL API uses parameter values rather than string interpolation, and the host database engine includes serious concepts such as write-ahead logging, B trees, and multi-version concurrency control. The project is porting SQL and HTTP behavior into Kabootar in stages. So the API is already a meaningful same-language experience, while parts of the network, TLS, and database engine remain host capabilities during the migration. TLS is an active area of work, not a completed all-Kab replacement."),
("Two DOM worlds", "NOW / HYBRID", ["Host DOM: document, window", "Kabootar DOM: kdom", "KML produces Kabootar-native nodes", "Platform can be host, Kabootar, or hybrid"],
"Kabootar has a dual DOM design. In a browser host, code can interact with familiar document and window APIs. But Kabootar also defines its own DOM, called kDOM. KML is an XML-like markup language that produces those native Kabootar nodes. A platform choice can be host, Kabootar, or hybrid. This permits incremental use in the web while also giving the language a native UI model that does not depend forever on one browser's document implementation."),
("Style is KSS", "NOW / SUBSET", ["KSS: Kabootar style system", "Parse, selectors, themes, layout", "Product layout and paint still migrate from host"],
"Styling has its own layer too. KSS, Kabootar Style Sheets, provides parsing, selectors, themes, and layout-oriented work around kDOM. It is not simply a different name for browser CSS; it is part of the native UI stack. Some parsing and selector logic already lives in Kabootar. The layout and paint engine still has host implementation debt, so it would be inaccurate to call it a finished browser engine. The value of the split is architectural: markup, style, DOM, and rendering are designed to become owned by the platform."),
("Kv8 is not Google V8", "IMPORTANT", ["Kv8: Kabootar JS-subset runtime", "kDOM + KSS + .kv8 bundles", "V8: performance reference, not embedded engine"],
"The name Kv8 needs a clear warning. Kv8 is not Google V8 embedded under a new name. It is Kabootar’s own JavaScript-subset environment, connected to kDOM, KSS, and dot kv8 bundles. It includes Kabootar-side lexer, parser, evaluation, async and Promise subset work, and a React-like fiber experiment. Google V8 may appear as a performance reference point in discussions, but it is not the product engine. Keeping that distinction visible prevents the architecture from being overstated."),
("kOS and kbrowser", "NOW / SUBSET", ["kOS: policy, VFS, shell, windows", "kbrowser: navigation, tabs, Kab content", "Host compositor and system capabilities remain underneath"],
"The native platform effort extends beneath applications. kOS contains Kabootar-side policy, a virtual file system, shell, windows, explorer, and memory helpers. kbrowser contains navigation, tabs, and Kabootar content handling. Together they form a path toward an owned environment for Kabootar apps. They are not yet a replacement for a commercial desktop operating system or Chrome. Some system calls, filesystem access, layout, painting, and compositor behavior remain hosted. The project documents those limits because a transparent transition is more useful than a false claim of completion."),
("The standard library has layers", "NOW / TRANSITION", ["Natives: current host capabilities", "Built-in imports: math, http, crypto, science", "File modules: lib/std/*.kab and platform pillars"],
"The standard library is organized in layers. First are native operations, such as common math, arrays, strings, objects, JSON, and other primitives. Second are built-in imports like math, strings, HTTP, crypto, and science. Third are ordinary file modules, including the dot kab modules under lib slash std and larger pillars such as kDOM, kstyle, kOS, kbrowser, games, and data. The migration rule remains consistent: native host functionality is useful today, but the product goal is to move policy and implementations into Kabootar."),
("Milestones so far", "TIMELINE", ["v0.2: rename, docs, WASM, stubs", "v0.3: classes and KML", "v0.4–v0.5: database and HTTP", "v1.0+: OS, imports, LSP, security, science, bytecode"],
"The development timeline shows the system gaining rooms one by one. Version zero point two established the Kabootar identity, documentation, WebAssembly, and runtime stubs. Version zero point three added classes and KML. Versions zero point four and zero point five introduced database and backend capabilities. Version one point zero brought the fullstack claim into focus through a kernel and virtual filesystem, imports, SQL subset, and editor tooling. Later work added security, science, documentation assistance, packages, async I O, bytecode, self-hosting, and performance tracks."),
("The roadmap: self-reliance", "NEXT", ["SH16: no Rust emit for apps", "SH6: Kab-VM default", "SH17: Kab JIT", "SH18: Kab GC", "SH28: archive host tree when stable"],
"The roadmap is deliberately sequenced. First, make the self-host compiler dense and reliable. Then stop using Rust emit for applications. Make the Kab virtual machine the default. Deepen a Kab JIT and Kab garbage collector. Move loader, standard library, operating-system policy, SQL, crypto, HTTP, science, and browser work in turn. Only when the Kab implementation is stable does the project archive the Rust tree. That last milestone, often called SH twenty-eight, is the finish line for a product that runs without rustc."),
("Kabootar today", "SUMMARY", ["A real language and runtime path", "A fullstack architecture in active migration", "Self-hosting is underway", "Zero Rust is the target, not a claim"],
"Kabootar began as Nova, a hosted interpreter. It is becoming a fullstack language with a self-host compiler, bytecode, Kab virtual machine, two memory models, native markup and style systems, HTTP and SQL capabilities, and a growing operating-system and browser surface. Its strongest idea is not a single feature. It is ownership of the whole stack by the language itself. Today the result is real but hybrid. Tomorrow’s job is to make more of it truly Kabootar. That is the project: one language, many destinations, and eventually its own foundation.")
]

def font(size, bold=False):
    paths = [r"C:\Windows\Fonts\arialbd.ttf" if bold else r"C:\Windows\Fonts\arial.ttf",
             r"C:\Windows\Fonts\segoeuib.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf"]
    for p in paths:
        if Path(p).exists():
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()

def wrap(draw, text, fnt, width):
    words, lines, line = text.split(), [], ""
    for word in words:
        test = f"{line} {word}".strip()
        if draw.textlength(test, font=fnt) <= width:
            line = test
        else:
            lines.append(line); line = word
    if line: lines.append(line)
    return lines

def draw_slide(index, title, status, bullets):
    im = Image.new("RGB", (W,H), BG); d = ImageDraw.Draw(im)
    # decorative circuit and label
    for x in range(80, W, 160): d.line((x,0,x,H), fill="#0d1a2d", width=2)
    for y in range(100, H, 150): d.line((0,y,W,y), fill="#0d1a2d", width=2)
    d.rounded_rectangle((96,70,360,120), 20, fill=GOLD)
    d.text((122,82), f"KABOOTAR  /  {index:02d}", font=font(23, True), fill=BG)
    status_color = GREEN if "NOW" in status else GOLD if "GOAL" in status or "NEXT" in status else CYAN
    d.rounded_rectangle((W-410,70,W-95,120), 20, outline=status_color, width=3)
    d.text((W-380,82), status, font=font(22, True), fill=status_color)
    title_lines=wrap(d,title,font(64,True),1500)
    y=190
    for line in title_lines:
        d.text((120,y),line,font=font(64,True),fill=INK); y+=78
    # architecture panel
    top=max(370,y+30)
    d.rounded_rectangle((110,top,W-110,H-130), 34, fill=PANEL, outline="#21416b", width=3)
    n=len(bullets); gap=(W-300)//max(n,1); bx=150
    for num, bullet in enumerate(bullets):
        x=bx+num*gap
        d.ellipse((x,top+72,x+78,top+150),fill=GOLD if num%2==0 else CYAN)
        d.text((x+26,top+91),str(num+1),font=font(28,True),fill=BG)
        lines=wrap(d,bullet,font(29,True),gap-38)
        yy=top+180
        for l in lines:
            d.text((x,yy),l,font=font(29,True),fill=INK); yy+=40
        if num<n-1:
            d.line((x+gap-55,top+111,x+gap-10,top+111),fill=MUTED,width=5)
            d.polygon([(x+gap-10,top+111),(x+gap-27,top+100),(x+gap-27,top+122)],fill=MUTED)
    d.rectangle((0,H-100,W,H),fill="#060b14")
    d.text((120,H-76),"Kabootar Technical Architecture",font=font(24,True),fill=MUTED)
    d.text((W-355,H-76),"github.com/hodhod22/kabootar",font=font(21),fill=MUTED)
    return im

def sentence_chunks(text):
    return [s.strip() for s in re.split(r"(?<=[.!?])\s+", text) if s.strip()]

def timestamp(seconds):
    ms=round((seconds-int(seconds))*1000); seconds=int(seconds)
    return f"{seconds//3600:02}:{(seconds%3600)//60:02}:{seconds%60:02},{ms:03}"

def duration(path):
    result=subprocess.run([str(FFMPEG),"-i",str(path)],capture_output=True,text=True)
    match=re.search(r"Duration: (\d+):(\d+):(\d+\.\d+)", result.stderr)
    if not match: raise RuntimeError(f"Cannot read duration for {path}")
    return int(match.group(1))*3600+int(match.group(2))*60+float(match.group(3))

async def speak(index, text, out):
    communicate=edge_tts.Communicate(text, "en-US-JennyNeural", rate="-6%")
    await communicate.save(str(out))

async def speak_all(audio):
    await asyncio.gather(*(speak(i, scene[3], audio / f"{i:02d}.mp3") for i, scene in enumerate(SCENES, 1)))

def main():
    if not FFMPEG.exists(): raise FileNotFoundError(FFMPEG)
    if OUT.exists(): shutil.rmtree(OUT)
    slides=OUT/"slides"; audio=OUT/"audio"; parts=OUT/"parts"
    for p in (slides,audio,parts): p.mkdir(parents=True,exist_ok=True)
    (OUT/"narration-en.txt").write_text("\n\n".join(f"SCENE {i+1}: {s[3]}" for i,s in enumerate(SCENES)),encoding="utf-8")
    for i,(title,status,bullets,text) in enumerate(SCENES,1):
        draw_slide(i,title,status,bullets).save(slides/f"{i:02d}.png",quality=95)
    asyncio.run(speak_all(audio))
    subtitles=[]; start=0.0; entries=0; concat=[]
    for i,scene in enumerate(SCENES,1):
        au=audio/f"{i:02d}.mp3"; sec=duration(au)
        chunks=sentence_chunks(scene[3]); weights=[len(x.split()) for x in chunks]; total=sum(weights)
        local=0.0
        for chunk,w in zip(chunks,weights):
            length=sec*w/total
            subtitles.append(f"{entries+1}\n{timestamp(start+local)} --> {timestamp(start+local+length)}\n{chunk}\n")
            local+=length; entries+=1
        start+=sec
        out=parts/f"{i:02d}.mp4"
        subprocess.run([str(FFMPEG),"-y","-loop","1","-i",str(slides/f"{i:02d}.png"),"-i",str(au),
                        "-vf","format=yuv420p","-c:v","libx264","-preset","veryfast","-tune","stillimage","-crf","21",
                        "-c:a","aac","-b:a","160k","-shortest","-movflags","+faststart",str(out)],check=True)
        concat.append(f"file '{out.as_posix()}'")
    srt=OUT/"Kabootar-Technical-Explainer.en.srt"; srt.write_text("\n".join(subtitles),encoding="utf-8")
    listing=parts/"concat.txt"; listing.write_text("\n".join(concat),encoding="ascii")
    joined=OUT/"joined.mp4"
    subprocess.run([str(FFMPEG),"-y","-f","concat","-safe","0","-i",str(listing),"-c","copy",str(joined)],check=True)
    final=OUT/"Kabootar-Technical-Explainer-Jenny-Neural-Subtitled.mp4"
    escaped_srt = srt.as_posix().replace(":", r"\:")
    vf=f"subtitles='{escaped_srt}':force_style='FontName=Arial,FontSize=20,PrimaryColour=&H00FFFFFF,OutlineColour=&H00101625,BackColour=&H80000000,BorderStyle=4,Outline=2,Shadow=0,MarginV=45,Alignment=2'"
    subprocess.run([str(FFMPEG),"-y","-i",str(joined),"-vf",vf,"-c:v","libx264","-preset","veryfast","-crf","21","-c:a","copy","-movflags","+faststart",str(final)],check=True)
    print(f"FINAL={final}")
    print(f"DURATION_SECONDS={duration(final):.2f}")
    print(f"SCENES={len(SCENES)} SUBTITLES={entries}")

if __name__ == "__main__":
    main()
