//! jira — Jira Cloud issue tools over REST v3. Config:
//! ~/.gray/jira/config.json {"base_url","email","api_token"} or env
//! JIRA_BASE_URL / JIRA_EMAIL / JIRA_API_TOKEN.

use std::io::{BufRead, Write};
use serde_json::{json, Value};

const NAME: &str = "jira";

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Jira",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "jira_search",
             "description": "JQL search; returns 'KEY | status | summary' lines.",
             "parameters": {"type": "object", "properties": {
                "jql": {"type": "string"}, "limit": {"type": "integer"}},
             "required": ["jql"]}},
            {"name": "jira_get",
             "description": "Issue detail by key (PROJ-123): status, assignee, description, recent comments.",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}}, "required": ["key"]}},
            {"name": "jira_comment",
             "description": "Comment on an issue.",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}, "body": {"type": "string"}},
             "required": ["key","body"]}},
            {"name": "jira_create",
             "description": "Create an issue in a project.",
             "parameters": {"type": "object", "properties": {
                "project": {"type": "string"}, "summary": {"type": "string"},
                "description": {"type": "string"},
                "issue_type": {"type": "string", "description": "default Task"}},
             "required": ["project","summary"]}},
            {"name": "jira_transition",
             "description": "Move an issue to a status by name (e.g. 'In Progress', 'Done').",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}, "status": {"type": "string"}},
             "required": ["key","status"]}},
        ],
        "commands": ["/jira"], "subcommands": ["jira"],
    })
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("jira").join("config.json")
}
fn read_config() -> Value {
    std::fs::read_to_string(config_path()).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}
fn cfg_val(k: &str, env: &str) -> Option<String> {
    std::env::var(env).ok().filter(|s| !s.is_empty())
        .or_else(|| read_config()[k].as_str().map(str::to_string))
}

fn api(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let base = cfg_val("base_url", "JIRA_BASE_URL")
        .ok_or("no JIRA base_url — `jira setup --base https://you.atlassian.net --email … --token …`")?;
    let email = cfg_val("email", "JIRA_EMAIL").ok_or("no Jira email configured")?;
    let token = cfg_val("api_token", "JIRA_API_TOKEN").ok_or("no Jira api_token configured")?;
    let url = format!("{}/rest/api/3{}", base.trim_end_matches('/'), path);
    let mut req = match method {
        "GET" => ureq::get(&url),
        "PUT" => ureq::put(&url),
        _ => ureq::post(&url),
    }
    .timeout(std::time::Duration::from_secs(30))
    .set("Accept", "application/json");
    req = req.set("Authorization",
        &format!("Basic {}", b64(&format!("{email}:{token}"))));
    let resp = match body {
        Some(b) => req.set("Content-Type", "application/json").send_json(b),
        None => req.call(),
    };
    match resp {
        Ok(r) => r.into_json().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(204, _)) => Ok(json!({})),
        Err(ureq::Error::Status(c, r)) => Err(format!("HTTP {c}: {}",
            r.into_string().unwrap_or_default().chars().take(300).collect::<String>())),
        Err(e) => Err(e.to_string()),
    }
}

fn b64(s: &str) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o2 = String::new();
    let b = s.as_bytes();
    for (i, c) in b.chunks(3).enumerate() {
        let _ = i;
        let n = match c.len() {
            3 => ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32,
            2 => ((c[0] as u32) << 16) | ((c[1] as u32) << 8),
            _ => (c[0] as u32) << 16,
        };
        o2.push(T[((n >> 18) & 63) as usize] as char);
        o2.push(T[((n >> 12) & 63) as usize] as char);
        o2.push(if c.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        o2.push(if c.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    o2
}

/// Plain text → minimal Atlassian Document Format.
fn adf(text: &str) -> Value {
    json!({"type": "doc", "version": 1, "content": [{
        "type": "paragraph",
        "content": [{"type": "text", "text": text}],
    }]})
}

/// ADF-ish → readable text (best effort).
fn unadf(v: &Value) -> String {
    fn walk(n: &Value, out: &mut String) {
        if let Some(t) = n["text"].as_str() { out.push_str(t); }
        if n["type"].as_str() == Some("paragraph") && !out.is_empty() { out.push('\n'); }
        if let Some(c) = n["content"].as_array() { for x in c { walk(x, out); } }
    }
    let mut s = String::new();
    walk(v, &mut s);
    s.trim().to_string()
}

fn search(a: &Value) -> Result<String, String> {
    let jql = a["jql"].as_str().ok_or("jql required")?;
    let n = a["limit"].as_u64().unwrap_or(20).min(100);
    let d = api("GET", &format!("/search/jql?jql={}&maxResults={n}&fields=summary,status,assignee",
        urlenc(jql)), None)?;
    let issues = d["issues"].as_array().cloned().unwrap_or_default();
    if issues.is_empty() { return Ok("no issues".into()); }
    Ok(issues.iter().map(|i| format!("{} | {} | {}",
        i["key"].as_str().unwrap_or("?"),
        i["fields"]["status"]["name"].as_str().unwrap_or("-"),
        i["fields"]["summary"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n"))
}

fn get(a: &Value) -> Result<String, String> {
    let key = a["key"].as_str().ok_or("key required")?;
    let d = api("GET", &format!("/issue/{key}?fields=summary,status,assignee,description,comment"), None)?;
    let f = &d["fields"];
    let mut out = format!("{} | {} | {}\n{}\n",
        key, f["status"]["name"].as_str().unwrap_or(""), f["summary"].as_str().unwrap_or(""),
        unadf(&f["description"]));
    for c in f["comment"]["comments"].as_array().cloned().unwrap_or_default()
        .iter().rev().take(5).collect::<Vec<_>>().iter().rev() {
        out.push_str(&format!("\n— {}: {}",
            c["author"]["displayName"].as_str().unwrap_or("?"), unadf(&c["body"])));
    }
    Ok(out)
}

fn comment(a: &Value) -> Result<String, String> {
    let key = a["key"].as_str().ok_or("key required")?;
    let body = a["body"].as_str().ok_or("body required")?;
    api("POST", &format!("/issue/{key}/comment"),
        Some(&json!({"body": adf(body)})))?;
    Ok(format!("commented on {key}"))
}

fn create(a: &Value) -> Result<String, String> {
    let project = a["project"].as_str().ok_or("project required")?;
    let summary = a["summary"].as_str().ok_or("summary required")?;
    let mut fields = json!({
        "project": {"key": project},
        "summary": summary,
        "issuetype": {"name": a["issue_type"].as_str().unwrap_or("Task")},
    });
    if let Some(d) = a["description"].as_str() { fields["description"] = adf(d); }
    let r = api("POST", "/issue", Some(&json!({"fields": fields})))?;
    Ok(format!("{} {}", r["key"].as_str().unwrap_or("?"),
        cfg_val("base_url", "JIRA_BASE_URL").unwrap_or_default()
            .trim_end_matches('/').to_string()
            + "/browse/" + r["key"].as_str().unwrap_or("")))
}

fn transition(a: &Value) -> Result<String, String> {
    let key = a["key"].as_str().ok_or("key required")?;
    let want = a["status"].as_str().ok_or("status required")?;
    let d = api("GET", &format!("/issue/{key}/transitions"), None)?;
    let t = d["transitions"].as_array().cloned().unwrap_or_default()
        .into_iter().find(|t| {
            t["name"].as_str().map(|n| n.eq_ignore_ascii_case(want)).unwrap_or(false)
                || t["to"]["name"].as_str().map(|n| n.eq_ignore_ascii_case(want)).unwrap_or(false)
        }).ok_or_else(|| format!("no transition to {want:?}"))?;
    api("POST", &format!("/issue/{key}/transitions"),
        Some(&json!({"transition": {"id": t["id"]}}))).ok();
    Ok(format!("{key} → {want}"))
}

fn urlenc(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z'|b'a'..=b'z'|b'0'..=b'9'|b'-'|b'_'|b'.'|b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}

fn tool(name: &str, args: &Value) -> Value {
    let r = match name {
        "jira_search" => search(args),
        "jira_get" => get(args),
        "jira_comment" => comment(args),
        "jira_create" => create(args),
        "jira_transition" => transition(args),
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(c) => json!({"result": {"content": c}}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn cli(argv: &[String]) -> Result<String, String> {
    match argv.first().map(String::as_str) {
        Some("setup") => {
            let mut cfg = read_config();
            let mut it = argv[1..].iter();
            while let Some(f) = it.next() {
                match f.as_str() {
                    "--base" => cfg["base_url"] = json!(it.next().ok_or("--base needs URL")?),
                    "--email" => cfg["email"] = json!(it.next().ok_or("--email needs value")?),
                    "--token" => cfg["api_token"] = json!(it.next().ok_or("--token needs value")?),
                    o => return Err(format!("unknown flag {o}")),
                }
            }
            std::fs::create_dir_all(config_path().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(config_path(), serde_json::to_string_pretty(&cfg).unwrap())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(config_path(),
                    std::fs::Permissions::from_mode(0o600));
            }
            Ok("saved".into())
        }
        Some("status") | None => Ok(format!(
            "Jira\n  base: {}\n  auth: {}",
            cfg_val("base_url","JIRA_BASE_URL").unwrap_or("MISSING".into()),
            if cfg_val("email","JIRA_EMAIL").is_some()
                && cfg_val("api_token","JIRA_API_TOKEN").is_some() { "set" } else { "MISSING" })),
        Some(o) => Err(format!("unknown verb {o}")),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(v["params"]["name"].as_str().unwrap_or(""), &v["params"]["args"]);
                    r["id"] = id; r
                } else if method == "command/run" {
                    let argv: Vec<String> = v["params"]["argv"].as_array()
                        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default();
                    let text = cli(&argv).unwrap_or_else(|e| e);
                    json!({"id": id, "result": {"text": text}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?; out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("jira") => args[1..].to_vec(), _ => args };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => { let _ = sidecar(); }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("jira — issue tools. `jira setup --base URL --email E --token T`");
                    return;
                }
                let _ = sidecar(); return;
            }
            match cli(&args) {
                Ok(t) => println!("{t}"),
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
    }
}
