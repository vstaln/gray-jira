<div align="center">
  <img alt="gray-jira" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-jira</h1>
  <p><strong>Jira Cloud issue tools — search, read, create, comment, transition.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-jira">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install jira
```

`gray-jira` exposes Jira Cloud's REST v3 API as agent tools. Config at
`~/.gray/jira/config.json` (`{"base_url","email","api_token"}`) or env
`JIRA_BASE_URL` / `JIRA_EMAIL` / `JIRA_API_TOKEN`.

## Setup

```sh
jira setup --base https://you.atlassian.net --email you@co --token <api-token>
jira status
```

API tokens come from <https://id.atlassian.com/manage-profile/security/api-tokens>.

## Tools

| Tool | What it does |
|---|---|
| `jira_search` | JQL search — `jql`, `limit`. |
| `jira_get` | One issue — `key`. |
| `jira_create` | Create — `project`, `summary`, `description`, `issue_type`. |
| `jira_comment` | Comment — `key`, `body`. |
| `jira_transition` | Move through workflow — `key`, `status`. |

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/jira
```

## License

MIT
