<#
ESP-017: the native bootstrap on Windows x86-64, in PowerShell, which every
supported Windows ships. No Python, no Rust and no C compiler is needed
after the seed.

Two steps, which may run on different hosts:

    bootstrap/native-windows.ps1 seed      -Argorixc target/debug/argorixc.exe -Out target/native-windows
    bootstrap/native-windows.ps1 bootstrap -Out target/native-windows [-RequireNoCCompiler] [-RequireRustFreeHost]

`seed` is the last step with a C compiler (MSVC `cl`, stage N1 of
bootstrap/independence-policy.json). Stage0 writes the C of the compiler;
`cl` builds stage1 from it, and the runtime shim (spec/core/native-x86-64.md)
once, as a declared dependency. Stage1 then writes the seed: the compiler as
a COFF object for `x86_64-windows`. `shim.json` records the shim's sources,
flags, compiler and objects.

`bootstrap` needs only the linker (`link.exe`) and the C library the shim
links against, in a developer environment (LIB set). It checks that:

- native1, linked from the seed, writes the seed's object, manifest and
  diagnostics, byte for byte, and so do native2, linked from native1's
  object, and native3, linked from native2's;
- the three are byte-identical executables (the linker runs with /Brepro);
- each is an x86-64 console program that imports only KERNEL32.dll and
  SHELL32.dll: the C library is linked in statically, and nothing of Rust
  or of a C compiler's runtime is loaded;
- every case of the fixture suites, compiled natively by native2 and native3
  (the same object), linked and run, gives its expected exit status, output
  and files;
- with -RequireNoCCompiler no C compiler is on the path, and with
  -RequireRustFreeHost no Rust tool is.

It writes `native-report.json` with the host, the toolchain's identity and
digests, every artifact's digest and every case's result.

`cases` compiles, links, runs and checks the cases of any `cases.json`
natively with a compiler of the bootstrap:

    bootstrap/native-windows.ps1 cases -Out target/native-windows -Cases a/cases.json,b/cases.json

Exit status: 0 when every check passed, 1 otherwise, with the reason on
standard error.
#>
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("seed", "linker", "bootstrap", "cases")]
    [string]$Command,
    [string]$Argorixc,
    [string]$Out = "target/native-windows",
    [string[]]$Cases = @(),
    [string]$Compiler,
    [switch]$RequireNoCCompiler,
    [switch]$RequireRustFreeHost
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$BuildFile = Join-Path $Root "argorix.build"
$Utf8 = New-Object System.Text.UTF8Encoding $false
$BigBudget = 100000000
$CaseTimeoutSeconds = 10
$CompilerTimeoutSeconds = 600
$Suites = @(
    "tests/selfhost/runtime/cases.json",
    "tests/selfhost/stdlib/cases.json",
    "tests/selfhost/lexer/cases.json",
    "tests/selfhost/parser/cases.json",
    "tests/selfhost/check/cases.json",
    "tests/selfhost/ir/cases.json",
    "tests/selfhost/c/cases.json",
    "tests/selfhost/pipeline/cases.json",
    "tests/selfhost/stage1/cases.json",
    "conformance/core_c/regression/cases.json"
)
$ShimSources = @(
    "bootstrap/c/argorix_core_runtime.c",
    "bootstrap/native/argorix_native_shim.c",
    "bootstrap/native/argorix_native_host.c"
)
# The C profile for MSVC: C11, optimized, warnings as errors. C5105 is the
# Windows SDK's own headers under the conforming preprocessor.
$ShimFlags = @("/nologo", "/std:c11", "/O2", "/W3", "/WX", "/wd5105", "/c")
$Stage1Flags = @("/nologo", "/std:c11", "/O2", "/W3", "/WX", "/wd5105",
    "/DARGORIX_STEP_LIMIT=400000000000ULL", "/DARGORIX_BUFFER_LIMIT_BYTES=268435456U")
# The compiler's own limits, as the C profile gives them (spec/core/native-x86-64.md).
$CompilerLimits = @("steps 400000000000", "buffer-bytes 268435456")
$LinkFlags = @("/nologo", "/Brepro", "/subsystem:console")
$Libraries = @("libcmt.lib", "libucrt.lib", "libvcruntime.lib", "kernel32.lib", "shell32.lib")
# The linker and what it loads from its own directory: no compiler front end
# or code generator among them.
$LinkerFiles = '^(link\.exe|link\.exe\.config|mspdb140\.dll|mspdbcore\.dll|msobj140\.dll|mspdbsrv\.exe|mspdbst\.dll|tbbmalloc\.dll|cvtres\.exe|msvcp140.*\.dll|vcruntime140.*\.dll)$'
$Outputs = @("compiler.obj", "compiler.json", "diagnostics.txt")
$CCompilers = @("cl", "clang", "clang-cl", "gcc", "cc", "tcc", "icx")
$RustTools = @("rustc", "cargo", "rustup")

class BootstrapError : System.Exception {
    BootstrapError([string]$message) : base($message) {}
}

function Fail([string]$message) {
    throw [BootstrapError]::new($message)
}

# ------------------------------------------------------------------ files

function Sha256([string]$path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
}

function Relative([string]$path) {
    $full = [IO.Path]::GetFullPath($path)
    if ($full.StartsWith($Root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring($Root.Length + 1).Replace("\", "/")
    }
    $full
}

function Artifact([string]$path) {
    [ordered]@{ path = (Relative $path); bytes = (Get-Item -LiteralPath $path).Length; sha256 = (Sha256 $path) }
}

function Fresh([string]$directory) {
    if (Test-Path -LiteralPath $directory) {
        Remove-Item -LiteralPath $directory -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    (Resolve-Path -LiteralPath $directory).Path
}

function Write-Text([string]$path, [string]$text) {
    [IO.File]::WriteAllText($path, $text, $Utf8)
}

function Same-Bytes([string]$left, [string]$right) {
    $a = [IO.File]::ReadAllBytes($left)
    $b = [IO.File]::ReadAllBytes($right)
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) {
        if ($a[$i] -ne $b[$i]) { return $false }
    }
    $true
}

function Copy-Package([string[]]$files, [string]$target) {
    foreach ($file in $files) {
        $destination = Join-Path $target $file
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath (Join-Path $Root $file) -Destination $destination
    }
}

# The files `argorix.build` names: its root, then its modules.
function Build-Entries {
    $files = @()
    foreach ($line in [IO.File]::ReadAllLines($BuildFile)) {
        if ($line -match '^(root|module) (.+)$') {
            $files += $Matches[2]
        }
    }
    $files
}

# The compiler's package for a native build: its sources, and the build file
# with an object for Windows in place of the C.
function Compiler-Package([string]$target) {
    $package = Fresh $target
    Copy-Package (Build-Entries) $package
    $lines = @()
    foreach ($line in [IO.File]::ReadAllLines($BuildFile)) {
        if ($line -notmatch '^(c|manifest|diagnostics) ') {
            $lines += $line
        }
    }
    $lines += @("object compiler.obj", "target x86_64-windows") + $CompilerLimits + @("manifest compiler.json", "diagnostics diagnostics.txt")
    Write-Text (Join-Path $package "argorix.build") (($lines -join "`n") + "`n")
    $package
}

# What the fixture harness passes stage0 for a case: the root's directory,
# then `compiler/` and `stdlib/`, every `.argx` file but the root, each
# directory sorted by name.
function Locked-Set([string]$rootFile) {
    $directories = @(([IO.Path]::GetDirectoryName($rootFile)).Replace("\", "/"), "compiler", "stdlib")
    $seen = @{}
    $files = New-Object System.Collections.Generic.List[string]
    foreach ($directory in $directories) {
        if ($seen.ContainsKey($directory)) { continue }
        $seen[$directory] = $true
        $names = [string[]]@(Get-ChildItem -LiteralPath (Join-Path $Root $directory) -Filter "*.argx" -File | ForEach-Object { $_.Name })
        [Array]::Sort($names, [StringComparer]::Ordinal)
        foreach ($name in $names) {
            $file = "$directory/$name"
            if ($file -ne $rootFile) { $files.Add($file) }
        }
    }
    $files.ToArray()
}

# ------------------------------------------------------------------ processes

# One argument on a Windows command line: quoted when it holds a space or a
# quote, with the backslashes before a quote, or before the closing one,
# doubled.
function Quote([string]$argument) {
    if ($argument -eq "") { return '""' }
    if ($argument -notmatch '[\s"]') { return $argument }
    $escaped = [regex]::Replace($argument, '(\\*)"', '$1$1\"')
    $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
    '"' + $escaped + '"'
}

# Runs a program with a time limit; returns its exit status and output.
function Invoke-Tool([string]$program, [string[]]$arguments, [int]$seconds) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $program
    $info.Arguments = (($arguments | ForEach-Object { Quote $_ }) -join " ")
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    $process = [System.Diagnostics.Process]::Start($info)
    $stdout = $process.StandardOutput.BaseStream
    $stderr = $process.StandardError.BaseStream
    $outBuffer = New-Object System.IO.MemoryStream
    $errBuffer = New-Object System.IO.MemoryStream
    $outTask = $stdout.CopyToAsync($outBuffer)
    $errTask = $stderr.CopyToAsync($errBuffer)
    if (-not $process.WaitForExit($seconds * 1000)) {
        try { $process.Kill() } catch { }
        return [pscustomobject]@{ Exit = $null; Stdout = ""; Stderr = ""; TimedOut = $true }
    }
    $process.WaitForExit()
    $outTask.Wait()
    $errTask.Wait()
    [pscustomobject]@{
        Exit = $process.ExitCode
        Stdout = $Utf8.GetString($outBuffer.ToArray())
        Stderr = $Utf8.GetString($errBuffer.ToArray())
        TimedOut = $false
    }
}

# Runs a compiler over `package` into a fresh `build`; returns its result.
function Run-Compiler([string]$compiler, [string]$package, [string]$build) {
    $build = Fresh $build
    $result = Invoke-Tool $compiler @("--package-root", $package, "--read-budget", "$BigBudget", "--build-root", $build, "--write-budget", "$BigBudget") $CompilerTimeoutSeconds
    if ($result.TimedOut) { Fail "$(Relative $compiler) timed out" }
    $stdout = $result.Stdout.Trim()
    if ($result.Exit -ne 0 -or -not $stdout.StartsWith("ARGORIX_RESULT:")) {
        Fail "$(Relative $compiler) failed ($($result.Exit)): $stdout $($result.Stderr)"
    }
    $stdout.Substring("ARGORIX_RESULT:".Length)
}

function Find-Tool([string]$name) {
    $found = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { return $found.Source }
    $null
}

function Link-Program([string[]]$objects, [string]$executable, [string]$shim) {
    $linker = Find-Tool "link"
    if (-not $linker) { Fail "no link.exe on the path: run from a developer environment" }
    $shimObjects = @($ShimSources | ForEach-Object { Join-Path $shim ([IO.Path]::GetFileNameWithoutExtension($_) + ".obj") })
    $arguments = $LinkFlags + @("/out:$executable") + $objects + $shimObjects + $Libraries
    $result = Invoke-Tool $linker $arguments 300
    if ($result.TimedOut -or $result.Exit -ne 0) {
        Fail "link failed on $(Relative $executable): $($result.Stdout) $($result.Stderr)"
    }
    $arguments
}

# ------------------------------------------------------------------ cases

function Strip-One-Newline([string]$text) {
    if ($text.EndsWith("`n")) { return $text.Substring(0, $text.Length - 1) }
    $text
}

function Compile-Case([string]$compiler, [string]$rootFile, [string]$work, [string]$label) {
    $package = Fresh (Join-Path $work "package")
    $modules = Locked-Set $rootFile
    Copy-Package (@($rootFile) + $modules) $package
    $lines = @("argorix-build 1", "root $rootFile") + @($modules | ForEach-Object { "module $_" }) + @("object case.obj", "target x86_64-windows", "manifest case.json", "diagnostics case.txt")
    Write-Text (Join-Path $package "argorix.build") (($lines -join "`n") + "`n")
    $build = Join-Path $work "build-$label"
    $result = Run-Compiler $compiler $package $build
    if ($result -ne "0") {
        $diagnostics = Get-Content -Raw -LiteralPath (Join-Path $build "case.txt") -ErrorAction SilentlyContinue
        Fail "${root}: $(Relative $compiler) returned ${result}: $diagnostics"
    }
    Join-Path $build "case.obj"
}

# Runs a case's executable with its host roots and budgets, and checks its
# exit status, output and build files against the case.
function Execute-Case([string]$suite, $case, [string]$executable, [string]$work) {
    $arguments = @()
    $host_ = $null
    if ($case.PSObject.Properties.Name -contains "host") { $host_ = $case.host }
    $build = Join-Path $work "build"
    $writes = $false
    if ($host_) {
        $names = $host_.PSObject.Properties.Name
        if ($names -contains "package_root") {
            $arguments += @("--package-root", [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $suite) $host_.package_root)))
        }
        if ($names -contains "read_budget") {
            $arguments += @("--read-budget", "$($host_.read_budget)")
        }
        if ($names -contains "write_budget") {
            $writes = $true
            $build = Fresh $build
            if ($names -contains "build_dirs") {
                foreach ($directory in $host_.build_dirs) {
                    New-Item -ItemType Directory -Force -Path (Join-Path $build $directory) | Out-Null
                }
            }
            $arguments += @("--build-root", $build, "--write-budget", "$($host_.write_budget)")
        }
    }
    $execution = Invoke-Tool $executable $arguments $CaseTimeoutSeconds
    if ($execution.TimedOut) { return @("timed out after $CaseTimeoutSeconds s") }
    $failures = @()
    if ($execution.Exit -ne $case.expected_exit) {
        $failures += "exit $($execution.Exit) != $($case.expected_exit)"
    }
    $stdout = Strip-One-Newline $execution.Stdout
    if ($stdout -cne $case.expected_stdout) {
        $failures += "stdout '$stdout' != '$($case.expected_stdout)'"
    }
    $stderr = Strip-One-Newline $execution.Stderr
    if ($stderr -cne $case.expected_stderr) {
        $failures += "stderr '$stderr' != '$($case.expected_stderr)'"
    }
    if ($writes) {
        $expected = @{}
        if ($host_.PSObject.Properties.Name -contains "expected_build") {
            foreach ($entry in $host_.expected_build.PSObject.Properties) {
                $expected[$entry.Name] = $entry.Value
            }
        }
        $found = @{}
        foreach ($file in Get-ChildItem -LiteralPath $build -Recurse -File) {
            $found[$file.FullName.Substring($build.Length + 1).Replace("\", "/")] = [IO.File]::ReadAllBytes($file.FullName)
        }
        $wrong = @()
        foreach ($name in $expected.Keys) {
            $want = $Utf8.GetBytes([string]$expected[$name])
            if (-not $found.ContainsKey($name) -or [Convert]::ToBase64String($found[$name]) -ne [Convert]::ToBase64String($want)) {
                $wrong += $name
            }
        }
        $unexpected = @($found.Keys | Where-Object { -not $expected.ContainsKey($_) })
        if ($wrong.Count -gt 0 -or $unexpected.Count -gt 0) {
            $failures += "build files differ: missing or different [$($wrong -join ', ')], unexpected [$($unexpected -join ', ')]"
        }
    }
    $failures
}

function Cases-Of([string]$manifest) {
    (Get-Content -Raw -LiteralPath $manifest -Encoding UTF8 | ConvertFrom-Json).cases
}

# Every case of the fixture suites, compiled by native2 and native3, which
# must write the same object, then linked and run.
function Run-Suites([string]$out, [string]$native2, [string]$native3, [string]$shim) {
    $results = @()
    $failed = @()
    foreach ($manifest in $Suites) {
        $path = Join-Path $Root $manifest
        foreach ($case in (Cases-Of $path)) {
            $rootFile = Relative (Join-Path (Split-Path -Parent $path) $case.file)
            $work = Join-Path $out ("suite/" + (Split-Path -Leaf (Split-Path -Parent $path)) + "-" + $case.id)
            New-Item -ItemType Directory -Force -Path $work | Out-Null
            $failures = @()
            try {
                $from2 = Compile-Case $native2 $rootFile $work "native2"
                $from3 = Compile-Case $native3 $rootFile $work "native3"
                if (-not (Same-Bytes $from2 $from3)) {
                    $failures += "native2 and native3 write different objects"
                }
                $exe = Join-Path $work "case.exe"
                Link-Program @($from3) $exe $shim | Out-Null
                $failures += Execute-Case $path $case $exe $work
                $digest = Sha256 $from3
            } catch [BootstrapError] {
                $failures += $_.Exception.Message
                $digest = $null
            }
            $results += [ordered]@{ suite = $manifest; id = $case.id; object_sha256 = $digest; passed = ($failures.Count -eq 0) }
            if ($failures.Count -gt 0) {
                $failed += "$manifest $($case.id): $($failures -join '; ')"
            }
        }
    }
    if ($failed.Count -gt 0) {
        Fail ("cases failed with the native3 compiler:`n" + ($failed -join "`n"))
    }
    [ordered]@{ cases = $results.Count; passed = $results.Count; results = $results }
}

# ------------------------------------------------------------------ host

function Host-Record {
    $os = [Environment]::OSVersion
    [ordered]@{
        system = "Windows"
        version = $os.Version.ToString()
        machine = $env:PROCESSOR_ARCHITECTURE
        powershell = $PSVersionTable.PSVersion.ToString()
    }
}

function Tools-On-Path([string[]]$names) {
    $found = [ordered]@{}
    foreach ($name in $names) {
        $found[$name] = Find-Tool $name
    }
    $found
}

# The linker and the libraries a native program links against, with their
# digests, found as the linker finds them.
function Toolchain-Record {
    $linker = Find-Tool "link"
    $banner = (Invoke-Tool $linker @() 60).Stdout.Split("`n")[0].Trim()
    $libraryRecords = [ordered]@{}
    $directories = @()
    if ($env:LIB) { $directories = $env:LIB.Split(";") | Where-Object { $_ } }
    foreach ($library in $Libraries) {
        $libraryRecords[$library] = $null
        foreach ($directory in $directories) {
            $candidate = Join-Path $directory $library
            if (Test-Path -LiteralPath $candidate) {
                $libraryRecords[$library] = [ordered]@{ path = $candidate; sha256 = (Sha256 $candidate) }
                break
            }
        }
    }
    [ordered]@{
        linker = [ordered]@{ path = $linker; version = $banner; sha256 = (Sha256 $linker); flags = $LinkFlags }
        libraries = $libraryRecords
    }
}

# An executable's architecture, subsystem and the DLLs it imports, read from
# its PE headers.
function Inspect-Executable([string]$path) {
    $b = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($b, 60)
    if ($b[$pe] -ne 80 -or $b[$pe + 1] -ne 69 -or $b[$pe + 2] -ne 0 -or $b[$pe + 3] -ne 0) {
        Fail "$(Relative $path) is not a PE executable"
    }
    $machine = [BitConverter]::ToUInt16($b, $pe + 4)
    $sectionCount = [BitConverter]::ToUInt16($b, $pe + 6)
    $optionalSize = [BitConverter]::ToUInt16($b, $pe + 20)
    $optional = $pe + 24
    $magic = [BitConverter]::ToUInt16($b, $optional)
    $subsystem = [BitConverter]::ToUInt16($b, $optional + 68)
    # PE32+: the data directories start 112 bytes in; imports are the second.
    $importRva = [BitConverter]::ToUInt32($b, $optional + 120)
    $table = $optional + $optionalSize
    $offsetOf = {
        param([uint32]$rva)
        for ($i = 0; $i -lt $sectionCount; $i++) {
            $at = $table + $i * 40
            $size = [Math]::Max([BitConverter]::ToUInt32($b, $at + 8), [BitConverter]::ToUInt32($b, $at + 16))
            $address = [BitConverter]::ToUInt32($b, $at + 12)
            if ($rva -ge $address -and $rva -lt $address + $size) {
                return [int]($rva - $address + [BitConverter]::ToUInt32($b, $at + 20))
            }
        }
        -1
    }
    $imports = @()
    if ($importRva -ne 0) {
        $descriptor = & $offsetOf $importRva
        while ($descriptor -ge 0) {
            $nameRva = [BitConverter]::ToUInt32($b, $descriptor + 12)
            if ($nameRva -eq 0) { break }
            $name = & $offsetOf $nameRva
            $end = $name
            while ($b[$end] -ne 0) { $end++ }
            $imports += [Text.Encoding]::ASCII.GetString($b, $name, $end - $name)
            $descriptor += 20
        }
    }
    [ordered]@{ machine = ("0x{0:x4}" -f $machine); pe32_plus = ($magic -eq 523); subsystem = $subsystem; imports = $imports }
}

# The DLLs a native program may import: the system's own, with the C library
# linked in statically.
$AllowedImports = @("KERNEL32.dll", "SHELL32.dll")

function Check-Executable([string]$path) {
    $inspection = Inspect-Executable $path
    if ($inspection.machine -ne "0x8664" -or -not $inspection.pe32_plus) {
        Fail "$(Relative $path) is not an x86-64 PE32+ executable ($($inspection.machine))"
    }
    if ($inspection.subsystem -ne 3) {
        Fail "$(Relative $path) is not a console program (subsystem $($inspection.subsystem))"
    }
    $other = @($inspection.imports | Where-Object { $AllowedImports -notcontains $_ })
    if ($other.Count -gt 0) {
        Fail "$(Relative $path) imports $($other -join ', '), beyond $($AllowedImports -join ' and ')"
    }
    $inspection
}

function Write-Json([string]$path, $value) {
    Write-Text $path (($value | ConvertTo-Json -Depth 12) + "`n")
}

# ------------------------------------------------------------------ steps

function Step-Seed([string]$out) {
    if (-not $Argorixc) { Fail "seed needs -Argorixc (stage0)" }
    $cl = Find-Tool "cl"
    if (-not $cl) { Fail "no cl.exe on the path: run from a developer environment" }
    $out = Fresh $out
    # Stage0 writes the compiler's C.
    $stage1C = Join-Path $out "stage1.c"
    $emit = Invoke-Tool (Resolve-Path $Argorixc).Path @("--stdlib", (Join-Path $Root "stdlib"), "core-emit-c", (Join-Path $Root "compiler/main.argx"), "--output", $stage1C) $CompilerTimeoutSeconds
    if ($emit.Exit -ne 0) { Fail "stage0 failed: $($emit.Stderr)" }
    # The shim, once.
    $shim = Fresh (Join-Path $out "shim")
    $sources = @($ShimSources | ForEach-Object { Join-Path $Root $_ })
    $compile = Invoke-Tool $cl ($ShimFlags + @("/I", (Join-Path $Root "bootstrap/c"), "/Fo$shim\") + $sources) 600
    if ($compile.Exit -ne 0) { Fail "cl failed on the shim: $($compile.Stdout) $($compile.Stderr)" }
    # Stage1, from stage0's C.
    $stage1 = Join-Path $out "stage1.exe"
    $objects = Fresh (Join-Path $out "stage1-objects")
    $compile = Invoke-Tool $cl ($Stage1Flags + @("/I", (Join-Path $Root "bootstrap/c"), "/Fo$objects\", "/Fe$stage1", $stage1C, (Join-Path $Root "bootstrap/c/argorix_core_runtime.c"))) 900
    if ($compile.Exit -ne 0) { Fail "cl failed on stage1: $($compile.Stdout) $($compile.Stderr)" }
    # Stage1 writes the seed.
    $package = Compiler-Package (Join-Path $out "seed-package")
    $seed = Join-Path $out "seed"
    $result = Run-Compiler $stage1 $package $seed
    if ($result -ne "0") { Fail "stage1 returned $result on the compiler" }
    $banner = (Invoke-Tool $cl @() 60).Stderr.Split("`n")[0].Trim()
    $record = [ordered]@{
        schema_version = 1
        task = "ESP-017"
        target = "x86_64-windows"
        compiler = [ordered]@{ path = $cl; version = $banner; sha256 = (Sha256 $cl); flags = $ShimFlags }
        sources = @($ShimSources | ForEach-Object { Artifact (Join-Path $Root $_) })
        headers = @(@("bootstrap/c/argorix_core_runtime.h", "bootstrap/c/argorix_core_host.h") | ForEach-Object { Artifact (Join-Path $Root $_) })
        objects = @(Get-ChildItem -LiteralPath $shim -Filter "*.obj" | Sort-Object Name | ForEach-Object { Artifact $_.FullName })
        stage1 = [ordered]@{ c = (Artifact $stage1C); flags = $Stage1Flags; executable = (Artifact $stage1) }
        seed = @($Outputs | ForEach-Object { Artifact (Join-Path $seed $_) })
    }
    Write-Json (Join-Path $out "shim.json") $record
    "seed: $(Relative (Join-Path $seed 'compiler.obj')) $((Get-Item (Join-Path $seed 'compiler.obj')).Length) bytes"
}

# The linker, copied out of the developer environment with what it loads, so
# the bootstrap can run with it alone on the path. `linker.json` records the
# files, their digests and the library path.
function Step-Linker([string]$out) {
    $linker = Find-Tool "link"
    if (-not $linker) { Fail "no link.exe on the path: run from a developer environment" }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $target = Fresh (Join-Path $out "linker")
    Get-ChildItem -LiteralPath (Split-Path -Parent $linker) -File | Where-Object { $_.Name -match $LinkerFiles } | Copy-Item -Destination $target
    $record = [ordered]@{
        schema_version = 1
        task = "ESP-017"
        source = (Split-Path -Parent $linker)
        files = @(Get-ChildItem -LiteralPath $target -File | Sort-Object Name | ForEach-Object { [ordered]@{ name = $_.Name; bytes = $_.Length; sha256 = (Sha256 $_.FullName) } })
        lib = $env:LIB
    }
    Write-Json (Join-Path $out "linker.json") $record
    "linker: $(@($record.files).Count) files in $(Relative $target)"
}

function Step-Bootstrap([string]$out) {
    $out = (Resolve-Path -LiteralPath $out).Path
    $seed = Join-Path $out "seed"
    $shim = Join-Path $out "shim"
    foreach ($name in $Outputs) {
        if (-not (Test-Path -LiteralPath (Join-Path $seed $name))) { Fail "seed/$name is missing: run the seed step first" }
    }
    $cCompilers = Tools-On-Path $CCompilers
    $rustTools = Tools-On-Path $RustTools
    if ($RequireNoCCompiler -and @($cCompilers.Values | Where-Object { $_ }).Count -gt 0) {
        Fail ("this host is meant to have no C compiler, but has " + (($cCompilers.GetEnumerator() | Where-Object { $_.Value } | ForEach-Object { "$($_.Key) at $($_.Value)" }) -join ", "))
    }
    if ($RequireRustFreeHost -and @($rustTools.Values | Where-Object { $_ }).Count -gt 0) {
        Fail ("this host is meant to have no Rust tools, but has " + (($rustTools.GetEnumerator() | Where-Object { $_.Value } | ForEach-Object { "$($_.Key) at $($_.Value)" }) -join ", "))
    }
    $generations = @()
    $objectFrom = Join-Path $seed "compiler.obj"
    $compilers = @{}
    foreach ($generation in 1..3) {
        $name = "native$generation"
        $directory = Fresh (Join-Path $out $name)
        $executable = Join-Path $directory "argorixc.exe"
        $linkArguments = Link-Program @($objectFrom) $executable $shim
        $package = Compiler-Package (Join-Path $directory "package")
        $build = Join-Path $directory "build"
        $watch = [Diagnostics.Stopwatch]::StartNew()
        $result = Run-Compiler $executable $package $build
        $watch.Stop()
        if ($result -ne "0") { Fail "$name returned $result on its own sources" }
        foreach ($output in $Outputs) {
            if (-not (Same-Bytes (Join-Path $build $output) (Join-Path $seed $output))) {
                Fail "$name writes a different $output than the seed"
            }
        }
        $generations += [ordered]@{
            name = $name
            executable = (Artifact $executable)
            inspection = (Check-Executable $executable)
            object = (Artifact (Join-Path $build "compiler.obj"))
            seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 2)
        }
        $compilers[$name] = $executable
        $objectFrom = Join-Path $build "compiler.obj"
    }
    foreach ($pair in @(@("native1", "native2"), @("native2", "native3"))) {
        if (-not (Same-Bytes $compilers[$pair[0]] $compilers[$pair[1]])) {
            Fail "$($pair[0]) and $($pair[1]) are different executables"
        }
    }
    $suite = Run-Suites $out $compilers["native2"] $compilers["native3"] $shim
    $report = [ordered]@{
        schema_version = 1
        task = "ESP-017"
        target = "x86_64-windows"
        host = (Host-Record)
        c_compilers = $cCompilers
        rust_tools = $rustTools
        requirements = [ordered]@{ no_c_compiler = [bool]$RequireNoCCompiler; rust_free_host = [bool]$RequireRustFreeHost }
        toolchain = (Toolchain-Record)
        link_line = $LinkFlags + @("/out:<program>.exe", "<program>.obj") + @($ShimSources | ForEach-Object { [IO.Path]::GetFileNameWithoutExtension($_) + ".obj" }) + $Libraries
        seed = @($Outputs | ForEach-Object { Artifact (Join-Path $seed $_) })
        generations = $generations
        fixed_point = $true
        suites = $suite
    }
    Write-Json (Join-Path $out "native-report.json") $report
    "native1, native2 and native3 write the seed and are identical; $($suite.passed) of $($suite.cases) cases pass"
}

function Step-Cases([string]$out) {
    $out = (Resolve-Path -LiteralPath $out).Path
    $shim = Join-Path $out "shim"
    $compiler = $Compiler
    if (-not $compiler) { $compiler = Join-Path $out "native3/argorixc.exe" }
    $results = @()
    $failed = @()
    foreach ($manifest in $Cases) {
        $path = (Resolve-Path -LiteralPath $manifest).Path
        foreach ($case in (Cases-Of $path)) {
            $file = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $path) $case.file))
            $work = Join-Path $out ("cases/" + (Split-Path -Leaf (Split-Path -Parent $path)) + "-" + $case.id)
            New-Item -ItemType Directory -Force -Path $work | Out-Null
            $failures = @()
            try {
                if ($file.StartsWith($Root + "\", [StringComparison]::OrdinalIgnoreCase)) {
                    $object = Compile-Case $compiler (Relative $file) $work "native"
                } else {
                    $package = Fresh (Join-Path $work "package")
                    Copy-Item -LiteralPath $file -Destination (Join-Path $package "main.argx")
                    Write-Text (Join-Path $package "argorix.build") "argorix-build 1`nroot main.argx`nobject case.obj`ntarget x86_64-windows`nmanifest case.json`ndiagnostics case.txt`n"
                    $build = Join-Path $work "build"
                    $result = Run-Compiler $compiler $package $build
                    if ($result -ne "0") { Fail "the compiler returned ${result}: $(Get-Content -Raw -LiteralPath (Join-Path $build 'case.txt'))" }
                    $object = Join-Path $build "case.obj"
                }
                $exe = Join-Path $work "case.exe"
                Link-Program @($object) $exe $shim | Out-Null
                $failures += Execute-Case $path $case $exe $work
                $digest = Sha256 $object
            } catch [BootstrapError] {
                $failures += $_.Exception.Message
                $digest = $null
            }
            $results += [ordered]@{ cases = (Relative $path); id = $case.id; object_sha256 = $digest; passed = ($failures.Count -eq 0) }
            if ($failures.Count -gt 0) { $failed += "$(Relative $path) $($case.id): $($failures -join '; ')" }
        }
    }
    Write-Json (Join-Path $out "cases-report.json") ([ordered]@{ schema_version = 1; task = "ESP-017"; cases = $results.Count; results = $results })
    if ($failed.Count -gt 0) { Fail ("cases failed natively:`n" + ($failed -join "`n")) }
    "$($results.Count) cases pass natively"
}

try {
    switch ($Command) {
        "seed" { Step-Seed $Out }
        "linker" { Step-Linker $Out }
        "bootstrap" { Step-Bootstrap $Out }
        "cases" { Step-Cases $Out }
    }
    exit 0
} catch [BootstrapError] {
    [Console]::Error.WriteLine("error: " + $_.Exception.Message)
    exit 1
}
