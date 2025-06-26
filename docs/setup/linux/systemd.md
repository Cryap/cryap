# systemd service

Copy this sample to `/etc/systemd/system/cryap.service`

```ini
[Unit]
Description=Cryap
After=network.target

Wants=postgresql.service
Wants=redis.service
After=postgresql.service
After=redis.service

[Service]
RestartSec=2s
Type=simple
User=cryap
Group=cryap
WorkingDirectory=/var/lib/cryap
ExecStart=/usr/bin/cryap
Restart=always

[Install]
WantedBy=multi-user.target
```
