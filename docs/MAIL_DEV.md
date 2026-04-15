# Local mail dev stack (Stalwart + Isotope Mail)

This repo includes a local Docker Compose mail stack:

- **Stalwart Mail Server** (SMTP/IMAP/JMAP + web admin)
- **Isotope Mail Client** (webmail UI) proxied behind the main dev nginx at `/mail/`

The intent is:

- DeployWerk sends transactional mail through **Stalwart** via SMTP.
- You can read the messages in **Isotope** over IMAP.

## Start the stack

From repo root:

```bash
./scripts/deploywerk-dev.sh run
```

Or on Windows PowerShell:

```powershell
docker compose up -d --build
```

## URLs and ports

- **DeployWerk web UI**: `http://127.0.0.1:5173/`
- **DeployWerk API (direct)**: `http://127.0.0.1:8080/`
- **Stalwart web admin**: `http://127.0.0.1:8082/` (container `stalwart`, port `8080` mapped to host `8082`)
- **Isotope webmail**: `http://127.0.0.1:5173/mail/`

Mail protocol ports (host → container):

- **SMTP inbound**: `2525 → 25`
- **SMTP submission (STARTTLS)**: `5870 → 587`
- **IMAP**: `1430 → 143`, **IMAPS**: `9930 → 993`

## Configure DeployWerk SMTP (recommended)

In `.env` (copy from `.env.example`), set:

```env
DEPLOYWERK_MAIL_ENABLED=true

DEPLOYWERK_SMTP_HOST=stalwart
DEPLOYWERK_SMTP_PORT=587
DEPLOYWERK_SMTP_TLS=starttls
DEPLOYWERK_SMTP_FROM=DeployWerk <noreply@dev.local>

# Create these credentials in Stalwart (next section)
DEPLOYWERK_SMTP_USER=deploywerk
DEPLOYWERK_SMTP_PASSWORD=deploywerk-dev-only-change-me
```

Then restart the API container:

```bash
docker compose restart api
```

## First boot: configure Stalwart for dev.local

1. Open **Stalwart web admin** at `http://127.0.0.1:8082/`.
2. Create a dev **domain**: `dev.local`.
3. Create a **mailbox**: `test@dev.local` with a password you’ll use in Isotope.
4. Create an **SMTP submission user** for DeployWerk (for example `deploywerk`) with password `deploywerk-dev-only-change-me`.
   - Ensure SMTP submission on port **587** with **STARTTLS** is enabled.

If you prefer, you can reuse a mailbox account for SMTP auth; separate credentials just makes intent clearer.

## Using Isotope (webmail)

Open `http://127.0.0.1:5173/mail/` and sign in using the mailbox you created, e.g.:

- Email/username: `test@dev.local`
- Password: (the password you set in Stalwart)

## Smoke check (end-to-end)

1. Confirm Stalwart is up:
   - `http://127.0.0.1:8082/` loads.
2. Confirm Isotope loads:
   - `http://127.0.0.1:5173/mail/` loads and shows login.
3. Confirm DeployWerk can send via SMTP:
   - In DeployWerk, create a team and then call the Phase 1 endpoint:
     - `POST /api/v1/teams/{team_id}/mail/send` with a payload like:
       - `from`: `noreply@dev.local`
       - `to`: `[\"test@dev.local\"]`
       - `subject`: `Hello`
       - `text`: `It works`
4. Confirm the message appears in Isotope for `test@dev.local`.

## Notes

- Isotope’s client normally expects its backend at `/api/`. Since DeployWerk also uses `/api/`, nginx rewrites Isotope’s `/api/` references to `/mail-api/` when serving `/mail/`.
- For real-world deliverability (MX, SPF, DKIM, DMARC), see `docs/spec/08-mail-platform.md`. The local stack is for development and UI/API iteration, not public email hosting.

