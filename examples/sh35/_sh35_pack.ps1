# Pack _stageE_N.json byte arrays into one raw blob for os_native_exec.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$outp = Join-Path $root "_code.bin"
$bj = Get-Content (Join-Path $root "_stageB.json") -Raw | ConvertFrom-Json
$nf2 = [int]$bj.nf2
$total = [int]$bj.total
$fs = [System.IO.File]::Create($outp)
try {
    for ($i = 0; $i -lt $nf2; $i++) {
        $p = Join-Path $root ("_stageE_{0}.json" -f $i)
        if (-not (Test-Path $p)) { throw "MISSING $p" }
        $j = Get-Content $p -Raw | ConvertFrom-Json
        $bytes = [byte[]]($j.bytes)
        $fs.Write($bytes, 0, $bytes.Length)
        if (($i % 50) -eq 0) { Write-Host ("pack " + $i + " off=" + $fs.Length) }
    }
} finally {
    $fs.Close()
}
$len = (Get-Item $outp).Length
Write-Host ("pack done bytes=" + $len + " expected=" + $total)
if ($len -ne $total) { throw "length mismatch $len vs $total" }
