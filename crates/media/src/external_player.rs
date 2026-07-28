//! External player launching, per `docs/16-media-handling.md`'s rule:
//! "Use direct argument arrays and avoid shell evaluation." Every path
//! here goes through [`std::process::Command`]'s argument-array API —
//! nothing is ever interpolated into a shell string.

use std::path::Path;
use std::process::{Child, Command};

/// A player invocation, built but not yet spawned — useful for printing
/// what would run (e.g. `item open --no-launch`) without executing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Builds the argument array for launching `player_path` on
/// `media_path`, with an optional subtitle file, per the external
/// application contract in `docs/16-media-handling.md` (local path,
/// item identifier is the caller's concern, optional subtitle path).
pub fn build_command(
    player_path: &str,
    media_path: &Path,
    subtitle_path: Option<&Path>,
) -> PlayerCommand {
    let mut args = vec![media_path.display().to_string()];
    if let Some(subtitle) = subtitle_path {
        args.push("--sub-file".to_string());
        args.push(subtitle.display().to_string());
    }
    PlayerCommand {
        program: player_path.to_string(),
        args,
    }
}

/// Spawns `cmd` directly — no shell, no string interpolation.
pub fn launch(cmd: &PlayerCommand) -> std::io::Result<Child> {
    Command::new(&cmd.program).args(&cmd.args).spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_plain_command_without_a_subtitle() {
        let cmd = build_command("mpv", Path::new("/library/movie.mp4"), None);
        assert_eq!(cmd.program, "mpv");
        assert_eq!(cmd.args, vec!["/library/movie.mp4".to_string()]);
    }

    #[test]
    fn builds_a_command_with_a_subtitle_argument() {
        let cmd = build_command(
            "mpv",
            Path::new("/library/movie.mp4"),
            Some(Path::new("/library/movie.srt")),
        );
        assert_eq!(
            cmd.args,
            vec![
                "/library/movie.mp4".to_string(),
                "--sub-file".to_string(),
                "/library/movie.srt".to_string(),
            ]
        );
    }

    #[test]
    fn shell_metacharacters_in_a_path_stay_literal_arguments() {
        // Regression guard: a path containing shell metacharacters must
        // still appear as a single literal argument, never interpreted.
        let path = Path::new("/library/movie; rm -rf ~.mp4");
        let cmd = build_command("mpv", path, None);
        assert_eq!(cmd.args.len(), 1);
        assert_eq!(cmd.args[0], "/library/movie; rm -rf ~.mp4");
    }
}
