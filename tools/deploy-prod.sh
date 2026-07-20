#!/usr/bin/env bash
# Build the current master on this host and put it live.
#
# Run it on the deploy host itself (prod). Overridable: REPO, WEBROOT, SERVICE.
set -euo pipefail

# git pull below rewrites this file while bash is still reading it, so re-exec
# from a snapshot first.
if [[ ${DEPLOY_FROM_SNAPSHOT:-} != 1 ]]; then
    snap=$(mktemp)
    cat "$0" > "$snap"
    DEPLOY_FROM_SNAPSHOT=1 exec bash "$snap" "$@"
fi
trap 'rm -f "$0"' EXIT

REPO=${REPO:-$HOME/work/OnlineRPG}
WEBROOT=${WEBROOT:-/var/www/openmmo}
SERVICE=${SERVICE:-openmmo-server}

cd "$REPO"

echo "==> git pull"
git pull --ff-only

echo "==> server (release)"
cargo build --release -p onlinerpg-server

echo "==> client deps"
(cd client && npm ci)

# `npm run build` runs build:wasm first, so the bundle always embeds fresh wasm.
echo "==> client bundle"
(cd client && npm run build)

# Both artifacts exist now. Swap the static files and restart together, so the
# wasm the browser downloads and the running server never disagree on the
# wire protocol.
echo "==> publish to $WEBROOT"
sudo rsync -a --delete client/dist/ "$WEBROOT/"
sudo chown -R www-data:www-data "$WEBROOT"

echo "==> restart $SERVICE"
sudo systemctl restart "$SERVICE"
sleep 3
sudo systemctl is-active "$SERVICE"

echo "==> deployed $(git log --oneline -1)"
