param([string]$Repository = (Split-Path $PSScriptRoot -Parent))

# Read-only inventory. Writes JSON to stdout; never builds or executes product binaries.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Repository).Path
function Relative([string]$Path) {
    [IO.Path]::GetRelativePath($root, [IO.Path]::GetFullPath($Path)).Replace('\', '/')
}
function Read-PEImports([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 64 -or [BitConverter]::ToUInt16($bytes, 0) -ne 0x5a4d) { throw "Not PE: $Path" }
    $pe = [BitConverter]::ToInt32($bytes, 0x3c)
    if ([BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550) { throw "Invalid PE: $Path" }
    $sectionsCount = [BitConverter]::ToUInt16($bytes, $pe + 6)
    $optionalSize = [BitConverter]::ToUInt16($bytes, $pe + 20)
    $optional = $pe + 24
    $magic = [BitConverter]::ToUInt16($bytes, $optional)
    $dataOffset = if ($magic -eq 0x20b) { 112 } elseif ($magic -eq 0x10b) { 96 } else { throw 'Unsupported PE format' }
    $sections = @()
    for ($i = 0; $i -lt $sectionsCount; $i++) {
        $offset = $optional + $optionalSize + 40 * $i
        $sections += [pscustomobject]@{
            rva = [BitConverter]::ToUInt32($bytes, $offset + 12)
            size = [Math]::Max([BitConverter]::ToUInt32($bytes, $offset + 8), [BitConverter]::ToUInt32($bytes, $offset + 16))
            raw = [BitConverter]::ToUInt32($bytes, $offset + 20)
        }
    }
    $rvaToOffset = {
        param([uint32]$rva)
        foreach ($section in $sections) {
            if ($rva -ge $section.rva -and $rva -lt ($section.rva + $section.size)) { return [int]($section.raw + $rva - $section.rva) }
        }
        throw "Unmapped RVA: $rva"
    }
    $importRva = [BitConverter]::ToUInt32($bytes, $optional + $dataOffset + 8)
    $delayRva = [BitConverter]::ToUInt32($bytes, $optional + $dataOffset + 13 * 8)
    $names = @()
    if ($importRva -ne 0) {
        $offset = & $rvaToOffset $importRva
        for ($i = 0; $i -lt 4096; $i++) {
            $descriptor = $offset + 20 * $i
            $nameRva = [BitConverter]::ToUInt32($bytes, $descriptor + 12)
            if ($nameRva -eq 0) { break }
            $start = & $rvaToOffset $nameRva
            $end = $start
            while ($end -lt $bytes.Length -and $bytes[$end] -ne 0) { $end++ }
            if ($end -eq $bytes.Length) { throw 'Unterminated import name' }
            $names += [Text.Encoding]::ASCII.GetString($bytes, $start, $end - $start)
        }
    }
    [pscustomobject]@{
        machine = ('0x{0:x}' -f [BitConverter]::ToUInt16($bytes, $pe + 4))
        imported_dlls = @($names | Sort-Object -Unique)
        delay_import_directory_present = ($delayRva -ne 0)
        scope = 'PE direct normal imports only; no static-library or runtime LoadLibrary provenance claim'
    }
}

Push-Location $root
try {
    $metadataText = & cargo metadata --offline --locked --format-version 1 --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'Cargo workspace metadata failed' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $tracked = @(& git ls-files)
    if ($LASTEXITCODE -ne 0) { throw 'git ls-files failed' }
    $sourcePaths = @($tracked | Where-Object { $_ -match '^(crates/[^/]+/src/|src/).*\.rs$' })
    $symbols = @()
    $sources = @()
    foreach ($path in $sourcePaths) {
        $lines = Get-Content -LiteralPath $path
        $sources += [pscustomobject]@{file=$path; sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower()}
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^\s*pub(?:\([^)]*\))?\s+(?:(?:async|const|unsafe)\s+)?(?:fn|struct|enum|trait|type|mod|use)\s+') {
                $symbols += [pscustomobject]@{file=$path;line=$i+1;declaration=$lines[$i].Trim()}
            }
        }
    }
    # Parse only the concrete Cargo.lock package fields needed for this inventory.
    # This is not a general TOML parser and not a feature/platform resolver.
    $lockText = Get-Content Cargo.lock -Raw
    $lockedPackages = @()
    foreach ($block in [regex]::Split($lockText, '(?m)^\[\[package\]\]\s*$') | Select-Object -Skip 1) {
        $name = [regex]::Match($block, '(?m)^name = "([^"]+)"').Groups[1].Value
        $version = [regex]::Match($block, '(?m)^version = "([^"]+)"').Groups[1].Value
        $source = [regex]::Match($block, '(?m)^source = "([^"]+)"').Groups[1].Value
        $checksum = [regex]::Match($block, '(?m)^checksum = "([^"]+)"').Groups[1].Value
        $dependencyBlock = [regex]::Match($block, '(?ms)^dependencies = \[(.*?)\]').Groups[1].Value
        $deps = @([regex]::Matches($dependencyBlock, '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
        $lockedPackages += [pscustomobject]@{name=$name;version=$version;source=$source;checksum=$checksum;dependencies=$deps}
    }
    $binaryPaths = @()
    foreach ($name in @('argorixc','argorix-vm','argorix-conformance','argorix-sign')) {
        foreach ($folder in @('target/debug','target/release','target/eval-tripwire/release')) {
            $path = "$folder/$name.exe"
            if (Test-Path -LiteralPath $path) {
                $binaryPaths += [pscustomobject]@{file=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower();bytes=(Get-Item -LiteralPath $path).Length;pe=(Read-PEImports (Join-Path $root $path));provenance='existing artifact; not rebuilt or asserted to match HEAD'}
            }
        }
    }
    $npm = Get-Content demo/argorix-chatbot-runtime/package-lock.json -Raw | ConvertFrom-Json -AsHashtable
    $nativeNpm = @($npm.packages.GetEnumerator() | Where-Object { $_.Key -match '^node_modules/(@next/swc|@img/|sharp$)' } | ForEach-Object {
        [pscustomobject]@{path=$_.Key;version=$_.Value.version;optional=$_.Value.optional;cpu=$_.Value.cpu;os=$_.Value.os;integrity=$_.Value.integrity}
    })
    $workflowFiles = @($tracked | Where-Object {$_ -match '^\.github/workflows/.*\.yml$'})
    $entryFiles = @($tracked | Where-Object {$_ -match '^(scripts/.*\.py|paper/(Makefile|scripts/.*\.(py|ps1))|evaluation/adversarial/(run\.py|harness/.*\.py)|tools/.*package\.json|demo/argorix-chatbot-runtime/(package(-lock)?\.json|lib/.*\.ts|app/api/.*\.ts))$'})
    $support = @($workflowFiles + $entryFiles + @('Cargo.toml','Cargo.lock'))
    $supportHashes = @($support | Sort-Object -Unique | ForEach-Object {[pscustomobject]@{file=$_;sha256=(Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLower()}})
    $surface = @()
    foreach ($pair in @(
        @('crates/argorix_parser/src/ast.rs','Program'),
        @('crates/argorix_parser/src/ast.rs','MessageFieldType'),
        @('crates/argorix_parser/src/ast.rs','HandlerInstruction'),
        @('crates/argorix_parser/src/lexer.rs','TokenKind'),
        @('crates/argorix_bytecode/src/bytecode.rs','Instruction')
    )) {
        $body = Get-Content -LiteralPath $pair[0] -Raw
        $block = [regex]::Match($body, ('(?ms)^pub (?:struct|enum) ' + $pair[1] + ' \{.*?^\}')).Value
        if (-not $block) { throw "Missing language surface $($pair[1])" }
        $members = @([regex]::Matches($block, '(?m)^    (?:pub )?([A-Za-z_][A-Za-z_0-9]*)') | ForEach-Object {$_.Groups[1].Value})
        $surface += [pscustomobject]@{file=$pair[0];symbol=$pair[1];members=$members}
    }
    [pscustomobject]@{
        schema_version=1
        commit=(& git rev-parse HEAD)
        branch=(& git branch --show-current)
        tracked_diff=(@(& git diff --name-only HEAD))
        toolchain=[pscustomobject]@{cargo=(& cargo --version);rustc=(& rustc --version);powershell=$PSVersionTable.PSVersion.ToString()}
        packages=@($metadata.packages | ForEach-Object {
            [pscustomobject]@{name=$_.name;version=$_.version;manifest=(Relative $_.manifest_path);features=$_.features;targets=@($_.targets | ForEach-Object {[pscustomobject]@{name=$_.name;kind=$_.kind;path=(Relative $_.src_path)}});dependencies=@($_.dependencies | ForEach-Object {[pscustomobject]@{name=$_.name;kind=$_.kind;requirement=$_.req;target=$_.target;optional=$_.optional;features=$_.features;default_features=$_.uses_default_features;workspace=($null -ne $_.path)}})}
        })
        source_files=$sources
        public_declaration_candidates=$symbols
        public_scan_limit='Lexical inventory, includes cfg-gated declarations and test modules within src; not rustdoc reachability or full signatures.'
        locked_packages=$lockedPackages
        lock_scope='All locked packages, including optional/target-specific dependencies; not exact selected link graph.'
        existing_binaries=$binaryPaths
        demo_native_lock_entries=$nativeNpm
        workflow_files=$workflowFiles
        support_files=$supportHashes
        language_surface=$surface
    } | ConvertTo-Json -Depth 15
} finally {
    Pop-Location
}
