param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$UserName,
    [Parameter(Mandatory = $true)][string]$StatusPath,
    [switch]$CheckOnly
)

$ErrorActionPreference = "Stop"

function Write-QaStatus([string]$State, [string]$Message) {
    $directory = Split-Path -Parent $StatusPath
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    @{
        schema_version = 1
        state = $State
        message = $Message
    } | ConvertTo-Json | Set-Content -Encoding UTF8 $StatusPath
}

function Test-InteractiveDesktop([string]$ExpectedUser) {
    if (Get-Process -Name LogonUI -ErrorAction SilentlyContinue) {
        return $false
    }
    foreach ($process in Get-CimInstance Win32_Process -Filter "Name = 'explorer.exe'") {
        $owner = Invoke-CimMethod -InputObject $process -MethodName GetOwner
        if ($owner.User -eq $ExpectedUser) {
            return $true
        }
    }
    return $false
}

try {
    if (-not (Test-InteractiveDesktop $UserName)) {
        Write-QaStatus "awaiting-login" "Log in to the $UserName Windows desktop in UTM."
        exit 20
    }
    if ($CheckOnly) {
        Write-QaStatus "desktop-ready" "Interactive Windows desktop is available."
        exit 0
    }

    $cargo = "C:\Users\$UserName\.cargo\bin\cargo.exe"
    if (-not (Test-Path -PathType Leaf $cargo)) {
        throw "Rust toolchain is missing: $cargo"
    }
    $profile = "C:\Users\$UserName"
    $env:CARGO_HOME = Join-Path $profile ".cargo"
    $env:RUSTUP_HOME = Join-Path $profile ".rustup"
    $env:USERPROFILE = $profile
    $env:HOME = $profile
    $env:Path = "$env:CARGO_HOME\bin;C:\Program Files\Git\cmd;$env:Path"
    Push-Location $RepoRoot
    try {
        & $cargo build -p gpui-builder --features showcase --bin layout-showcase
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $runner = Join-Path $RepoRoot "scripts\run_windows_native_ui_smoke.ps1"
    $binary = Join-Path $RepoRoot "target\debug\layout-showcase.exe"
    $artifact = Join-Path $RepoRoot "target\qa\native-ui\windows\gpui-builder-smoke.json"
    $screenshot = Join-Path $RepoRoot "target\qa\native-ui\windows\gpui-builder.png"
    $artifactDirectory = Split-Path -Parent $artifact
    New-Item -ItemType Directory -Force -Path $artifactDirectory | Out-Null
    & icacls.exe $artifactDirectory /grant "${UserName}:(OI)(CI)M" /T /C | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to grant the interactive QA user access to $artifactDirectory"
    }
    $powershell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy Bypass",
        "-File `"$runner`"",
        "-Binary `"$binary`"",
        "-Artifact `"$artifact`"",
        "-Screenshot `"$screenshot`""
    ) -join " "

    $action = New-ScheduledTaskAction -Execute $powershell -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId $UserName -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    $taskName = "GpuiToolkitNativeUiSmoke"
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    Write-QaStatus "scheduled" "Interactive native UI capture task started."
} catch {
    Write-QaStatus "error" ($_ | Out-String)
    throw
}
