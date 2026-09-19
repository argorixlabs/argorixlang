param([switch]$SelfTest)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$contractPath = Join-Path $PSScriptRoot 'requirements.json'
$contract = Get-Content -LiteralPath $contractPath -Raw | ConvertFrom-Json
$registry = Get-Content -LiteralPath (Join-Path $root 'tasks/madurez/BACKLOG.json') -Raw | ConvertFrom-Json
$espText = Get-Content -LiteralPath (Join-Path $root 'PLAN_ESPADA_INDEPENDIENTE.md') -Raw
$taskIds = @($registry.tasks | ForEach-Object { $_.id })
$taskIds += @([regex]::Matches($espText, '(?m)^\| (ESP-\d{3}) \| [^\r\n]*\| (?:PENDIENTE|HECHA) \|$') | ForEach-Object { $_.Groups[1].Value })

function Assert-Contract($c) {
    if ($c.schema_version -ne 1 -or $c.release_levels.Count -ne 3) { throw 'Schema or release levels invalid' }
    if (@($c.requirements | Select-Object -ExpandProperty id -Unique).Count -ne $c.requirements.Count) { throw 'Duplicate requirement ID' }
    if (@($c.test_cases | Select-Object -ExpandProperty id -Unique).Count -ne $c.test_cases.Count) { throw 'Duplicate test ID' }
    if (@($c.applications | Select-Object -ExpandProperty id -Unique).Count -ne $c.applications.Count) { throw 'Duplicate application ID' }
    if ($c.platform_profiles.Count -ne 2 -or @($c.platform_profiles.id | Sort-Object -Unique).Count -ne 2) { throw 'Platform profiles incomplete' }
    foreach ($platform in $c.platform_profiles) {
        if (-not $platform.reference -or -not $platform.abi -or -not $platform.min_test_envelope -or $platform.release_levels.Count -ne 3) { throw "Incomplete platform: $($platform.id)" }
        foreach ($level in @('R1','R2','R3')) { if ($platform.release_levels -notcontains $level) { throw "Platform missing level $level" } }
    }
    foreach ($requirement in $c.requirements) {
        if (-not $requirement.mandatory -or $c.release_levels -notcontains $requirement.required_level) { throw "Unscoped requirement: $($requirement.id)" }
        if (-not $requirement.owner_role -or $c.roles -notcontains $requirement.owner_role) { throw "Owner missing: $($requirement.id)" }
        if (-not $requirement.acceptance -or $requirement.test_ids.Count -lt 1 -or $requirement.evidence_expected.Count -lt 1) { throw "No acceptance trace: $($requirement.id)" }
        foreach ($task in $requirement.implementation_tasks) { if ($taskIds -notcontains $task) { throw "Unknown task $task for $($requirement.id)" } }
        foreach ($testId in $requirement.test_ids) {
            $matching = @($c.test_cases | Where-Object { $_.id -eq $testId })
            if ($matching.Count -ne 1 -or $matching[0].requirements -notcontains $requirement.id) { throw "TEST TRACE missing: $($requirement.id) -> $testId" }
        }
    }
    foreach ($test in $c.test_cases) {
        if (-not $test.given -or -not $test.when -or -not $test.then -or -not $test.oracle -or -not $test.evidence_expected) { throw "Incomplete test: $($test.id)" }
        if ($test.status -eq 'PLANNED' -and $test.evidence_state -ne 'NOT_PRODUCED') { throw "Planned test falsely claims evidence: $($test.id)" }
        if ($test.requirements.Count -lt 1) { throw "Orphan test: $($test.id)" }
        foreach ($reqId in $test.requirements) {
            $matching = @($c.requirements | Where-Object { $_.id -eq $reqId })
            if ($matching.Count -ne 1 -or $matching[0].test_ids -notcontains $test.id) { throw "REQUIREMENT TRACE missing: $($test.id) -> $reqId" }
        }
    }
    if ($c.applications.Count -ne 3) { throw 'Three reference applications required' }
    foreach ($app in $c.applications) {
        foreach ($mode in @('success','denied','fault')) {
            $matching = @($c.test_cases | Where-Object { $_.application -eq $app.id -and $_.kind -eq $mode })
            if ($matching.Count -ne 1) { throw "Application scenario missing/duplicated: $($app.id)/$mode" }
        }
    }
    foreach ($floor in @('soak_hours_per_platform','external_beta_developers','external_beta_projects','remote_agent_processes','reference_message_count','committed_state_rpo','recovery_rto','prohibited_effects')) {
        if (-not $c.acceptance_floors.$floor) { throw "Acceptance floor missing: $floor" }
    }
}

Assert-Contract $contract
$negative = 'not requested'
if ($SelfTest) {
    $copy = ($contract | ConvertTo-Json -Depth 20) | ConvertFrom-Json
    $copy.requirements[0].test_ids = @('TC-NONEXISTENT')
    $rejected = $false
    try { Assert-Contract $copy } catch {
        if ($_.Exception.Message -notlike 'TEST TRACE missing:*') { throw }
        $rejected = $true
    }
    if (-not $rejected) { throw 'Negative control failed: orphan requirement test accepted' }
    $negative = 'PASS: broken requirement/test link rejected in memory'
}
[pscustomobject]@{
    status='PASS'
    task='MAT-001'
    implementation_baseline_commit=$contract.implementation_baseline_commit
    requirements=$contract.requirements.Count
    tests=$contract.test_cases.Count
    applications=$contract.applications.Count
    platform_profiles=$contract.platform_profiles.Count
    release_levels=$contract.release_levels.Count
    evidence_produced=0
    negative_control=$negative
    scope='Contract consistency only; product acceptance tests have not run'
} | ConvertTo-Json -Depth 4
