//! Admin commands, available two ways:
//!
//! - the interactive shell: type commands into the running backend's terminal, and
//! - one-shot from the command line: `backend usergroup add 1 admin`, which runs the command
//!   against the database and exits (works while another instance is serving).
//!
//! Changes are logged at `info` level; read-only output is printed to stdout.

use std::{
    fmt::Write as _,
    io::{BufRead, IsTerminal},
};

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{
    models::{Permissions, User, Usergroup},
    notify,
};

const HELP: &str = "\
commands:
  help                                      show this help
  usergroup list                            list all usergroups
  usergroup show <GROUP>                    show a usergroup and its members
  usergroup add <USER> <GROUP>              add a user to a usergroup
  usergroup remove <USER> <GROUP>           remove a user from a usergroup
  user show <USER>                          show a user's groups and effective permissions

  <USER>  GarryNet user id, or 17-digit SteamID64
  <GROUP> usergroup id or name (case-insensitive; quote names with spaces)";

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Invalid(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, CommandError> {
    Err(CommandError::Invalid(msg.into()))
}

/// Reads commands from stdin for as long as the server runs. Does nothing once stdin is
/// closed (e.g. when started by a service manager with no terminal attached).
pub fn spawn(db: PgPool) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // A plain thread rather than `tokio::io::stdin`: tokio's stdin read can't be cancelled
    // and would keep Ctrl+C shutdown waiting until Enter is pressed.
    let reader = std::thread::Builder::new()
        .name("shell-stdin".into())
        .spawn(move || {
            for line in std::io::stdin().lock().lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        log::warn!("interactive shell stopped: failed to read stdin: {e}");
                        break;
                    }
                }
            }
        });
    if let Err(e) = reader {
        log::warn!("interactive shell unavailable: {e}");
        return;
    }

    actix_web::rt::spawn(async move {
        while let Some(line) = rx.recv().await {
            let args = match tokenize(&line) {
                Ok(args) if args.is_empty() => continue,
                Ok(args) => args,
                Err(e) => {
                    eprintln!("error: {e}");
                    continue;
                }
            };
            match execute(&db, &args).await {
                Ok(out) if out.is_empty() => {}
                Ok(out) => println!("{out}"),
                Err(e) => eprintln!("error: {e}"),
            }
        }
    });

    if std::io::stdin().is_terminal() {
        log::info!("interactive shell ready; type `help` for commands");
    }
}

/// Runs one command. Returns text to print (empty for commands whose result is logged).
pub async fn execute(db: &PgPool, args: &[String]) -> Result<String, CommandError> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["help" | "?" | "--help" | "-h"] => Ok(HELP.to_owned()),

        ["usergroup" | "usergroups", "list"] => usergroup_list(db).await,
        ["usergroup", "show", group @ ..] if !group.is_empty() => {
            usergroup_show(db, &group.join(" ")).await
        }
        ["usergroup", action @ ("add" | "remove"), user, group @ ..] if !group.is_empty() => {
            let user = resolve_user(db, user).await?;
            let group = resolve_group(db, &group.join(" ")).await?;
            if *action == "add" {
                usergroup_add(db, &user, &group).await
            } else {
                usergroup_remove(db, &user, &group).await
            }
        }
        ["usergroup", ..] => invalid(
            "usage: usergroup list | show <GROUP> | add <USER> <GROUP> | remove <USER> <GROUP>",
        ),

        ["user", "show", user] => user_show(db, user).await,
        ["user", ..] => invalid("usage: user show <USER>"),

        [cmd, ..] => invalid(format!("unknown command `{cmd}`; type `help` for commands")),
    }
}

async fn usergroup_list(db: &PgPool) -> Result<String, CommandError> {
    let groups = Usergroup::list(db).await?;
    if groups.is_empty() {
        return Ok("no usergroups".into());
    }
    let width = groups.iter().map(|g| g.name.len()).max().unwrap_or(0).max(4);
    let mut out = format!("{:>4}  {:<width$}  {:>7}  PERMISSIONS", "ID", "NAME", "MEMBERS");
    for g in &groups {
        write!(
            out,
            "\n{:>4}  {:<width$}  {:>7}  {}",
            g.id,
            g.name,
            g.members.len(),
            g.permissions
        )
        .unwrap();
    }
    Ok(out)
}

async fn usergroup_show(db: &PgPool, key: &str) -> Result<String, CommandError> {
    let group = resolve_group(db, key).await?;
    let members = User::in_usergroup(db, group.id).await?;

    let mut out = format!(
        "usergroup `{}` (id {})\n  permissions:  {}\n  color:        {}\n  effect:       {}\n  created:      {}\n  last updated: {}\n  members ({}):",
        group.name,
        group.id,
        group.permissions,
        group.color,
        format!("{:?}", group.effect).to_lowercase(),
        group.created_at.format("%Y-%m-%d %H:%M UTC"),
        group
            .last_updated
            .map_or("never".into(), |t| t.format("%Y-%m-%d %H:%M UTC").to_string()),
        members.len(),
    );
    for user in &members {
        write!(out, "\n    {:>4}  {}  {}", user.id, user.steam_id, user.username).unwrap();
    }
    Ok(out)
}

async fn usergroup_add(db: &PgPool, user: &User, group: &Usergroup) -> Result<String, CommandError> {
    if !Usergroup::add_member(db, group.id, user.id).await? {
        return Ok(format!("{} is already in `{}`", describe(user), group.name));
    }
    log::info!(
        "shell: added {} to usergroup `{}` (id {}); effective permissions now {}",
        describe(user),
        group.name,
        group.id,
        Usergroup::permissions_for_user(db, user.id).await?
    );
    notify::usergroup_membership(db, group.id, user.id, true, None).await;
    Ok(String::new())
}

async fn usergroup_remove(
    db: &PgPool,
    user: &User,
    group: &Usergroup,
) -> Result<String, CommandError> {
    if !Usergroup::remove_member(db, group.id, user.id).await? {
        return Ok(format!("{} is not in `{}`", describe(user), group.name));
    }
    let permissions = Usergroup::permissions_for_user(db, user.id).await?;
    log::info!(
        "shell: removed {} from usergroup `{}` (id {}); effective permissions now {permissions}",
        describe(user),
        group.name,
        group.id,
    );
    if !permissions.contains(Permissions::SITE_ACCESS) {
        log::warn!(
            "{} no longer has Site Access and can only use support pages",
            describe(user)
        );
    }
    notify::usergroup_membership(db, group.id, user.id, false, None).await;
    Ok(String::new())
}

async fn user_show(db: &PgPool, key: &str) -> Result<String, CommandError> {
    let user = resolve_user(db, key).await?;
    let groups = Usergroup::for_user(db, user.id).await?;
    let permissions = Usergroup::permissions_for_user(db, user.id).await?;
    let group_names: Vec<String> = groups
        .iter()
        .map(|g| format!("{} (id {})", g.name, g.id))
        .collect();
    Ok(format!(
        "{}\n  steam id:    {}\n  joined:      {}\n  last seen:   {}\n  usergroups:  {}\n  permissions: {permissions}",
        describe(&user),
        user.steam_id,
        user.joined.format("%Y-%m-%d %H:%M UTC"),
        user.last_seen.format("%Y-%m-%d %H:%M UTC"),
        if group_names.is_empty() { "none".into() } else { group_names.join(", ") },
    ))
}

fn describe(user: &User) -> String {
    format!("user {} ({})", user.id, user.username)
}

fn is_steam_id(s: &str) -> bool {
    s.len() == 17 && s.starts_with("7656119") && s.bytes().all(|b| b.is_ascii_digit())
}

async fn resolve_user(db: &PgPool, key: &str) -> Result<User, CommandError> {
    let user = if is_steam_id(key) {
        User::find_by_steam_id(db, key).await?
    } else if let Ok(id) = key.parse() {
        User::find_by_id(db, id).await?
    } else {
        return invalid(format!("`{key}` is not a user id or SteamID64"));
    };
    match user {
        Some(user) => Ok(user),
        None => invalid(format!("no user `{key}`")),
    }
}

/// Numeric keys are tried as an id first, then as a name.
async fn resolve_group(db: &PgPool, key: &str) -> Result<Usergroup, CommandError> {
    if let Ok(id) = key.parse()
        && let Some(group) = Usergroup::find_by_id(db, id).await?
    {
        return Ok(group);
    }
    match Usergroup::find_by_name(db, key).await? {
        Some(group) => Ok(group),
        None => invalid(format!(
            "no usergroup `{key}`; `usergroup list` shows them all"
        )),
    }
}

/// Splits a line on whitespace. Double or single quotes group words: `"Super Admin"`.
fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quote = None;

    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_token = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_token {
                    args.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            (None, c) => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if in_token {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_words_and_quotes() {
        assert_eq!(
            tokenize("  usergroup add 3   \"Super Admin\" ").unwrap(),
            ["usergroup", "add", "3", "Super Admin"]
        );
        assert_eq!(tokenize("a 'b c' \"\"").unwrap(), ["a", "b c", ""]);
        assert!(tokenize("").unwrap().is_empty());
        assert!(tokenize("usergroup add 3 \"admin").is_err());
    }

    #[test]
    fn steam_ids() {
        assert!(is_steam_id("76561197960287930"));
        assert!(!is_steam_id("42"));
        assert!(!is_steam_id("12345678901234567"));
    }
}
