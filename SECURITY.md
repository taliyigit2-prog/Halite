# Security

Please do not post credentials or private audio in public issues. Report a
vulnerability through GitHub's **Security → Report a vulnerability** when enabled,
or ask the maintainer for a private reporting channel without publishing details.

Halite keeps audio processing local. Downloading helpers/models, retrieving lyrics,
and optional MusicBrainz lookup use the network; reference recordings are never
uploaded. Voice generation runs with Hugging Face and Transformers offline mode.

Model/runtime releases are pinned in `studio-manifest.json` and verified with
SHA-256. Python inference dependencies are pinned with package hashes. Only
user-selected files and application-owned voice outputs can be used by new file
commands. Metadata is written to a validated temporary copy, with a recoverable
backup beside the original. Keep backups until changes have been checked.

Do not load model files from untrusted sources. Do not commit `.env`, certificates,
private keys, application databases, voice profiles or model caches. If a secret
is exposed, revoke/rotate it before considering a Git history rewrite.

Release generation uses a read-only build job and a separate, narrowly scoped
publishing job. Pull-request checks have read-only permissions. Dependencies and
GitHub Actions pins are reviewed through Dependabot.
