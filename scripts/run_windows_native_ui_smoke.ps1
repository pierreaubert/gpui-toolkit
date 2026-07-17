param(
    [Parameter(Mandatory = $true)][string]$Binary,
    [Parameter(Mandatory = $true)][string]$Artifact,
    [Parameter(Mandatory = $true)][string]$Screenshot
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class GpuiNativeWindowCapture {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr window, out Rect rect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr window);
}
"@

$artifactDirectory = Split-Path -Parent $Artifact
$screenshotDirectory = Split-Path -Parent $Screenshot
New-Item -ItemType Directory -Force -Path $artifactDirectory, $screenshotDirectory | Out-Null
Remove-Item -Force -ErrorAction SilentlyContinue $Artifact, $Screenshot, "$Artifact.error.txt"

$env:GPUI_NATIVE_SMOKE_HOLD_MS = if ($env:GPUI_NATIVE_SMOKE_HOLD_MS) {
    $env:GPUI_NATIVE_SMOKE_HOLD_MS
} else {
    "8000"
}

$process = $null
try {
    $process = Start-Process -FilePath $Binary -ArgumentList @(
        "--smoke-test",
        "--smoke-artifact",
        $Artifact
    ) -PassThru

    $windowHandle = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        Start-Sleep -Milliseconds 100
        $process.Refresh()
        if ($process.HasExited) {
            throw "layout showcase exited before its native window was visible"
        }
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) {
            $windowHandle = $process.MainWindowHandle
            break
        }
    }
    if ($windowHandle -eq [IntPtr]::Zero) {
        throw "Layout Builder Showcase window was not discoverable"
    }

    [void][GpuiNativeWindowCapture]::SetForegroundWindow($windowHandle)
    Start-Sleep -Seconds 1
    $rect = [GpuiNativeWindowCapture+Rect]::new()
    if (-not [GpuiNativeWindowCapture]::GetWindowRect($windowHandle, [ref]$rect)) {
        throw "GetWindowRect failed for the GPUI showcase window"
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -lt 100 -or $height -lt 100) {
        throw "GPUI showcase window has invalid capture bounds: ${width}x${height}"
    }

    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen(
                $rect.Left,
                $rect.Top,
                0,
                0,
                ([System.Drawing.Size]::new($width, $height))
            )
        } finally {
            $graphics.Dispose()
        }
        $bitmap.Save($Screenshot, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }

    if (-not $process.WaitForExit(30000)) {
        throw "layout showcase did not finish its smoke run within 30 seconds"
    }
    if ($process.ExitCode -ne 0) {
        throw "layout showcase exited with code $($process.ExitCode)"
    }
    if (-not (Test-Path -PathType Leaf $Artifact)) {
        throw "layout showcase did not write its smoke artifact"
    }
} catch {
    $_ | Out-String | Set-Content -Encoding UTF8 "$Artifact.error.txt"
    throw
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
