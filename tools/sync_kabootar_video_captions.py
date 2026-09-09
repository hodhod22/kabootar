from __future__ import annotations

import asyncio
import importlib.util
import re
import subprocess
from pathlib import Path

import edge_tts

ROOT = Path(r"C:\after-2026-06-03\new-kabootar-reserv\nova-interpreter")
BASE = ROOT / "assets" / "video" / "technical-rebuild"
FFMPEG = Path(r"C:\Users\hodho\AppData\Local\Python\pythoncore-3.14-64\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe")

spec = importlib.util.spec_from_file_location("video", ROOT / "tools" / "build_kabootar_technical_video.py")
video = importlib.util.module_from_spec(spec)
spec.loader.exec_module(video)

def media_duration(path: Path) -> float:
    result = subprocess.run([str(FFMPEG), "-i", str(path)], capture_output=True, text=True)
    match = re.search(r"Duration: (\d+):(\d+):(\d+\.\d+)", result.stderr)
    if not match:
        raise RuntimeError(f"Cannot determine duration: {path}")
    return int(match.group(1)) * 3600 + int(match.group(2)) * 60 + float(match.group(3))

def stamp(seconds: float) -> str:
    milliseconds = round((seconds - int(seconds)) * 1000)
    seconds = int(seconds)
    return f"{seconds // 3600:02}:{seconds % 3600 // 60:02}:{seconds % 60:02},{milliseconds:03}"

async def sentence_markers(text: str):
    result = []
    speaker = edge_tts.Communicate(text, "en-US-JennyNeural", rate="-6%")
    async for event in speaker.stream():
        if event["type"] == "SentenceBoundary":
            result.append((event["offset"] / 10_000_000, event["duration"] / 10_000_000, event["text"]))
    return result

async def main():
    all_markers = await asyncio.gather(*(sentence_markers(scene[3]) for scene in video.SCENES))
    captions, segment_start, number = [], 0.0, 1
    for index, markers in enumerate(all_markers, 1):
        for offset, duration, text in markers:
            captions.append(f"{number}\n{stamp(segment_start + offset)} --> {stamp(segment_start + offset + duration)}\n{text}\n")
            number += 1
        segment_start += media_duration(BASE / "parts" / f"{index:02d}.mp4")

    srt = BASE / "Kabootar-Technical-Explainer.word-synced.en.srt"
    srt.write_text("\n".join(captions), encoding="utf-8")
    final = BASE / "Kabootar-Technical-Explainer-Jenny-Neural-Word-Synced.mp4"
    source = BASE / "joined.mp4"
    escaped_srt = srt.as_posix().replace(":", r"\:")
    style = "FontName=Arial,FontSize=20,PrimaryColour=&H00FFFFFF,OutlineColour=&H00101625,BackColour=&H80000000,BorderStyle=4,Outline=2,Shadow=0,MarginV=45,Alignment=2"
    subprocess.run([str(FFMPEG), "-y", "-i", str(source), "-vf",
                    f"subtitles='{escaped_srt}':force_style='{style}'",
                    "-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-c:a", "copy",
                    "-movflags", "+faststart", str(final)], check=True)
    print(f"FINAL={final}")
    print(f"CAPTIONS={len(captions)}")
    print(f"DURATION={media_duration(final):.2f}")

if __name__ == "__main__":
    asyncio.run(main())
