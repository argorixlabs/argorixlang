param([switch]$SelfTest)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$catalog = Get-Content (Join-Path $PSScriptRoot 'claims.json') -Raw | ConvertFrom-Json
$requirements = Get-Content (Join-Path $PSScriptRoot 'requirements.json') -Raw | ConvertFrom-Json
$backlog = Get-Content (Join-Path $root 'tasks/madurez/BACKLOG.json') -Raw | ConvertFrom-Json
$trustText = Get-Content (Join-Path $PSScriptRoot 'trust-boundaries.md') -Raw
$reportText = Get-Content (Join-Path $root 'ArgorixLang-threat-model.md') -Raw
$espText = Get-Content (Join-Path $root 'PLAN_ESPADA_INDEPENDIENTE.md') -Raw
$validRequirements = @($requirements.requirements | ForEach-Object { $_.id })
$validTasks = @($backlog.tasks | ForEach-Object { $_.id })
$errors = [System.Collections.Generic.List[string]]::new()

if ($catalog.schema_version -ne 1) { $errors.Add('schema_version') }
if ($catalog.primary_deployment -ne 'local_cli_runtime') { $errors.Add('primary_deployment') }
if ($catalog.claims.Count -ne 11) { $errors.Add('claim_count') }
$ids = @($catalog.claims | ForEach-Object { $_.id })
$boundaries = @($catalog.claims | ForEach-Object { $_.boundary })
if (@($ids | Sort-Object -Unique).Count -ne $ids.Count) { $errors.Add('duplicate_claim') }
if (@($boundaries | Sort-Object -Unique).Count -ne $boundaries.Count) { $errors.Add('duplicate_boundary') }

foreach ($claim in $catalog.claims) {
    if ($claim.id -notmatch '^CL-[0-9]{2}$' -or $claim.boundary -notmatch '^TB-[0-9]{2}$') { $errors.Add("id:$($claim.id)") }
    if ($claim.guarantee.Length -lt 15 -or $claim.residual_risk.Length -lt 15 -or $claim.test -notmatch '^T-[0-9]{2}:') { $errors.Add("fields:$($claim.id)") }
    if ($trustText -notmatch [regex]::Escape($claim.boundary)) { $errors.Add("boundary_document:$($claim.boundary)") }
    if ($reportText -notmatch [regex]::Escape($claim.boundary)) { $errors.Add("report_boundary:$($claim.boundary)") }
    foreach ($file in $claim.evidence) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $file))) { $errors.Add("evidence:$file") }
    }
    foreach ($id in $claim.requirements) {
        if ($id -notin $validRequirements) { $errors.Add("requirement:$id") }
    }
    foreach ($id in $claim.tasks) {
        if ($id -in $validTasks) { continue }
        if ($id -match '^ESP-[0-9]{3}$' -and $espText -match [regex]::Escape("### $id")) { continue }
        $errors.Add("task:$id")
    }
}

if ($SelfTest) {
    $injected = @($catalog.claims[0].requirements) + 'SEC-999'
    $rejected = @($injected | Where-Object { $_ -notin $validRequirements })
    if ($rejected.Count -ne 1 -or $rejected[0] -ne 'SEC-999') { $errors.Add('negative_control') }
}

if ($errors.Count -gt 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Output "PASS: $($catalog.claims.Count) claims, $($boundaries.Count) boundaries, requirement/task/file links; negative_control=$SelfTest"
