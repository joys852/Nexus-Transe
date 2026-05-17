# 为当前 PowerShell 会话启用 uv 国内镜像（临时）
# 用法: . .\scripts\use-china-mirror.ps1

$env:UV_INDEX_URL = "https://pypi.tuna.tsinghua.edu.cn/simple"
$env:UV_EXTRA_INDEX_URL = "https://mirrors.aliyun.com/pypi/simple/"
$env:UV_PYTHON_INSTALL_MIRROR = "https://npmmirror.com/mirrors/python-build-standalone/"

# 可选：永久写入用户环境变量（取消注释）
# [Environment]::SetEnvironmentVariable("UV_INDEX_URL", $env:UV_INDEX_URL, "User")

Write-Host "uv mirror enabled:" -ForegroundColor Green
Write-Host "  UV_INDEX_URL = $env:UV_INDEX_URL"
Write-Host "  UV_PYTHON_INSTALL_MIRROR = $env:UV_PYTHON_INSTALL_MIRROR"
Write-Host ""
Write-Host "Run from repo root:" -ForegroundColor Cyan
Write-Host "  cd E:\IDE"
Write-Host "  uv sync --directory packages/nexus-engine --extra dev"
