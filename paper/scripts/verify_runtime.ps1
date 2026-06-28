[CmdletBinding()]
param(
    [string]$InputRoot = "demo/argorix-chatbot-runtime/generated",
    [string]$OutputPath = "paper/data/verification-results.json"
)

$ErrorActionPreference = "Stop"
$requiredArtifacts = @(
    "session.argx",
    "session.argbc.json",
    "session.trace.json",
    "session.security.json",
    "session.evidence.json"
)
$maxDiagnosticLength = 4096
$repositoryRoot = [IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot "..\..")
)

function Resolve-FromRepository([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }
    return [IO.Path]::GetFullPath((Join-Path $repositoryRoot $Path))
}

function Assert-NoReparsePoint([string]$Path, [string]$Description) {
    $item = Get-Item -LiteralPath $Path -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "$Description cannot be a symlink, junction, or reparse point: $Path"
    }
}

function Test-IsWithin([string]$Candidate, [string]$Root) {
    $candidateFull = [IO.Path]::GetFullPath($Candidate)
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
    return $candidateFull.StartsWith(
        $rootFull + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase
    )
}

function ConvertTo-SafeDiagnostic([string]$Text, [string]$Root) {
    if ($null -eq $Text) {
        return ""
    }
    $safe = $Text -replace "`e\[[0-9;?]*[ -/]*[@-~]", ""
    $safe = $safe.Replace($Root, "<input-root>")
    $safe = $safe -replace '(?im)(\bAuthorization\s*:\s*)(?:(?:Bearer|Basic|Token)\s+)?[^\s,\r\n]+', '$1<redacted>'
    $safe = $safe -replace '(?i)("(?:[^"]*[_-])?(?:token|api[_-]?key|password|secret)"\s*:\s*)"(?:[^"\\]|\\.)*"', '${1}"<redacted>"'
    $safe = $safe -replace "(?im)(\b(?:[A-Za-z][A-Za-z0-9_-]*[_-])?(?:token|api[_-]?key|password|secret)\s*[:=]\s*)(?:`"[^`"]*`"|'[^']*'|[^\s,;]+)", '$1<redacted>'
    $safe = $safe -replace '(?i)\bsk-(?:proj-)?[A-Za-z0-9_-]{16,}\b', '<redacted>'
    $safe = $safe -replace '\bgh[pousr]_[A-Za-z0-9]{20,}\b', '<redacted>'
    $safe = $safe -replace '\bxox[baprs]-[A-Za-z0-9-]{10,}\b', '<redacted>'
    $safe = $safe -replace '\bAKIA[0-9A-Z]{16}\b', '<redacted>'
    $safe = $safe -replace '\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b', '<redacted>'
    $safe = $safe -replace '[^\P{C}\r\n\t]', ''
    $safe = $safe.Trim()
    if ($safe.Length -gt $maxDiagnosticLength) {
        return $safe.Substring(0, $maxDiagnosticLength - 15) + "...[truncated]"
    }
    return $safe
}

$inputFull = Resolve-FromRepository $InputRoot
$outputFull = Resolve-FromRepository $OutputPath
if (-not (Test-Path -LiteralPath $inputFull -PathType Container)) {
    throw "Input root does not exist: $inputFull"
}
Assert-NoReparsePoint $inputFull "Input root"

if (
    $outputFull.Equals($inputFull, [StringComparison]::OrdinalIgnoreCase) -or
    (Test-IsWithin $outputFull $inputFull)
) {
    throw "Output path cannot equal or be inside the input root."
}

$cargo = "C:\Users\nanos\.cargo\bin\cargo.exe"
if (-not (Test-Path -LiteralPath $cargo -PathType Leaf)) {
    throw "Required Cargo executable not found: $cargo"
}

& $cargo build -q -p argorix-vm
if ($LASTEXITCODE -ne 0) {
    throw "argorix-vm build failed with exit code $LASTEXITCODE"
}

$binary = Join-Path $repositoryRoot "target\debug\argorix-vm.exe"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "Built argorix-vm binary not found: $binary"
}

$sessions = @(
    Get-ChildItem -LiteralPath $inputFull -Directory -Force |
        Where-Object {
            if (($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                return $false
            }
            foreach ($artifact in $requiredArtifacts) {
                $artifactPath = Join-Path $_.FullName $artifact
                if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
                    return $false
                }
                $artifactItem = Get-Item -LiteralPath $artifactPath -Force
                if (($artifactItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                    throw "Complete session contains a reparse-point artifact: $artifactPath"
                }
                if (-not (Test-IsWithin $artifactItem.FullName $inputFull)) {
                    throw "Artifact escapes input root: $artifactPath"
                }
            }
            return $true
        } |
        Sort-Object -Property Name
)

$records = foreach ($session in $sessions) {
    $evidencePath = Join-Path $session.FullName "session.evidence.json"
    $stdoutFile = [IO.Path]::GetTempFileName()
    $stderrFile = [IO.Path]::GetTempFileName()
    try {
        & $binary verify-evidence $evidencePath 1> $stdoutFile 2> $stderrFile
        $exitCode = $LASTEXITCODE
        $stdout = [IO.File]::ReadAllText($stdoutFile)
        $stderr = [IO.File]::ReadAllText($stderrFile)
    }
    catch {
        $exitCode = if ($LASTEXITCODE) { [int]$LASTEXITCODE } else { 1 }
        $stdout = if (Test-Path -LiteralPath $stdoutFile) {
            [IO.File]::ReadAllText($stdoutFile)
        } else { "" }
        $capturedError = if (Test-Path -LiteralPath $stderrFile) {
            [IO.File]::ReadAllText($stderrFile)
        } else { "" }
        $stderr = ($capturedError + [Environment]::NewLine + $_.Exception.Message).Trim()
    }
    finally {
        Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
    }

    [ordered]@{
        request_id = $session.Name
        exit_code = [int]$exitCode
        verified = ($exitCode -eq 0)
        evidence_path = "$($session.Name)/session.evidence.json"
        stdout = ConvertTo-SafeDiagnostic $stdout $inputFull
        stderr = ConvertTo-SafeDiagnostic $stderr $inputFull
    }
}

$outputDirectory = Split-Path -Parent $outputFull
[IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
$json = ConvertTo-Json -InputObject @($records) -Depth 4
[IO.File]::WriteAllText(
    $outputFull,
    $json + [Environment]::NewLine,
    [Text.UTF8Encoding]::new($false)
)

$verifiedCount = @($records | Where-Object verified).Count
$failedCount = @($records).Count - $verifiedCount
Write-Host "Recorded $(@($records).Count) checks: $verifiedCount verified, $failedCount failed."
