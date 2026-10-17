# Urna Velox — atalho para Windows: roda aquisicao/teste-real.sh dentro do WSL2.
#
#   .\aquisicao\teste-real.ps1                  completo
#   .\aquisicao\teste-real.ps1 -Rapido          curto (~3 min)
#   .\aquisicao\teste-real.ps1 -Nacional        coleta todas as seções (~517 mil BUs, ~9 GB, horas)
#   .\aquisicao\teste-real.ps1 -Distro Ubuntu-24.04
param([switch]$Rapido, [switch]$Nacional, [string]$Distro = "")
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path -replace '\\', '/'
$d = @(); if ($Distro) { $d = @("-d", $Distro) }
$linuxRepo = (& wsl @d wslpath -a $repo).Trim()
$args2 = @(); if ($Rapido) { $args2 = @("--rapido") }
$pre = ""; if ($Nacional) { $pre = "UV_NACIONAL=1 " }
& wsl @d --cd "$linuxRepo" -- bash -c "$pre./aquisicao/teste-real.sh $($args2 -join ' ')"
exit $LASTEXITCODE
