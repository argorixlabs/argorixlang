param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$report = Get-Content (Join-Path $PSScriptRoot 'dependencies.json') -Raw | ConvertFrom-Json
$fresh = (& (Join-Path $PSScriptRoot 'collect-inventory.ps1') -Repository $root) | ConvertFrom-Json

function Assert-FileHashes($Expected, $Actual) {
    $expectedNames = @($Expected | ForEach-Object {$_.file} | Sort-Object)
    $actualNames = @($Actual | ForEach-Object {$_.file} | Sort-Object)
    if (Compare-Object $expectedNames $actualNames) { throw 'File coverage differs; review inventory' }
    foreach ($item in $Expected) {
        $other = @($Actual | Where-Object {$_.file -eq $item.file})
        if ($other.Count -ne 1 -or $other[0].sha256 -ne $item.sha256) { throw "HASH mismatch: $($item.file)" }
    }
}

if ($fresh.commit -ne $report.commit) { throw 'Commit changed; review/refresh snapshot before claiming it is current' }
$names = @($fresh.packages | ForEach-Object {$_.name} | Sort-Object)
$recordedNames = @($report.components | ForEach-Object {$_.name} | Sort-Object)
if (Compare-Object $names $recordedNames) { throw 'Workspace coverage differs' }
foreach ($component in $report.components) {
    if (-not $component.migration_target -or -not $component.principal_api -or -not $component.formats) { throw "Missing migration contract: $($component.name)" }
    foreach ($target in $component.targets) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $target.path))) { throw "Target not found: $($target.path)" }
    }
    foreach ($field in @('version','features','targets','dependencies')) {
        $current = $fresh.packages | Where-Object {$_.name -eq $component.name}
        if (($component.$field | ConvertTo-Json -Depth 12 -Compress) -ne ($current.$field | ConvertTo-Json -Depth 12 -Compress)) { throw "Metadata changed: $($component.name)/$field" }
    }
}
Assert-FileHashes $report.scan.source_files $fresh.source_files
Assert-FileHashes $report.scan.support_files $fresh.support_files
Assert-FileHashes $report.scan.existing_binaries $fresh.existing_binaries
foreach ($field in @('language_surface','locked_packages','public_declaration_candidates')) {
    if (($report.scan.$field | ConvertTo-Json -Depth 12 -Compress) -ne ($fresh.$field | ConvertTo-Json -Depth 12 -Compress)) { throw "Snapshot changed: $field" }
}
foreach ($symbol in $report.scan.public_declaration_candidates) {
    $line = (Get-Content -LiteralPath (Join-Path $root $symbol.file))[$symbol.line - 1].Trim()
    if ($line -ne $symbol.declaration) { throw "Symbol evidence changed: $($symbol.file):$($symbol.line)" }
}
foreach ($package in $fresh.locked_packages) {
    foreach ($dependency in $package.dependencies) {
        $parts = $dependency.Split(' ')
        $matches = @($fresh.locked_packages | Where-Object {$_.name -eq $parts[0]})
        if ($parts.Count -gt 1) { $matches = @($matches | Where-Object {$_.version -eq $parts[1]}) }
        if ($matches.Count -ne 1) { throw "Unresolved lock edge: $($package.name) -> $dependency" }
    }
}
$external = @($fresh.packages.dependencies | Where-Object {-not $_.workspace} | ForEach-Object {$_.name} | Sort-Object -Unique)
$classified = @($report.external_direct_dependencies | ForEach-Object {$_.name} | Sort-Object -Unique)
if (Compare-Object $external $classified) { throw 'An external dependency lacks classification' }
foreach ($path in $report.execution_paths) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $path.evidence))) { throw "Path evidence missing: $($path.id)" }
}
$negativeControl = 'not requested'
if ($SelfTest) {
    $copy = ($fresh.source_files | ConvertTo-Json -Depth 4) | ConvertFrom-Json
    $copy[0].sha256 = '0000000000000000000000000000000000000000000000000000000000000000'
    $rejected = $false
    try { Assert-FileHashes $report.scan.source_files $copy } catch {
        if ($_.Exception.Message -notlike 'HASH mismatch:*') { throw }
        $rejected = $true
    }
    if (-not $rejected) { throw 'Negative control failed: corrupted hash was accepted' }
    $negativeControl = 'PASS: corrupted in-memory hash rejected; no files changed'
}
[pscustomobject]@{
    status='PASS'
    task='ESP-001'
    commit=$fresh.commit
    packages=$fresh.packages.Count
    unique_binaries=@($fresh.packages.targets | Where-Object {$_.kind -contains 'bin'}).Count
    source_files=$fresh.source_files.Count
    public_declaration_candidates=$fresh.public_declaration_candidates.Count
    locked_packages=$fresh.locked_packages.Count
    external_direct_dependencies=$external.Count
    existing_binary_artifacts=$fresh.existing_binaries.Count
    source_and_support_hashes='MATCH'
    lock_edges='RESOLVED_WITHIN_LOCKFILE_NOT_FEATURE_RESOLVED'
    negative_control=$negativeControl
    scope='Inventory validation only; no build, conformance campaign or security certification'
} | ConvertTo-Json -Depth 4
