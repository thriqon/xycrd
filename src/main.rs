use futures::StreamExt;
use kube::api::{Api, ListParams};
use kube_derive::CustomResource;
use kube_runtime::{reflector::reflector, reflector::store::Writer, watcher::watcher};
use log::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt, BufWriter},
    time::{sleep, Duration},
};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "xycrd.thriqon.github.io",
    version = "v1alpha1",
    kind = "EndpointMonitor",
    namespaced
)]
struct EndpointMonitorSpec {
    url: String,
    content_check: Option<String>,
    tags: Option<String>,
}

impl EndpointMonitor {
    fn as_string(&self) -> String {
        let namespace = self.metadata.namespace.as_ref().unwrap();
        let name = self.metadata.name.as_ref().unwrap();

        let content_check_tag = if let Some(content_check) = self.spec.content_check.as_ref() {
            format!("cont;{};{}", self.spec.url, content_check)
        } else {
            self.spec.url.clone()
        };

        let additional_tags = if let Some(tags) = self.spec.tags.as_ref() {
            tags
        } else {
            ""
        };

        format!(
            "0.0.0.0 {}__{} # noconn {} {}",
            namespace, name, content_check_tag, additional_tags
        )
    }
}

async fn persist_xymon_config(
    output_file: &Path,
    data: Vec<EndpointMonitor>,
) -> Result<(), std::io::Error> {
    // TODO insert randomness
    let tmpname = output_file.with_file_name(".xycrd.tmp");
    let tmpfile = File::create(tmpname.clone()).await?;

    {
        let mut writer = BufWriter::new(tmpfile);
        for em in data {
            writer
                .write_all(em.as_string().into_bytes().as_slice())
                .await?;
            writer.write_all(b"\n").await?;
        }
        writer.shutdown().await?;
    }

    fs::rename(tmpname, output_file).await?;

    Ok(())
}

fn parse_pid(b: &[u8]) -> Result<i32, std::io::Error> {
    String::from_utf8_lossy(b)
        .trim()
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[test]
fn test_parse_pid() {
    assert_eq!(parse_pid(b"123").unwrap(), 123);
}

#[test]
fn test_parse_pid_empty_string() {
    parse_pid(b"").unwrap_err();
}

#[test]
fn test_parse_pid_text() {
    parse_pid(b"asd").unwrap_err();
}

async fn ping_xymon(pid_file: &Path) -> Result<(), std::io::Error> {
    let xymon_pid = tokio::fs::read(pid_file).await?;
    trace!("read PID is {:?}", xymon_pid);
    let xymon_pid = parse_pid(&xymon_pid)?;
    trace!("parsed PID is {:?}", xymon_pid);

    unsafe {
        let errno = libc::kill(xymon_pid, libc::SIGHUP);
        if errno == 0 {
            Ok(())
        } else {
            debug!("SIGHUP errno={:?}", errno);
            Err(std::io::Error::from_raw_os_error(errno))
        }
    }
}

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(long)]
    print_install_files: bool,

    #[structopt(long)]
    context: Option<String>,

    #[structopt(long, parse(from_os_str))]
    kubeconfig: Option<PathBuf>,

    #[structopt(long, default_value = "10")]
    reload_delay: u64,

    #[structopt(long, default_value = "/var/run/xymon/xymond.pid", parse(from_os_str))]
    xymond_pid: PathBuf,

    #[structopt(
        long,
        conflicts_with("xymond-pid"),
        help = "Skip SIGHUPping xymond on change"
    )]
    skip_sighup: bool,

    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    #[structopt(long, default_value = "/var/lib/xycrd/xycrd.cfg", parse(from_os_str))]
    output_file: PathBuf,
}

fn print_install_yaml() -> Result<(), std::io::Error> {
    let mut stdout = std::io::stdout();
    stdout.write_all(include_bytes!("install.yaml"))
}

async fn client_from_args(c: &Cli) -> kube::Result<kube::Client> {
    let kubeconfigoptions = kube::config::KubeConfigOptions {
        context: c.context.clone(),
        cluster: None,
        user: None,
    };

    let config = match &c.kubeconfig {
        None => kube::config::Config::from_kubeconfig(&kubeconfigoptions).await?,
        Some(p) => {
            kube::config::Config::from_custom_kubeconfig(
                kube::config::Kubeconfig::read_from(p)?,
                &kubeconfigoptions,
            )
            .await?
        }
    };

    kube::Client::try_from(config)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();

    if args.print_install_files {
        print_install_yaml()?;
        return Ok(());
    }

    stderrlog::new().verbosity(args.verbose + 2).init()?;

    let client = client_from_args(&args).await?;
    let ems_api: Api<EndpointMonitor> = kube::Api::all(client);

    let writer: Writer<EndpointMonitor> = Writer::new(());
    let watcher = watcher(ems_api, ListParams::default());
    let reader = writer.as_reader();
    let store = reflector(writer, watcher);
    tokio::pin!(store);

    loop {
        let _ = store.next().await.expect("stream closed");
        debug!("prepare update after {}", args.reload_delay);
        let sleeper = sleep(Duration::from_secs(args.reload_delay));
        tokio::pin!(sleeper);

        loop {
            tokio::select! {
                Some(_) = store.next() => {
                    info!("ignore additional update");
                },

                () = &mut sleeper => {
                    info!("reload delay elapsed, storing snippet");
                    persist_xymon_config(args.output_file.as_path(), reader.state()).await?;
                    if args.skip_sighup {
                        break;
                    }

                    info!("sending SIGHUP");
                    match ping_xymon(args.xymond_pid.as_path()).await {
                        Ok(_) => {},
                        Err(e) => {
                            warn!("ping xymon: {}", e);
                        },
                    };
                    break;
                }
            }
        }
    }
}
