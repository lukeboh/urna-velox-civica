#!/usr/bin/env bash
# Roda NO SERVIDOR (como root, via sudo): compila o Urna Velox a partir de /tmp/uv-src.tar.gz (enviado pelo
# 10_implantar.sh com `git archive`) e instala os binários em /opt/urna-velox/bin. Uso: sudo bash compilar.sh
set -euo pipefail
[ -f /tmp/uv-src.tar.gz ] || { echo "falta /tmp/uv-src.tar.gz"; exit 1; }
rm -rf /srv/urna-velox/src && install -d -o urnavelox -g urnavelox /srv/urna-velox/src
tar -xzf /tmp/uv-src.tar.gz -C /srv/urna-velox/src && chown -R urnavelox:urnavelox /srv/urna-velox/src
echo "== compilando ($(nproc) CPUs; com LTO leva ~15-25 min em 2 OCPUs A1)"
sudo -u urnavelox -H bash -c 'cd /srv/urna-velox/src && /srv/urna-velox/.cargo/bin/cargo build --release -p uv-aquisicao -p urna-velox 2>&1 | tail -3'
for b in uv-servico uv-coletor uv-simulador uv-carga uv-motor; do
  install -m 0755 "/srv/urna-velox/src/target/release/$b" /opt/urna-velox/bin/
done
/opt/urna-velox/bin/uv-servico --version
cp /srv/urna-velox/src/VERSION /opt/urna-velox/VERSION
