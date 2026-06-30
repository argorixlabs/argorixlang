[CmdletBinding()]
param(
    [ValidateSet("analyze", "verify", "figures", "tables", "paper", "qa", "test", "clean", "all")]
    [string]$Target = "all",
    [string]$InputRoot,
    [string]$CargoPath,
    [long]$SourceDateEpoch = 0,
    [switch]$VisualInspectionPassed
)

$ErrorActionPreference = "Stop"
$PaperRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$RepoRoot = (Resolve-Path (Join-Path $PaperRoot "..")).Path
$TmpRoot = Join-Path $PaperRoot "tmp"
$BuildRoot = Join-Path $TmpRoot "build"
$RenderRoot = Join-Path $TmpRoot "pdfs"
$FinalPdf = Join-Path $PaperRoot "argorixlang-preprint.pdf"
$TectonicVersion = "0.16.9"
$TectonicAsset = "tectonic-0.16.9-x86_64-pc-windows-msvc.zip"
$TectonicSha256 = "131a24604785a9600989a3d91225f597df52ac06f00aeffe86fd529f99ee5cdd"
$TectonicExeSha256 = "a0a9a5eaf1a940d9a615ad78d35225ca59420c7984576c6402fffb3e9fb05ceb"
$TectonicUrl = "https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%400.16.9/$TectonicAsset"

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "command failed ($LASTEXITCODE): $Program $($Arguments -join ' ')"
    }
}

function Resolve-InputRoot {
    if ($InputRoot) { return (Resolve-Path $InputRoot).Path }
    $local = Join-Path $RepoRoot "demo/argorix-chatbot-runtime/generated"
    if (Test-Path (Join-Path $local "request-*")) { return $local }
    $shared = Join-Path $RepoRoot "../../demo/argorix-chatbot-runtime/generated"
    return (Resolve-Path $shared).Path
}

function Get-Tectonic {
    $cacheBase = if ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA "ArgorixLang/tools/tectonic-$TectonicVersion"
    } else {
        Join-Path ([Environment]::GetFolderPath("UserProfile")) ".cache/argorixlang/tectonic-$TectonicVersion"
    }
    $exe = Join-Path $cacheBase "tectonic.exe"
    if (Test-Path $exe) {
        $exeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($exeHash -eq $TectonicExeSha256) {
            $version = (& $exe --version 2>&1 | Out-String).Trim()
            if ($LASTEXITCODE -eq 0 -and $version -eq "Tectonic $TectonicVersion") {
                return $exe
            }
        }
        Remove-Item -LiteralPath $exe -Force
    }
    New-Item -ItemType Directory -Force -Path $cacheBase | Out-Null
    $archive = Join-Path $cacheBase $TectonicAsset
    Invoke-WebRequest -Uri $TectonicUrl -OutFile $archive
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $TectonicSha256) {
        Remove-Item -LiteralPath $archive -Force
        throw "Tectonic archive checksum mismatch: expected $TectonicSha256, got $actual"
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $cacheBase -Force
    $exeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($exeHash -ne $TectonicExeSha256) {
        Remove-Item -LiteralPath $exe -Force
        throw "Extracted Tectonic executable checksum mismatch"
    }
    return $exe
}

function Find-Poppler {
    param([string]$Name)
    $command = Get-Command "$Name.exe" -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command -and $command.Source -notlike "*.cmd") { return $command.Source }
    if ($command -and $command.Source -like "*.cmd") {
        $deps = Split-Path (Split-Path $command.Source -Parent) -Parent
        $bundled = Join-Path $deps "native/poppler/Library/bin/$Name.exe"
        if (Test-Path $bundled) { return $bundled }
    }
    throw "$Name from Poppler is required (Codex bundled runtime or system installation)"
}

function Get-SourceEpoch {
    if ($SourceDateEpoch -ne 0) { return $SourceDateEpoch }
    $epoch = & python (Join-Path $PSScriptRoot "build_metadata.py") `
        stable-epoch --repo $RepoRoot
    if ($LASTEXITCODE -ne 0) { throw "failed to derive stable paper source epoch" }
    return [long]$epoch
}

function Invoke-Analyze {
    $input = Resolve-InputRoot
    Invoke-Checked python @(
        (Join-Path $PSScriptRoot "analyze_runtime.py"), "--input", $input,
        "--summary", (Join-Path $PaperRoot "data/runtime_summary.json"),
        "--sessions", (Join-Path $PaperRoot "data/sessions.csv"),
        "--events", (Join-Path $PaperRoot "data/event_counts.csv")
    )
}

function Invoke-Verify {
    $args = @(
        "-NoProfile", "-File", (Join-Path $PSScriptRoot "verify_runtime.ps1"),
        "-InputRoot", (Resolve-InputRoot),
        "-OutputPath", (Join-Path $PaperRoot "data/verification-results.json")
    )
    if ($CargoPath) { $args += @("-CargoPath", $CargoPath) }
    Invoke-Checked powershell $args
}

function Invoke-Figures {
    Invoke-Checked python @(
        (Join-Path $PSScriptRoot "generate_figures.py"),
        "--data", (Join-Path $PaperRoot "data"), "--output", (Join-Path $PaperRoot "figures")
    )
}

function Invoke-Tables {
    Invoke-Checked python @(
        (Join-Path $PSScriptRoot "render_tables.py"),
        "--data", (Join-Path $PaperRoot "data"), "--output", (Join-Path $PaperRoot "tables")
    )
}

function Invoke-Paper {
    Invoke-Checked python @((Join-Path $PSScriptRoot "check_manuscript.py"))
    $tectonic = Get-Tectonic
    New-Item -ItemType Directory -Force -Path $BuildRoot | Out-Null
    $stdout = Join-Path $BuildRoot "tectonic-stdout.log"
    $stderr = Join-Path $BuildRoot "tectonic-stderr.log"
    $previousSourceDateEpoch = $env:SOURCE_DATE_EPOCH
    $effectiveEpoch = Get-SourceEpoch
    $env:SOURCE_DATE_EPOCH = "$effectiveEpoch"
    $process = Start-Process -FilePath $tectonic -ArgumentList @(
        "-X", "compile", (Join-Path $PaperRoot "main.tex"), "--outdir", $BuildRoot,
        "--keep-logs", "--keep-intermediates", "--print"
    ) -Wait -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if ($null -eq $previousSourceDateEpoch) {
        Remove-Item Env:SOURCE_DATE_EPOCH
    } else {
        $env:SOURCE_DATE_EPOCH = $previousSourceDateEpoch
    }
    $output = @((Get-Content $stdout), (Get-Content $stderr))
    $output | Set-Content -Encoding UTF8 (Join-Path $BuildRoot "tectonic-output.log")
    $output | Write-Host
    if ($process.ExitCode -ne 0) { throw "Tectonic compilation failed" }
    $log = Get-Content -Raw (Join-Path $BuildRoot "main.log")
    $fatalPatterns = @(
        "LaTeX Warning:.*undefined", "Citation .* undefined", "Reference .* undefined",
        "There were undefined references", "Overfull \\[hv]box", "I couldn't open database file",
        "I found no \\bibdata command", "Emergency stop", "! LaTeX Error"
    )
    foreach ($pattern in $fatalPatterns) {
        if ($log -match $pattern) { throw "fatal TeX diagnostic matched: $pattern" }
    }
    $blg = Join-Path $BuildRoot "main.blg"
    if (Test-Path $blg) {
        $bibDiagnostics = (Get-Content $blg) |
            Where-Object { $_ -match "Warning--|error" } |
            Where-Object { $_ -ne "Warning--empty year in vazquez_atrust" }
        if ($bibDiagnostics) { throw "bibliography diagnostics remain in main.blg: $bibDiagnostics" }
    }
    $sourcePdf = Join-Path $BuildRoot "main.pdf"
    $temporaryPdf = Join-Path $PaperRoot ".argorixlang-preprint.$([guid]::NewGuid().ToString('N')).pdf.tmp"
    try {
        Copy-Item -LiteralPath $sourcePdf -Destination $temporaryPdf -Force
        Move-Item -LiteralPath $temporaryPdf -Destination $FinalPdf -Force
    } finally {
        if (Test-Path $temporaryPdf) {
            Remove-Item -LiteralPath $temporaryPdf -Force
        }
    }
}

function Invoke-Qa {
    if (-not (Test-Path $FinalPdf)) { Invoke-Paper }
    $effectiveEpoch = Get-SourceEpoch
    $pdftoppm = Find-Poppler "pdftoppm"
    $pdfinfo = Find-Poppler "pdfinfo"
    if (Test-Path $RenderRoot) { Remove-Item -LiteralPath $RenderRoot -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $RenderRoot | Out-Null
    Invoke-Checked $pdftoppm @("-png", "-r", "144", $FinalPdf, (Join-Path $RenderRoot "page"))
    $qaArgs = @(
        (Join-Path $PSScriptRoot "qa_pdf.py"), "--pdf", $FinalPdf,
        "--output", (Join-Path $PaperRoot "data/final-qa.json"),
        "--pdfinfo", $pdfinfo, "--engine", "Tectonic $TectonicVersion",
        "--source-date-epoch", "$effectiveEpoch",
        "--test-results", (Join-Path $TmpRoot "test-results.json")
    )
    if ($VisualInspectionPassed) { $qaArgs += "--visual-inspection-passed" }
    Invoke-Checked python $qaArgs
}

function Invoke-Tests {
    Invoke-Checked python @((Join-Path $PSScriptRoot "check_manuscript.py"))
    New-Item -ItemType Directory -Force -Path $TmpRoot | Out-Null
    $stdout = Join-Path $TmpRoot "pytest-stdout.log"
    $stderr = Join-Path $TmpRoot "pytest-stderr.log"
    $previousInputRoot = $env:ARGORIX_PAPER_INPUT_ROOT
    $env:ARGORIX_PAPER_INPUT_ROOT = Resolve-InputRoot
    $process = Start-Process -FilePath python -ArgumentList @(
        "-m", "pytest", (Join-Path $PaperRoot "tests"), "-q"
    ) -Wait -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if ($null -eq $previousInputRoot) {
        Remove-Item Env:ARGORIX_PAPER_INPUT_ROOT
    } else {
        $env:ARGORIX_PAPER_INPUT_ROOT = $previousInputRoot
    }
    $output = ((Get-Content -Raw $stdout) + (Get-Content -Raw $stderr))
    $output | Write-Host
    $passed = if ($output -match '(\d+) passed') { [int]$Matches[1] } else { 0 }
    $failed = if ($output -match '(\d+) failed') { [int]$Matches[1] } else { 0 }
    $result = [ordered]@{ passed = $passed; failed = $failed; total = $passed + $failed }
    $resultJson = $result | ConvertTo-Json -Compress
    $testResult = Join-Path $TmpRoot "test-results.json"
    $temporary = Join-Path $TmpRoot ".test-results.$([guid]::NewGuid().ToString('N')).json.tmp"
    [IO.File]::WriteAllText($temporary, $resultJson + [Environment]::NewLine)
    try {
        Move-Item -LiteralPath $temporary -Destination $testResult -Force
    } finally {
        if (Test-Path $temporary) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
    if ($process.ExitCode -ne 0) { throw "pytest failed with exit code $($process.ExitCode)" }
}

function Invoke-Clean {
    if (Test-Path $TmpRoot) { Remove-Item -LiteralPath $TmpRoot -Recurse -Force }
}

switch ($Target) {
    "analyze" { Invoke-Analyze }
    "verify" { Invoke-Verify }
    "figures" { Invoke-Figures }
    "tables" { Invoke-Tables }
    "paper" { Invoke-Paper }
    "qa" { Invoke-Qa }
    "test" { Invoke-Tests }
    "clean" { Invoke-Clean }
    "all" {
        Invoke-Analyze
        Invoke-Verify
        Invoke-Tables
        Invoke-Figures
        Invoke-Tests
        Invoke-Paper
        Invoke-Qa
    }
}
