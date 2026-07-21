# systemd units

The units running on prod. Paths assume the deploy layout `deploy-prod.sh` expects:
repo at `/home/ubuntu/work/OnlineRPG`, running as `ubuntu`.

## Install

```bash
sudo cp tools/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now openmmo-server openmmo-agent-client
```

## Secrets

Neither unit carries credentials. Both read an optional `EnvironmentFile` that is
absent from this repo by design:

- `/etc/openmmo/server.env`
- `/etc/openmmo/agent-client.env`

The agent client additionally needs a logged-in codex CLI (`codex login`, writes
`~/.codex/auth.json`) and `agent-client/data/config.toml`, which is gitignored
because it holds deployment-only values. Copy `data/config.toml.example` and fill
it in.
