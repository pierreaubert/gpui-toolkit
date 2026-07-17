param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Archive
)

$ErrorActionPreference = "Stop"
$source = Join-Path $Root "src"
if ($Root -notmatch "^[A-Za-z]:\\gpui-toolkit-qa$") {
    throw "refusing to replace non-dedicated QA root: $Root"
}

Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $source
New-Item -ItemType Directory -Force -Path $source | Out-Null
& tar.exe -xzf $Archive -C $source
if ($LASTEXITCODE -ne 0) {
    throw "tar.exe failed to extract the gpui-toolkit QA workspace"
}
