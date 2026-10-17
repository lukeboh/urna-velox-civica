# Urna Velox — atalho para Windows: roda bench/benchmark.sh dentro do WSL2.
#
#   .\bench\benchmark.ps1                 execução completa
#   .\bench\benchmark.ps1 -Rapido         execução curta (~1–2 min)
#   .\bench\benchmark.ps1 -Distro Ubuntu-24.04
#
# Requer o WSL2 com uma distribuição Linux que tenha Java 21+ e Node 20+ (o Rust é instalado
# automaticamente, só para o seu usuário do Linux, se faltar).
param([switch]$Rapido, [string]$Distro = "")
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path -replace '\\', '/'
$d = @(); if ($Distro) { $d = @("-d", $Distro) }
$linuxRepo = (& wsl @d wslpath -a $repo).Trim()
$args2 = @(); if ($Rapido) { $args2 = @("--rapido") }
& wsl @d --cd "$linuxRepo" -- bash ./bench/benchmark.sh @args2
exit $LASTEXITCODE
