param([ValidateSet('dev','build','check','contracts')][string]$Action = 'dev')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/build-lock.ps1"
$buildLock = Enter-PinmeterBuildLock $projectRoot
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
function Run-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed: $LASTEXITCODE" }
}
Push-Location "$projectRoot/src/backend"
try {
    switch ($Action) {
        'contracts' {
            Run-Checked node @("$projectRoot/tools/desktop.mjs",'prepare')
            Run-Checked cargo @('run','-p','pinmeter-host','--features','dev-tools','--bin','export-contracts')
        }
        'check' {
            Run-Checked node @("$projectRoot/tools/desktop.mjs",'prepare')
            Run-Checked node @("$projectRoot/tools/check-project.mjs")
            Run-Checked node @("$projectRoot/tools/sync-taskbar-tokens.mjs",'--check')
            Run-Checked node @("$projectRoot/tools/desktop.mjs",'check-tools')
            Run-Checked cargo @('fmt','--all','--','--check')
            Run-Checked cargo @('test','--workspace','--all-features','--locked')
            Run-Checked cargo @('clippy','--workspace','--all-features','--all-targets','--locked','--','-D','warnings')
            Run-Checked cargo @('run','-p','pinmeter-host','--features','dev-tools','--bin','export-contracts','--','--check')
            Push-Location "$projectRoot/src/frontend"
            try { Run-Checked npm.cmd @('run','build'); Run-Checked npm.cmd @('test'); Run-Checked npm.cmd @('run','format:check') } finally { Pop-Location }
        }
        default { Run-Checked node @("$projectRoot/tools/desktop.mjs", $Action) }
    }
} finally { Pop-Location; Exit-PinmeterBuildLock $buildLock }
