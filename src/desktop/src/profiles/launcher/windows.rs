use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

use super::common::{command_words_with_args, LaunchPlan};
use crate::profiles::terminal::{self, TerminalChoice};

pub(super) fn spawn(plan: LaunchPlan) -> anyhow::Result<()> {
    let choice = terminal::read_preference();
    match choice {
        TerminalChoice::PowerShell => spawn_powershell(plan),
        other => bail!("terminal '{}' is not supported on Windows", other.id()),
    }
}

fn spawn_powershell(plan: LaunchPlan) -> anyhow::Result<()> {
    let script_path = write_powershell_launch_script(&plan)?;
    let params = format!(
        "-ExecutionPolicy Bypass -NoExit -File {}",
        quote_windows_process_arg(&script_path.to_string_lossy())
    );

    // Use ShellExecuteW through the `open` crate instead of Rust `Command`.
    // `Command` inherits all inheritable handles by default on Windows; if a
    // launched CLI keeps the daemon's TCP listener handle alive, VibeAround's
    // next start sees 127.0.0.1:12358 as occupied by a stale PID.
    open::with(params, "powershell.exe").context("open PowerShell")?;
    Ok(())
}

fn write_powershell_launch_script(plan: &LaunchPlan) -> anyhow::Result<PathBuf> {
    let (command, args) = normalize_windows_launch_command(&plan.command, &plan.args);
    let script_path =
        std::env::temp_dir().join(format!("vibearound-launch-{}.ps1", uuid::Uuid::new_v4()));
    let body = build_powershell_script(plan, &command, &args);
    std::fs::write(&script_path, body)
        .with_context(|| format!("write launch script {:?}", script_path))?;
    ::common::auth::set_owner_only(&script_path).ok();
    Ok(script_path)
}

fn build_powershell_script(plan: &LaunchPlan, command: &str, args: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "$Host.UI.RawUI.WindowTitle = {}\n",
        powershell_single_quoted(&format!("VibeAround - {}", plan.window_label))
    ));
    out.push_str(&format!(
        "Write-Host '# VibeAround profile: {}'\n",
        plan.window_label.replace('\'', "''")
    ));
    for (k, v) in &plan.env {
        out.push_str(&format!("$env:{} = '{}'\n", k, v.replace('\'', "''")));
    }
    append_powershell_color_env(&mut out);
    out.push_str(&format!(
        "Set-Location -LiteralPath '{}'\n",
        escape_powershell_single_quoted(&plan.workspace.to_string_lossy())
    ));
    out.push_str(&powershell_command_block(command, args));
    out.push('\n');
    out.push_str("if ($LASTEXITCODE -ne $null -and $LASTEXITCODE -ne 0) {\n");
    out.push_str("  Write-Host \"`nCommand exited with code $LASTEXITCODE\"\n");
    out.push_str("}\n");
    out.push_str("$scriptPath = $MyInvocation.MyCommand.Path\n");
    out.push_str("if ($scriptPath) { Remove-Item -LiteralPath $scriptPath -Force -ErrorAction SilentlyContinue }\n");
    out
}

fn append_powershell_color_env(out: &mut String) {
    out.push_str("Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue\n");
    out.push_str("if (-not $env:TERM -or $env:TERM -eq 'dumb') { $env:TERM = 'xterm-256color' }\n");
    out.push_str("if (-not $env:COLORTERM) { $env:COLORTERM = 'truecolor' }\n");
    out.push_str("if (-not $env:CLICOLOR) { $env:CLICOLOR = '1' }\n");
}

fn powershell_command_block(command: &str, args: &[String]) -> String {
    let argv = command_words_with_args(command, args);
    let Some((program, program_args)) = argv.split_first() else {
        return String::new();
    };

    let mut out = String::new();
    out.push_str(&format!(
        "$vaCommand = {}\n",
        powershell_single_quoted(program)
    ));
    out.push_str("$vaArgs = @(\n");
    for arg in program_args {
        out.push_str("  ");
        out.push_str(&powershell_single_quoted(arg));
        out.push('\n');
    }
    out.push_str(")\n& $vaCommand @vaArgs");
    out
}

fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", escape_powershell_single_quoted(value))
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn normalize_windows_launch_command(command: &str, args: &[String]) -> (String, Vec<String>) {
    let argv = command_words_with_args(command, args);
    let Some((program, program_args)) = argv.split_first() else {
        return (command.to_string(), args.to_vec());
    };

    if !command_stem_eq(program, "codex") {
        return (command.to_string(), args.to_vec());
    }

    let Some(program_path) = find_windows_command(program) else {
        return (command.to_string(), args.to_vec());
    };
    let Some(ext) = program_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
    else {
        return (command.to_string(), args.to_vec());
    };
    if ext != "cmd" && ext != "ps1" {
        return (command.to_string(), args.to_vec());
    }

    let Some(codex_js) = npm_shim_js_entry(&program_path) else {
        return (command.to_string(), args.to_vec());
    };

    let mut rewritten_args = Vec::with_capacity(program_args.len() + 1);
    rewritten_args.push(codex_js.to_string_lossy().into_owned());
    rewritten_args.extend(program_args.iter().cloned());
    ("node".to_string(), rewritten_args)
}

fn command_stem_eq(command: &str, expected: &str) -> bool {
    let file_name = command
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(command)
        .trim_matches('"');
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    stem.eq_ignore_ascii_case(expected)
}

fn find_windows_command(program: &str) -> Option<PathBuf> {
    let program = program.trim_matches('"');
    let path = Path::new(program);
    if path.is_absolute() || program.contains('\\') || program.contains('/') {
        return existing_windows_command_path(path);
    }

    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        if let Some(candidate) = existing_windows_command_path(&dir.join(program)) {
            return Some(candidate);
        }
    }
    None
}

fn existing_windows_command_path(base: &Path) -> Option<PathBuf> {
    if base.extension().is_some() {
        return base.exists().then(|| base.to_path_buf());
    }

    for ext in [".ps1", ".cmd", ".exe", ".com", ".bat"] {
        let candidate = base.with_extension(ext.trim_start_matches('.'));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn npm_shim_js_entry(shim_path: &Path) -> Option<PathBuf> {
    let body = std::fs::read_to_string(shim_path).ok()?;
    let token = extract_npm_shim_js_token(&body)?;
    let base_dir = shim_path.parent()?;
    let candidate = expand_npm_shim_js_token(base_dir, &token);
    candidate.exists().then_some(candidate)
}

fn extract_npm_shim_js_token(body: &str) -> Option<String> {
    for line in body.lines() {
        let mut rest = line;
        while let Some(start) = rest.find('"') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('"') else {
                break;
            };
            let token = &rest[..end];
            if let Some(js_pos) = token.to_ascii_lowercase().find(".js") {
                return Some(token[..js_pos + 3].to_string());
            }
            rest = &rest[end + 1..];
        }
    }
    None
}

fn expand_npm_shim_js_token(base_dir: &Path, token: &str) -> PathBuf {
    let normalized = token.replace('\\', "/");
    for prefix in ["%dp0%/", "%~dp0/", "$basedir/"] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let mut path = base_dir.to_path_buf();
            for segment in rest.split('/') {
                path.push(segment);
            }
            return path;
        }
    }
    PathBuf::from(token)
}

fn quote_windows_process_arg(value: &str) -> String {
    if !value.is_empty() && !value.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        return value.to_string();
    }

    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    let mut pending_backslashes = 0usize;
    for ch in value.chars() {
        match ch {
            '\\' => pending_backslashes += 1,
            '"' => {
                for _ in 0..(pending_backslashes * 2 + 1) {
                    out.push('\\');
                }
                out.push('"');
                pending_backslashes = 0;
            }
            other => {
                for _ in 0..pending_backslashes {
                    out.push('\\');
                }
                pending_backslashes = 0;
                out.push(other);
            }
        }
    }
    for _ in 0..(pending_backslashes * 2) {
        out.push('\\');
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_command_block_keeps_hook_config_as_one_arg() {
        let args = vec![
            "-c".to_string(),
            "hooks.SessionStart=[{ hooks = [{ type = 'command', command = '\"C:\\Program Files\\VibeAround\\vibearound-hook.exe\" --agent codex' }] }]".to_string(),
        ];
        let block = powershell_command_block("claude code --permission-mode acceptEdits", &args);

        assert!(block.contains("$vaCommand = 'claude'"));
        assert!(block.contains("$vaArgs = @("));
        assert!(block.contains("'code'"));
        assert!(block.contains("'--permission-mode'"));
        assert!(block.contains("'acceptEdits'"));
        assert!(block.contains("'-c'"));
        assert!(block.contains("hooks.SessionStart="));
        assert!(block.contains("& $vaCommand @vaArgs"));
    }

    #[test]
    fn extracts_codex_js_from_npm_cmd_shim() {
        let shim = r#"@IF EXIST "%~dp0\node.exe" (
  "%~dp0\node.exe"  "%dp0%\node_modules\@openai\codex\bin\codex.js" %*
) ELSE (
  node  "%dp0%\node_modules\@openai\codex\bin\codex.js" %*
)"#;

        assert_eq!(
            extract_npm_shim_js_token(shim).as_deref(),
            Some("%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js")
        );
    }

    #[test]
    fn extracts_codex_js_from_npm_powershell_shim() {
        let shim = r#"if (Test-Path "$basedir/node.exe") {
  & "$basedir/node.exe" "$basedir/node_modules/@openai/codex/bin/codex.js" $args
} else {
  & "node.exe" "$basedir/node_modules/@openai/codex/bin/codex.js" $args
}"#;

        assert_eq!(
            extract_npm_shim_js_token(shim).as_deref(),
            Some("$basedir/node_modules/@openai/codex/bin/codex.js")
        );
    }

    #[test]
    fn windows_launch_rewrites_codex_shim_to_node_entrypoint() {
        let root = std::env::temp_dir().join(format!(
            "vibearound-codex-shim-test-{}",
            uuid::Uuid::new_v4()
        ));
        let js_path = root
            .join("node_modules")
            .join("@openai")
            .join("codex")
            .join("bin")
            .join("codex.js");
        std::fs::create_dir_all(js_path.parent().unwrap()).unwrap();
        std::fs::write(&js_path, "").unwrap();
        let shim_path = root.join("codex.ps1");
        std::fs::write(
            &shim_path,
            r#"& "node.exe" "$basedir/node_modules/@openai/codex/bin/codex.js" $args"#,
        )
        .unwrap();

        let args = vec![
            "-c".to_string(),
            "hooks.SessionStart=[{ hooks = [{ command = \"hook --agent codex\" }] }]".to_string(),
        ];
        let (command, rewritten_args) =
            normalize_windows_launch_command(&shim_path.to_string_lossy(), &args);

        assert_eq!(command, "node");
        assert_eq!(
            rewritten_args.first().map(String::as_str),
            Some(js_path.to_str().unwrap())
        );
        assert_eq!(&rewritten_args[1..], &args[..]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn quotes_windows_process_arg_for_shell_execute_parameters() {
        assert_eq!(
            quote_windows_process_arg("C:\\Temp\\launch.ps1"),
            "C:\\Temp\\launch.ps1"
        );
        assert_eq!(
            quote_windows_process_arg("C:\\Temp Dir\\launch.ps1"),
            "\"C:\\Temp Dir\\launch.ps1\""
        );
        assert_eq!(
            quote_windows_process_arg("C:\\Temp Dir\\quote\"here.ps1"),
            "\"C:\\Temp Dir\\quote\\\"here.ps1\""
        );
    }
}
