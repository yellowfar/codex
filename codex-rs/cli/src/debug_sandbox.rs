use std::path::PathBuf;

use codex_common::CliConfigOverrides;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_core::config_types::SandboxMode;
use codex_core::exec::spawn_command_under_linux_sandbox;
use codex_core::exec_env::create_env;
use codex_core::seatbelt::spawn_command_under_seatbelt;
use codex_core::spawn::StdioPolicy;

use crate::LandlockCommand;
use crate::SeatbeltCommand;
use crate::exit_status::handle_exit_status;

pub async fn run_command_under_seatbelt(
    command: SeatbeltCommand,
    codex_linux_sandbox_exe: Option<PathBuf>,
) -> anyhow::Result<()> {
    let SeatbeltCommand {
        full_auto,
        config_overrides,
        command,
    } = command;
    run_command_under_sandbox(
        full_auto,
        command,
        config_overrides,
        codex_linux_sandbox_exe,
        SandboxType::Seatbelt,
    )
    .await
}

pub async fn run_command_under_landlock(
    command: LandlockCommand,
    codex_linux_sandbox_exe: Option<PathBuf>,
) -> anyhow::Result<()> {
    let LandlockCommand {
        full_auto,
        config_overrides,
        command,
    } = command;
    run_command_under_sandbox(
        full_auto,
        command,
        config_overrides,
        codex_linux_sandbox_exe,
        SandboxType::Landlock,
    )
    .await
}

enum SandboxType {
    Seatbelt,
    Landlock,
}

async fn run_command_under_sandbox(
    full_auto: bool,
    command: Vec<String>,
    config_overrides: CliConfigOverrides,
    codex_linux_sandbox_exe: Option<PathBuf>,
    sandbox_type: SandboxType,
) -> anyhow::Result<()> {
    let sandbox_mode = create_sandbox_mode(full_auto);
    let cwd = std::env::current_dir()?;
    let config = Config::load_with_cli_overrides(
        config_overrides
            .parse_overrides()
            .map_err(anyhow::Error::msg)?,
        ConfigOverrides {
            sandbox_mode: Some(sandbox_mode),
            codex_linux_sandbox_exe,
            ..Default::default()
        },
    )?;
    let stdio_policy = StdioPolicy::Inherit;
    let env = create_env(&config.shell_environment_policy);

    let mut child = match sandbox_type {
        SandboxType::Seatbelt => {
            spawn_command_under_seatbelt(command, &config.sandbox_policy, cwd, stdio_policy, env)
                .await?
        }
        SandboxType::Landlock => {
            #[expect(clippy::expect_used)]
            let codex_linux_sandbox_exe = config
                .codex_linux_sandbox_exe
                .expect("codex-linux-sandbox executable not found");
            spawn_command_under_linux_sandbox(
                codex_linux_sandbox_exe,
                command,
                &config.sandbox_policy,
                cwd,
                stdio_policy,
                env,
            )
            .await?
        }
    };
    let status = child.wait().await?;

    handle_exit_status(status);
}

pub fn create_sandbox_mode(full_auto: bool) -> SandboxMode {
    if full_auto {
        SandboxMode::WorkspaceWrite
    } else {
        SandboxMode::ReadOnly
    }
}
