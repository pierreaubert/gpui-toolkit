param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Archive
)

$ErrorActionPreference = "Stop"
$source = Join-Path $Root "src"
if ($Root -notmatch "^[A-Za-z]:\\gpui-toolkit-qa\\[a-z0-9-]+$") {
    throw "refusing a non-dedicated or non-unique QA root: $Root"
}

if (Test-Path -LiteralPath $source) {
    throw "refusing to replace existing QA source: $source"
}
New-Item -ItemType Directory -Force -Path $source | Out-Null
& tar.exe -xzf $Archive -C $source
if ($LASTEXITCODE -ne 0) {
    throw "tar.exe failed to extract the gpui-toolkit QA workspace"
}
foreach ($sibling in @("gpui-toolkit", "math-audio")) {
    if (-not (Test-Path -PathType Leaf (Join-Path $source "$sibling\Cargo.toml"))) {
        throw "missing pinned sibling after extraction: $sibling"
    }
}
