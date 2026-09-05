//! Follow command directories without mistaking paths in prose or output for
//! a change of workspace. Codex's session directory remains the default for
//! each command that omits workdir.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

#[derive(Default)]
pub(super) struct CodexCwd {
    base: Option<PathBuf>,
    pub current: Option<PathBuf>,
    pub from_command: bool,
}

impl CodexCwd {
    pub fn observe(&mut self, entry: &Value) {
        let payload = &entry["payload"];
        match entry["type"].as_str() {
            Some("session_meta" | "turn_context") => {
                if let Some(path) = payload["cwd"].as_str().map(PathBuf::from) {
                    if path.is_absolute() && self.base.as_ref() != Some(&path) {
                        self.base = Some(path.clone());
                        self.current = Some(path);
                        self.from_command = false;
                    }
                }
            }
            Some("response_item") => {
                let name = payload["name"].as_str().unwrap_or_default();
                match (payload["type"].as_str(), name.rsplit('.').next()) {
                    (Some("function_call"), Some("exec_command" | "shell_command")) => {
                        let input = match &payload["arguments"] {
                            Value::String(text) => serde_json::from_str(text).ok(),
                            value => Some(value.clone()),
                        };
                        if let Some(input) = input {
                            self.command(
                                input["workdir"].as_str(),
                                input["cmd"].as_str().or_else(|| input["command"].as_str()),
                            );
                        }
                    }
                    (Some("custom_tool_call"), Some("exec")) => {
                        if let Some(code) = payload["input"].as_str() {
                            for (workdir, command) in exec_directories(code) {
                                self.command(workdir.as_deref(), command.as_deref());
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn command(&mut self, workdir: Option<&str>, command: Option<&str>) {
        let Some(base) = self.base.as_deref() else {
            return;
        };
        let Some(mut directory) =
            workdir.map_or_else(|| Some(base.to_path_buf()), |path| resolve_path(base, path))
        else {
            return;
        };
        if let Some(command) = command {
            directory = leading_cd(&directory, command);
        }
        self.current = Some(directory);
        self.from_command = true;
    }
}

fn resolve_path(base: &Path, path: &str) -> Option<PathBuf> {
    if path.is_empty() || path.contains(['$', '`']) || path == "-" {
        return None;
    }
    if path == "~" {
        return dirs::home_dir();
    }
    if let Some(path) = path.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(path));
    }
    Some(base.join(path))
}

/// Follow a literal leading `cd`, including a chain of relative directory
/// changes. Paths in arguments, scripts, or printed text are not cwd evidence.
fn leading_cd(base: &Path, mut command: &str) -> PathBuf {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"^\s*cd\s+(?:--\s+)?(?:"([^"$`]+)"|'([^']+)'|([^\s;&|]+))\s*(?:&&|;|\n|$)"#)
            .expect("valid cd regex")
    });
    let mut directory = base.to_path_buf();
    while let Some(caps) = re.captures(command) {
        let path = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .unwrap()
            .as_str();
        let Some(next) = resolve_path(&directory, path).filter(|path| path.is_dir()) else {
            break;
        };
        directory = next;
        command = &command[caps.get(0).unwrap().end()..];
    }
    directory
}

#[derive(Debug, PartialEq)]
enum Token {
    Word(String),
    String(String),
    Symbol(char),
}

/// Read literal tool arguments from functions.exec without evaluating code.
/// Tokenizing skips quoted shell bodies and comments, so a workdir mentioned
/// inside Python, a command string, or a comment cannot move the widgets.
fn exec_directories(code: &str) -> Vec<(Option<String>, Option<String>)> {
    let tokens = tokens(code);
    let mut commands = Vec::new();
    for (index, window) in tokens.windows(5).enumerate() {
        if !matches!(window, [Token::Word(object), Token::Symbol('.'), Token::Word(method), Token::Symbol('('), Token::Symbol('{')] if object == "tools" && method == "exec_command")
        {
            continue;
        }
        let mut depth = 1;
        let mut workdir = None;
        let mut command = None;
        let mut unknown_workdir = false;
        let mut spread = false;
        for i in index + 5..tokens.len() {
            match &tokens[i] {
                Token::Symbol('{' | '[' | '(') => depth += 1,
                Token::Symbol('}' | ']' | ')') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Token::Word(key) | Token::String(key)
                    if depth == 1 && matches!(key.as_str(), "workdir" | "cmd") =>
                {
                    if tokens.get(i + 1) != Some(&Token::Symbol(':')) {
                        unknown_workdir |= key == "workdir";
                        continue;
                    }
                    let value = match (tokens.get(i + 2), tokens.get(i + 3)) {
                        (Some(Token::String(value)), Some(Token::Symbol(',' | '}'))) => {
                            Some(value.clone())
                        }
                        _ => None,
                    };
                    if key == "workdir" {
                        unknown_workdir = value.is_none();
                        workdir = value;
                    } else {
                        command = value;
                    }
                }
                Token::Symbol('.') if depth == 1 => spread = true,
                _ => {}
            }
        }
        if !unknown_workdir && !spread {
            commands.push((workdir, command));
        }
    }
    commands
}

fn tokens(code: &str) -> Vec<Token> {
    let mut chars = code.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(ch) = chars.next() {
                    if ch == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            '"' | '\'' | '`' => {
                let mut value = String::new();
                let mut closed = false;
                while let Some(next) = chars.next() {
                    if next == ch {
                        closed = true;
                        break;
                    }
                    if next == '\\' {
                        if let Some(escaped) = chars.next() {
                            value.push(match escaped {
                                'n' => '\n',
                                'r' => '\r',
                                't' => '\t',
                                other => other,
                            });
                        }
                    } else {
                        value.push(next);
                    }
                }
                if closed && !(ch == '`' && value.contains("${")) {
                    tokens.push(Token::String(value));
                } else {
                    tokens.push(Token::Symbol('?'));
                }
            }
            ch if ch.is_alphanumeric() || matches!(ch, '_' | '$') => {
                let mut word = String::from(ch);
                while chars
                    .peek()
                    .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
                {
                    word.push(chars.next().unwrap());
                }
                tokens.push(Token::Word(word));
            }
            ch if ch.is_whitespace() => {}
            ch => tokens.push(Token::Symbol(ch)),
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_workdir_survives_repeated_turn_context_and_resets_for_default_commands() {
        let mut cwd = CodexCwd::default();
        let context = json!({"type":"turn_context","payload":{"cwd":"/repo"}});
        cwd.observe(&context);
        cwd.observe(&json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":{"cmd":"cargo test","workdir":"/other/worktree"}}}));
        cwd.observe(&context);
        assert_eq!(cwd.current, Some(PathBuf::from("/other/worktree")));
        cwd.observe(&json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":{"cmd":"pwd"}}}));
        assert_eq!(cwd.current, Some(PathBuf::from("/repo")));
    }

    #[test]
    fn exec_reads_literal_workdirs_in_order_and_ignores_embedded_code() {
        let code = r#"// tools.exec_command({workdir:"/comment"})
            text(await tools.exec_command({cmd:'echo "workdir: /quoted"',workdir:"/first path"}));
            text(await tools.exec_command({workdir:'/second',cmd:"cargo test"}));
            const example = 'tools.exec_command({workdir:"/example"})';
            text(await tools.exec_command({cmd:"pwd",workdir:dynamicPath}));"#;
        assert_eq!(
            exec_directories(code),
            vec![
                (
                    Some("/first path".into()),
                    Some("echo \"workdir: /quoted\"".into())
                ),
                (Some("/second".into()), Some("cargo test".into()))
            ]
        );
    }

    #[test]
    fn unknown_workdirs_do_not_reset_the_display_to_the_launch_directory() {
        for code in [
            "tools.exec_command({cmd:'pwd', workdir})",
            "tools.exec_command({...options, cmd:'pwd'})",
            "tools.exec_command({workdir:'/prefix' + suffix, cmd:'pwd'})",
        ] {
            assert!(exec_directories(code).is_empty(), "{code}");
        }
    }

    #[test]
    fn follows_quoted_cd_but_not_a_path_in_prose_or_output() {
        let directory = tempfile::tempdir().unwrap();
        let worktree = directory.path().join("my worktree");
        std::fs::create_dir(&worktree).unwrap();
        let mut cwd = CodexCwd::default();
        cwd.observe(&json!({"type":"session_meta","payload":{"cwd":directory.path()}}));
        cwd.observe(&json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","input":"text(await tools.exec_command({cmd:\"cd 'my worktree' && pwd\"}));"}}));
        assert_eq!(cwd.current, Some(worktree.clone()));
        cwd.observe(&json!({"type":"response_item","payload":{"type":"function_call_output","output":"workdir: /other"}}));
        cwd.observe(&json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"text":"cd /other"}]}}));
        assert_eq!(cwd.current, Some(worktree));
    }
}
