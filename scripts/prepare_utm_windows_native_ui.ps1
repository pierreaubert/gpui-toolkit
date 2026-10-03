param(
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$UserName,
    [Parameter(Mandatory = $true)][string]$StatusPath,
    [Parameter(Mandatory = $true)][string]$RunId,
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
    if ($RunId -notmatch '^[a-z0-9-]+$') {
        throw "invalid QA run ID: $RunId"
    }
    if (-not (Test-InteractiveDesktop $UserName)) {
        Write-QaStatus "awaiting-login" "Log in to the $UserName Windows desktop in UTM."
        exit 20
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
    $rustc = Join-Path $env:CARGO_HOME "bin\rustc.exe"
    $rustup = Join-Path $env:CARGO_HOME "bin\rustup.exe"
    if (-not (Test-Path -PathType Leaf $rustc) -or -not (Test-Path -PathType Leaf $rustup)) {
        throw "Rust compiler or rustup is missing from $env:CARGO_HOME\bin"
    }
    $toolchains = & $rustup toolchain list
    if ($LASTEXITCODE -ne 0 -or -not ($toolchains -match '^1\.99\.0-aarch64-pc-windows-msvc')) {
        throw "Rust 1.99.0 Windows ARM64 toolchain is not installed"
    }
    $cargoVersion = & $cargo +1.99.0 --version
    if ($LASTEXITCODE -ne 0) { throw "cargo 1.99.0 failed" }
    $rustcVersion = & $rustc +1.99.0 --version
    if ($LASTEXITCODE -ne 0) { throw "rustc 1.99.0 failed" }
    $targets = & $rustup target list --installed --toolchain 1.99.0
    if ($LASTEXITCODE -ne 0 -or $targets -notcontains "aarch64-pc-windows-msvc") {
        throw "aarch64-pc-windows-msvc target is not installed"
    }
    if (-not (Get-Command git.exe -ErrorAction SilentlyContinue)) {
        throw "Git for Windows is unavailable"
    }
    if ($CheckOnly) {
        Write-QaStatus "desktop-ready" "Interactive desktop; $cargoVersion; $rustcVersion; aarch64-pc-windows-msvc installed."
        exit 0
    }

    @{
        pid = $PID
        run_id = $RunId
        repo_root = $RepoRoot
    } | ConvertTo-Json | Set-Content -Encoding UTF8 "$StatusPath.build-owner.json"
    Push-Location $RepoRoot
    try {
        & $cargo +1.99.0 check --locked -p gpui-toolkit-gpui-util --target aarch64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) {
            throw "Windows gpui-util check failed with exit code $LASTEXITCODE"
        }
        & $cargo +1.99.0 build --locked --target aarch64-pc-windows-msvc -p gpui-builder --features showcase --bin layout-showcase
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $runner = Join-Path $RepoRoot "scripts\run_windows_native_ui_smoke.ps1"
    $binary = Join-Path $RepoRoot "target\aarch64-pc-windows-msvc\debug\layout-showcase.exe"
    $artifactDirectory = Join-Path $profile "AppData\Local\Temp\gpui-toolkit-qa-$RunId"
    if (-not (Test-Path -PathType Container (Split-Path -Parent $artifactDirectory))) {
        throw "interactive user's temporary directory is unavailable"
    }
    $artifact = Join-Path $artifactDirectory "gpui-builder-smoke.json"
    $screenshot = Join-Path $artifactDirectory "gpui-builder.png"
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
    $taskName = "GpuiToolkitNativeUiSmoke-$RunId"
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    Write-QaStatus "scheduled" "Interactive native UI capture task started."
} catch {
    Write-QaStatus "error" ($_ | Out-String)
    throw
}
