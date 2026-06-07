mod darwin;
mod model;
mod render;
mod sampler;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use foundations::telemetry::settings::{LogOutput, LogVerbosity, TelemetrySettings};
use foundations::telemetry::{self, TelemetryConfig, log};
use render::{ProcessTableState, Tui};
use sampler::{ProcessFilter, Sampler, machine_info, system_info_report};
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const INSTALL_SCRIPT_URL: &str = "https://macvmtop.hongjunwu.com/install.sh";
const FALLBACK_INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/wu-hongjun/macvmtop/main/docs/install.sh";
const LATEST_RELEASE_URL: &str = "https://github.com/wu-hongjun/macvmtop/releases/latest";
const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
const TUI_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Seconds between samples. Defaults to 1.0.
    #[arg(short, long, default_value_t = 1.0, global = true)]
    interval: f64,

    /// Number of processes to display.
    #[arg(short = 'p', long, default_value_t = 15, global = true)]
    processes: usize,

    /// Restrict sampled processes to this PID. Repeat for multiple PIDs.
    #[arg(long = "pid", value_name = "PID", value_parser = parse_pid, global = true)]
    pids: Vec<i32>,

    /// Emit JSON for `once`.
    #[arg(long, global = true)]
    json: bool,

    /// Enable debug logs on stderr.
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print one sampled snapshot and exit.
    Once,
    /// Print JSON system information and exit.
    Json {
        /// Include sampled metrics in addition to system information.
        #[arg(long)]
        sample: bool,
        /// Number of samples to collect when --sample is enabled.
        #[arg(long, default_value_t = 1, value_parser = parse_positive_usize)]
        count: usize,
        /// Print human-readable JSON.
        #[arg(long, conflicts_with = "compact")]
        pretty: bool,
        /// Print compact single-line JSON.
        #[arg(long)]
        compact: bool,
    },
    /// Probe VM-visible metrics.
    Probe,
    /// Check whether a newer GitHub release is available.
    CheckUpdate,
    /// Install the latest GitHub release using the hosted installer.
    Update {
        /// Override the install directory used by prebuilt release archives.
        #[arg(long, value_name = "DIR")]
        install_dir: Option<PathBuf>,
    },
    /// Run the live mactop-style terminal UI.
    #[command(alias = "live")]
    Tui,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _telemetry = init_foundations(cli.verbose)?;
    log::debug!("starting macvmtop"; "command" => ?cli.command);

    let interval = sample_interval(cli.interval);
    let process_filter = ProcessFilter::from_pids(cli.pids);
    let command = cli.command.unwrap_or(Command::Tui);

    match command {
        Command::Once => run_once(interval, cli.processes, cli.json, &process_filter),
        Command::Json {
            sample,
            count,
            pretty,
            compact,
        } => run_json(
            interval,
            cli.processes,
            sample,
            count,
            JsonFormat::from_flags(pretty, compact),
            &process_filter,
        ),
        Command::Probe => run_probe(interval, cli.processes, &process_filter),
        Command::CheckUpdate => run_check_update(),
        Command::Update { install_dir } => run_update(install_dir),
        Command::Tui => run_tui(interval, cli.processes, &process_filter),
    }
}

fn parse_pid(value: &str) -> std::result::Result<i32, String> {
    let parsed = value
        .parse::<i32>()
        .map_err(|_| format!("`{value}` is not a valid positive PID"))?;

    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("PID must be greater than 0".to_string())
    }
}

fn init_foundations(verbose: bool) -> Result<telemetry::TelemetryDriver> {
    let service_info = foundations::service_info!();
    let mut settings = TelemetrySettings::default();
    settings.logging.output = LogOutput::Stderr;
    settings.logging.verbosity = if verbose {
        LogVerbosity::Debug
    } else {
        LogVerbosity::Warning
    };

    telemetry::init(TelemetryConfig {
        service_info: &service_info,
        settings: &settings,
    })
    .context("initialize Cloudflare Foundations telemetry")
}

fn run_once(
    interval: Duration,
    process_limit: usize,
    json: bool,
    process_filter: &ProcessFilter,
) -> Result<()> {
    let machine = machine_info();
    let sample = sampled_after_interval(interval, process_limit, process_filter)?;

    if json {
        print_json(
            &model::SystemSnapshotReport { machine, sample },
            JsonFormat::Pretty,
        )?;
    } else {
        render::print_once(&machine, &sample, process_limit)?;
    }

    Ok(())
}

fn run_probe(
    interval: Duration,
    process_limit: usize,
    process_filter: &ProcessFilter,
) -> Result<()> {
    let machine = machine_info();
    let sample = sampled_after_interval(
        interval.min(Duration::from_millis(500)),
        process_limit,
        process_filter,
    )?;
    render::print_probe(&machine, &sample);
    Ok(())
}

fn run_json(
    interval: Duration,
    process_limit: usize,
    include_sample: bool,
    count: usize,
    format: JsonFormat,
    process_filter: &ProcessFilter,
) -> Result<()> {
    if !include_sample && count != 1 {
        bail!("--count requires --sample");
    }

    if include_sample {
        let report = model::SystemSamplesReport {
            machine: machine_info(),
            samples: repeated_samples(interval, process_limit, count, process_filter)?,
        };
        print_json(&report, format)?;
    } else {
        print_json(&system_info_report(), format)?;
    }

    Ok(())
}

fn run_tui(interval: Duration, process_limit: usize, process_filter: &ProcessFilter) -> Result<()> {
    let machine = machine_info();
    let mut sampler = Sampler::new();
    sampler.sample(process_limit, process_filter)?;
    thread::sleep(interval);
    let mut sample = sampler.sample(process_limit, process_filter)?;
    let mut process_state = ProcessTableState::default();
    process_state.clamp(render::visible_process_count(
        &sample,
        process_limit,
        &process_state,
    ));

    let mut tui = Tui::enter()?;
    tui.draw(&machine, &sample, process_limit, &process_state)?;

    loop {
        let next_sample_at = Instant::now() + interval;
        while Instant::now() < next_sample_at {
            let wait = next_sample_at
                .saturating_duration_since(Instant::now())
                .min(TUI_EVENT_POLL_INTERVAL);

            if poll_tui_events(
                wait,
                &mut process_state,
                &sample,
                process_limit,
                &mut tui,
                &machine,
            )? {
                return Ok(());
            }
        }

        if !process_state.paused {
            sample = sampler.sample(process_limit, process_filter)?;
            process_state.clamp(render::visible_process_count(
                &sample,
                process_limit,
                &process_state,
            ));
        }
        tui.draw(&machine, &sample, process_limit, &process_state)?;
    }
}

fn run_check_update() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let Some(latest) = fetch_latest_release_tag()? else {
        println!("macvmtop {current}");
        println!("No published GitHub release was found.");
        println!("Install or update from source with: {INSTALL_SCRIPT_URL}");
        return Ok(());
    };

    let latest_version = latest.trim_start_matches('v');
    println!("current: {current}");
    println!("latest:  {latest_version} ({latest})");

    match compare_versions(current, latest_version) {
        Ordering::Less => {
            println!("update available");
            println!("run: macvmtop update");
        }
        Ordering::Equal => println!("macvmtop is up to date"),
        Ordering::Greater => println!("local version is newer than the latest published release"),
    }

    Ok(())
}

fn run_update(install_dir: Option<PathBuf>) -> Result<()> {
    println!("Downloading macvmtop installer from {INSTALL_SCRIPT_URL}");
    let script_path = temporary_installer_path();
    let script_url = match download_installer(INSTALL_SCRIPT_URL, &script_path) {
        Ok(()) => INSTALL_SCRIPT_URL,
        Err(primary_error) => {
            eprintln!("primary installer download failed: {primary_error}");
            eprintln!("falling back to {FALLBACK_INSTALL_SCRIPT_URL}");
            download_installer(FALLBACK_INSTALL_SCRIPT_URL, &script_path)?;
            FALLBACK_INSTALL_SCRIPT_URL
        }
    };

    let mut command = ProcessCommand::new("sh");
    command.arg(&script_path);

    if let Some(install_dir) = install_dir {
        command.env("MACVMTOP_INSTALL_DIR", install_dir);
    }

    println!("Running installer from {script_url}");
    let status = command.status().context("run macvmtop installer")?;
    let _ = fs::remove_file(&script_path);

    if !status.success() {
        bail!("installer exited with {status}");
    }

    Ok(())
}

fn download_installer(url: &str, path: &Path) -> Result<()> {
    let output = ProcessCommand::new("curl")
        .args(["-fsSL", "-o"])
        .arg(path)
        .arg(url)
        .output()
        .with_context(|| format!("download installer from {url}"))?;

    if !output.status.success() {
        bail!(
            "curl exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn temporary_installer_path() -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("macvmtop-install-{}-{now}.sh", std::process::id()))
}

fn fetch_latest_release_tag() -> Result<Option<String>> {
    let output = ProcessCommand::new("curl")
        .args([
            "-sSL",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}\n%{http_code}",
            LATEST_RELEASE_URL,
        ])
        .output()
        .context("run curl to check latest GitHub release")?;

    if !output.status.success() {
        bail!(
            "curl failed while checking latest release: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout =
        String::from_utf8(output.stdout).context("decode GitHub release check response")?;
    let (url, status) = stdout
        .rsplit_once('\n')
        .context("parse GitHub release check response status")?;

    match status {
        "200" => {
            let tag = release_tag_from_url(url)
                .context("GitHub latest release redirect did not include a release tag")?;
            Ok(Some(tag))
        }
        "404" => Ok(None),
        _ => bail!("GitHub latest release request returned HTTP {status}"),
    }
}

fn release_tag_from_url(url: &str) -> Option<String> {
    let tail = url.split("/releases/tag/").nth(1)?;
    let tag = tail.split(['?', '#']).next()?;

    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

fn sampled_after_interval(
    interval: Duration,
    process_limit: usize,
    process_filter: &ProcessFilter,
) -> Result<model::SystemSample> {
    let mut sampler = Sampler::new();
    sampler.sample(process_limit, process_filter)?;
    thread::sleep(interval);
    sampler.sample(process_limit, process_filter)
}

fn repeated_samples(
    interval: Duration,
    process_limit: usize,
    count: usize,
    process_filter: &ProcessFilter,
) -> Result<Vec<model::SystemSample>> {
    let mut sampler = Sampler::new();
    let mut samples = Vec::with_capacity(count);

    sampler.sample(process_limit, process_filter)?;
    for _ in 0..count {
        thread::sleep(interval);
        samples.push(sampler.sample(process_limit, process_filter)?);
    }

    Ok(samples)
}

fn poll_tui_events(
    timeout: Duration,
    process_state: &mut ProcessTableState,
    sample: &model::SystemSample,
    process_limit: usize,
    tui: &mut Tui,
    machine: &model::MachineInfo,
) -> Result<bool> {
    if !event::poll(timeout)? {
        return Ok(false);
    }

    loop {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(true);
            }

            if process_state.search_editing {
                match key.code {
                    KeyCode::Enter => process_state.search_editing = false,
                    KeyCode::Esc => process_state.search_editing = false,
                    KeyCode::Backspace => {
                        process_state.search.pop();
                    }
                    KeyCode::Char(ch)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        process_state.search.push(ch);
                    }
                    _ => {}
                }
            } else {
                let visible = render::visible_process_count(sample, process_limit, process_state);
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Char('/') => process_state.search_editing = true,
                    KeyCode::Esc => process_state.clear_search(),
                    KeyCode::Char(' ') => process_state.paused = !process_state.paused,
                    KeyCode::Down | KeyCode::Char('j') => process_state.move_down(visible),
                    KeyCode::Up | KeyCode::Char('k') => process_state.move_up(),
                    KeyCode::PageDown => process_state.page_down(visible),
                    KeyCode::PageUp => process_state.page_up(),
                    KeyCode::Home => process_state.select_first(),
                    KeyCode::End => process_state.select_last(visible),
                    _ => {}
                }
            }

            process_state.clamp(render::visible_process_count(
                sample,
                process_limit,
                process_state,
            ));
            tui.draw(machine, sample, process_limit, process_state)?;
        }

        if !event::poll(Duration::from_millis(0))? {
            break;
        }
    }

    Ok(false)
}

#[derive(Copy, Clone, Debug)]
enum JsonFormat {
    Pretty,
    Compact,
}

impl JsonFormat {
    fn from_flags(_pretty: bool, compact: bool) -> Self {
        if compact { Self::Compact } else { Self::Pretty }
    }
}

fn print_json<T: Serialize>(value: &T, format: JsonFormat) -> Result<()> {
    match format {
        JsonFormat::Pretty => println!("{}", serde_json::to_string_pretty(value)?),
        JsonFormat::Compact => println!("{}", serde_json::to_string(value)?),
    }
    Ok(())
}

fn sample_interval(seconds: f64) -> Duration {
    if seconds.is_finite() && seconds > 0.0 {
        Duration::from_secs_f64(seconds).max(MIN_SAMPLE_INTERVAL)
    } else {
        MIN_SAMPLE_INTERVAL
    }
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("`{value}` is not a valid positive integer"))?;
    if parsed == 0 {
        Err("value must be greater than 0".to_string())
    } else {
        Ok(parsed)
    }
}

fn compare_versions(current: &str, latest: &str) -> Ordering {
    let current = parse_version_segments(current);
    let latest = parse_version_segments(latest);

    for idx in 0..current.len().max(latest.len()) {
        let current = current.get(idx).copied().unwrap_or_default();
        let latest = latest.get(idx).copied().unwrap_or_default();

        match current.cmp(&latest) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

fn parse_version_segments(version: &str) -> Vec<u64> {
    version
        .trim_start_matches('v')
        .split('.')
        .map(|segment| {
            segment
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or_default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_like_versions() {
        assert_eq!(compare_versions("0.1.0", "0.1.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.1.0", "0.1.1"), Ordering::Less);
        assert_eq!(compare_versions("0.2.0", "0.1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0"), Ordering::Equal);
        assert_eq!(compare_versions("v1.0.0", "1.0.1"), Ordering::Less);
    }

    #[test]
    fn extracts_release_tag_from_github_latest_redirect() {
        assert_eq!(
            release_tag_from_url("https://github.com/wu-hongjun/macvmtop/releases/tag/v0.1.2"),
            Some("v0.1.2".to_string())
        );
        assert_eq!(
            release_tag_from_url(
                "https://github.com/wu-hongjun/macvmtop/releases/tag/v0.1.2?expanded=true"
            ),
            Some("v0.1.2".to_string())
        );
        assert_eq!(
            release_tag_from_url("https://github.com/wu-hongjun/macvmtop/releases"),
            None
        );
    }

    #[test]
    fn clamps_sample_interval_to_minimum() {
        assert_eq!(sample_interval(0.0), MIN_SAMPLE_INTERVAL);
        assert_eq!(sample_interval(-1.0), MIN_SAMPLE_INTERVAL);
        assert_eq!(sample_interval(f64::NAN), MIN_SAMPLE_INTERVAL);
        assert_eq!(sample_interval(0.05), MIN_SAMPLE_INTERVAL);
        assert_eq!(sample_interval(1.0), Duration::from_secs(1));
    }
}
