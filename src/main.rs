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
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Seconds between samples.
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
    /// Run the live mactop-style terminal UI.
    #[command(alias = "live")]
    Tui,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _telemetry = init_foundations(cli.verbose)?;
    log::debug!("starting macvmtop"; "command" => ?cli.command);

    let interval = Duration::from_secs_f64(cli.interval.max(0.1));
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
                .min(Duration::from_millis(50));

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
