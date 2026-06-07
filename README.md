# Trapline — The Bug Bounty Recon Deck

> 215 battle-tested commands. Real-time flag detection. One-click HackerOne reports.

**[Download Free (Windows)](https://github.com/foodstampplug/trapline/releases/download/v1.0.0/Trapline_1.0.0_x64-setup.exe)** · **[Get Pro — $9](https://foodstampplug.gumroad.com/l/trapline-pro)** · **[trapline.xyz](https://trapline.xyz)**

---

## What it does

Trapline is a Windows desktop app that fixes three things that cost bug bounty hunters time:

1. **Finding the right command fast** — 215 commands across 30 categories with live search. No more digging through Notion docs or bash aliases.
2. **Missing findings in streamed output** — 94 detection rules scan every output line in real time. ATO tokens, secrets, CORS misconfigs, private IPs — flagged in red before you finish reading.
3. **The report template grind** — One click generates a complete HackerOne/Bugcrowd submission: CVSS:3.1 vector auto-calculated, OWASP reference matched from your title, impact statement pre-filled.

---

## Quickfire — the 11 highest-ROI tests, in order

These run first on every new target. All four categories below have paid out on live programs.

| # | Command | What it finds |
|---|---------|---------------|
| 1 | `subfinder -d TARGET -silent \| httpx -silent -sc -title -td -server` | Live subdomains with status, title, tech stack |
| 2 | `curl -s https://TARGET/config.json \| jq .` | SPA config files — API keys, internal URLs, Sentry DSNs |
| 3 | `curl -s -I -H "Origin: https://evil.com" https://api.TARGET/v1/user` | CORS origin reflection |
| 4 | `curl -s https://api.TARGET/api/v3/portal` | Kong portal UUID leak (unauthenticated) |
| 5 | `curl -s https://TARGET/.git/config` | Exposed .git directory |
| 6 | `curl -s https://TARGET/.env` | Exposed .env file |
| 7 | `waybackurls TARGET \| grep -E "api_key\|token\|secret\|password"` | Historical secrets in archived URLs |
| 8 | `ffuf -u https://TARGET/FUZZ -w wordlist.txt -mc 200,301,302,403` | Hidden endpoints and directories |
| 9 | `curl -s https://TARGET/api/v1/user -H "Authorization: Bearer TOKEN_A" -X GET` | IDOR baseline — replace user ID and replay |
| 10 | `nuclei -u TARGET -t exposures/ -severity medium,high,critical` | Known exposure templates |
| 11 | `gau TARGET \| grep -E "\.json\|\.xml\|\.yaml\|\.config"` | Config file URLs from passive sources |

---

## Features

### 215 Commands across 30 categories
Every command came from a real engagement. Categories include:
- Quickfire (highest-ROI, run first)
- Subdomain enumeration
- IDOR & access control
- CORS testing
- GraphQL auditing
- JWT attacks
- SSRF
- XSS & injection
- Race conditions
- ATO chains
- API & endpoint discovery
- Backup & hidden files
- Sentry & key verification
- Cloud misconfiguration
- And 16 more

### 94 Real-Time Detection Rules
Fires on every output line as it streams. Catches:
- **ATO tokens** — `idToken`, `access_token`, `oauth_token`, `auth_data`, `bearer_token`
- **API secrets** — Stripe `sk_live_`, Twilio `AC[a-f0-9]{32}`, SendGrid `SG.`, GitHub `gh[pousr]_`, OpenAI `sk-`
- **Private IPs** — 10.x, 192.168.x, 172.16-31.x
- **Connection strings** — MongoDB, PostgreSQL, MySQL, Redis
- **AWS** — `AKIA` access keys, ARNs
- **Private keys** — RSA, EC, OPENSSH
- **CORS** — null origin allowed, wildcard, origin reflected
- **Stack traces** — JS, Python, Java, .NET

### Finding Tracker
Click the bug icon on any output card to capture a finding — title, severity, program, endpoint, PoC curl, impact, remediation. Persists to a local JSON file. Nothing leaves your machine.

### One-Click Report Generator
Generates the full HackerOne template from any tracked finding:
- CVSS:3.1 vector + numeric score (auto-calculated from severity)
- OWASP reference (matched from title keywords)
- Impact statement formula (attacker type → action → consequence → regulatory exposure)
- Steps to reproduce with your PoC evidence

Copy to clipboard. Paste into HackerOne or Bugcrowd.

### Discord Webhook
Send any output card or full loot session to your Discord server with one click. Color-coded by finding severity.

---

## Installation

### Free (Windows)
Download `Trapline_1.0.0_x64-setup.exe` from [Releases](https://github.com/foodstampplug/trapline/releases) and run it.

No terminal. No dependencies. Launches like any other desktop app.

### Required tools (optional but recommended)
The app has a built-in tool checker. Install what you need:

```bash
# Core recon
go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest
go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest
go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest
go install github.com/tomnomnom/waybackurls@latest
go install github.com/lc/gau/v2/cmd/gau@latest
go install github.com/ffuf/ffuf/v2@latest

# JS analysis
go install github.com/lc/subjs@latest
go install github.com/tomnomnom/gf@latest
```

`curl` and `nslookup` come pre-installed on Windows 10+.

---

## Tech Stack

- **Frontend** — Vanilla JS, CSS, HTML (no framework)
- **Backend** — Rust (Tauri v2)
- **Detection engine** — `once_cell::sync::Lazy` compiled regex, 94 rules
- **Process runner** — `CREATE_NO_WINDOW` on Windows, full stdout/stderr streaming
- **Storage** — JSON files in `%APPDATA%\Trapline\`

---

## Pricing

| | Free | Pro ($9 one-time) |
|---|---|---|
| All 215 commands | ✓ | ✓ |
| 94 detection rules | ✓ | ✓ |
| Finding tracker | ✓ | ✓ |
| Report generator | ✓ | ✓ |
| Discord webhook | ✓ | ✓ |
| Windows .exe | ✓ | ✓ |
| Windows MSI installer | — | ✓ |
| macOS (.dmg) | — | ✓ coming v1.1 |
| Linux (.AppImage / .deb) | — | ✓ coming v1.1 |
| Future updates | — | ✓ |

No subscription. No account required. Your data never leaves your machine.

---

## Built by

[@foodstampplug](https://x.com/foodstampplug) — bug bounty hunter (LPL Financial, Priceline, Dyson, Inspectorio)

[Discord](https://discord.gg/ZBAfhTUv) · [trapline.xyz](https://trapline.xyz)
