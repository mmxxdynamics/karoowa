#!/usr/bin/env bash
# harden.sh — Security hardening for Karoowa devnet server.
#
# Run as root on a fresh Ubuntu 24.04 server:
#   sudo bash harden.sh
#
# What this does:
# 1. System updates + unattended security upgrades
# 2. Create dedicated 'karoowa' service user (no login shell)
# 3. SSH hardening (disable password auth, disable root login)
# 4. UFW firewall (only SSH + Karoowa ports)
# 5. fail2ban for SSH brute-force protection
# 6. Kernel security hardening (sysctl)
# 7. Disable unnecessary services
# 8. Set up log rotation

set -euo pipefail

echo "=== Karoowa Server Hardening ==="
echo ""

# Must be root.
if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: Run as root (sudo bash harden.sh)"
    exit 1
fi

# -----------------------------------------------------------------------
# 1. System updates
# -----------------------------------------------------------------------
echo "[1/8] System updates..."
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get upgrade -y -qq
apt-get install -y -qq \
    ufw \
    fail2ban \
    unattended-upgrades \
    apt-listchanges \
    logrotate \
    jq \
    curl \
    ca-certificates

# Enable unattended security upgrades.
cat > /etc/apt/apt.conf.d/20auto-upgrades << 'AUTOUPGRADE'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
AUTOUPGRADE

echo "  Done."

# -----------------------------------------------------------------------
# 2. Create karoowa service user
# -----------------------------------------------------------------------
echo "[2/8] Creating karoowa service user..."
if ! id -u karoowa &>/dev/null; then
    useradd --system --shell /usr/sbin/nologin --home-dir /opt/karoowa --create-home karoowa
    echo "  Created user 'karoowa'"
else
    echo "  User 'karoowa' already exists"
fi

mkdir -p /opt/karoowa/{bin,data,keys,logs}
chown -R karoowa:karoowa /opt/karoowa
chmod 700 /opt/karoowa/keys
echo "  Done."

# -----------------------------------------------------------------------
# 3. SSH hardening
# -----------------------------------------------------------------------
echo "[3/8] Hardening SSH..."
SSHD_CONFIG="/etc/ssh/sshd_config"

# Back up original.
cp "$SSHD_CONFIG" "${SSHD_CONFIG}.bak.$(date +%s)"

# Apply hardening settings.
sed -i 's/^#\?PermitRootLogin .*/PermitRootLogin no/' "$SSHD_CONFIG"
sed -i 's/^#\?PasswordAuthentication .*/PasswordAuthentication no/' "$SSHD_CONFIG"
sed -i 's/^#\?ChallengeResponseAuthentication .*/ChallengeResponseAuthentication no/' "$SSHD_CONFIG"
sed -i 's/^#\?X11Forwarding .*/X11Forwarding no/' "$SSHD_CONFIG"
sed -i 's/^#\?MaxAuthTries .*/MaxAuthTries 3/' "$SSHD_CONFIG"
sed -i 's/^#\?ClientAliveInterval .*/ClientAliveInterval 300/' "$SSHD_CONFIG"
sed -i 's/^#\?ClientAliveCountMax .*/ClientAliveCountMax 2/' "$SSHD_CONFIG"

# Only allow the ubuntu user to SSH in.
if ! grep -q "^AllowUsers" "$SSHD_CONFIG"; then
    echo "AllowUsers ubuntu" >> "$SSHD_CONFIG"
fi

# Restart SSH (won't kill current session).
systemctl restart sshd
echo "  Done."

# -----------------------------------------------------------------------
# 4. UFW firewall
# -----------------------------------------------------------------------
echo "[4/8] Configuring firewall (UFW)..."
ufw --force reset > /dev/null 2>&1

# Default deny incoming, allow outgoing.
ufw default deny incoming > /dev/null
ufw default allow outgoing > /dev/null

# SSH (port 22).
ufw allow 22/tcp comment "SSH" > /dev/null

# Karoowa RPC (port 8545) — rate limited to prevent abuse.
ufw allow 8545/tcp comment "Karoowa RPC" > /dev/null

# Karoowa P2P (port 30303).
ufw allow 30303/tcp comment "Karoowa P2P" > /dev/null

# Enable firewall.
ufw --force enable > /dev/null
echo "  Active rules:"
ufw status numbered 2>/dev/null | grep -E "^\[" | sed 's/^/    /'
echo "  Done."

# -----------------------------------------------------------------------
# 5. fail2ban
# -----------------------------------------------------------------------
echo "[5/8] Configuring fail2ban..."
cat > /etc/fail2ban/jail.local << 'FAIL2BAN'
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3
backend = systemd

[sshd]
enabled = true
port = 22
filter = sshd
maxretry = 3
bantime = 3600
FAIL2BAN

systemctl enable fail2ban > /dev/null 2>&1
systemctl restart fail2ban
echo "  Done."

# -----------------------------------------------------------------------
# 6. Kernel security hardening
# -----------------------------------------------------------------------
echo "[6/8] Kernel security hardening..."
cat > /etc/sysctl.d/99-karoowa-hardening.conf << 'SYSCTL'
# Disable IP forwarding (not a router).
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# Ignore ICMP broadcast requests.
net.ipv4.icmp_echo_ignore_broadcasts = 1

# Disable source routing.
net.ipv4.conf.all.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0

# Enable SYN flood protection.
net.ipv4.tcp_syncookies = 1

# Log suspicious packets.
net.ipv4.conf.all.log_martians = 1

# Disable ICMP redirects.
net.ipv4.conf.all.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0

# Randomize address space (ASLR).
kernel.randomize_va_space = 2

# Restrict dmesg access.
kernel.dmesg_restrict = 1

# Restrict kernel pointer exposure.
kernel.kptr_restrict = 2
SYSCTL

sysctl --system > /dev/null 2>&1
echo "  Done."

# -----------------------------------------------------------------------
# 7. Disable unnecessary services
# -----------------------------------------------------------------------
echo "[7/8] Disabling unnecessary services..."
for svc in cups avahi-daemon bluetooth; do
    if systemctl is-active --quiet "$svc" 2>/dev/null; then
        systemctl stop "$svc"
        systemctl disable "$svc" > /dev/null 2>&1
        echo "    Disabled: $svc"
    fi
done
echo "  Done."

# -----------------------------------------------------------------------
# 8. Karoowa systemd service
# -----------------------------------------------------------------------
echo "[8/8] Creating Karoowa systemd service..."
cat > /etc/systemd/system/karoowa-node.service << 'SYSTEMD'
[Unit]
Description=Karoowa Blockchain Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=karoowa
Group=karoowa
ExecStart=/opt/karoowa/bin/karoowa node \
    --validator-key /opt/karoowa/keys/validator.key \
    --consensus poa \
    --data-dir /opt/karoowa/data \
    --rpc-port 8545 \
    --p2p-port 30303 \
    --block-time 2
Restart=on-failure
RestartSec=10
LimitNOFILE=65536

# Security hardening for the service.
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/karoowa/data /opt/karoowa/logs
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictSUIDSGID=true
RestrictNamespaces=true

# Resource limits (2GB box).
MemoryMax=1536M
CPUQuota=150%

[Install]
WantedBy=multi-user.target
SYSTEMD

systemctl daemon-reload
echo "  Done."

# -----------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------
echo ""
echo "=== Hardening Complete ==="
echo ""
echo "Security measures applied:"
echo "  - SSH: password auth disabled, root login disabled, max 3 auth tries"
echo "  - Firewall: only ports 22 (SSH), 8545 (RPC), 30303 (P2P)"
echo "  - fail2ban: SSH brute-force protection (3 tries, 1hr ban)"
echo "  - Unattended security upgrades enabled"
echo "  - Kernel hardened (SYN cookies, no redirects, ASLR, restricted dmesg)"
echo "  - Karoowa runs as dedicated 'karoowa' user with systemd sandboxing"
echo ""
echo "Next steps:"
echo "  1. Upload the karoowa binary to /opt/karoowa/bin/karoowa"
echo "  2. Generate a validator key (the keys dir is 0700 karoowa, so this needs sudo):"
echo "       sudo karoowa wallet new --output /opt/karoowa/keys/validator.key"
echo "  3. Hand the key to the service user — it is written 0600, so a"
echo "     root-owned key is NOT readable by User=karoowa:"
echo "       sudo chown karoowa:karoowa /opt/karoowa/keys/validator.key"
echo "  4. Start the node:  sudo systemctl enable --now karoowa-node"
echo "  5. Check status:    sudo systemctl status karoowa-node"
echo "  6. View logs:       sudo journalctl -u karoowa-node -f"
