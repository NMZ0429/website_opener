use anyhow::Result;
use std::process::Command;

use crate::cli::BrowserChoice;

pub fn open_url(url: &str, browser: BrowserChoice) -> Result<()> {
    let mut cmd = build_command(url, browser)?;
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("browser exited with {:?}", status.code());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn build_command(url: &str, browser: BrowserChoice) -> Result<Command> {
    let mut cmd = Command::new("open");
    match browser {
        BrowserChoice::Default => {
            cmd.arg(url);
        }
        BrowserChoice::Safari => {
            cmd.args(["-a", "Safari", url]);
        }
        BrowserChoice::Chrome => {
            cmd.args(["-a", "Google Chrome", url]);
        }
        BrowserChoice::Firefox => {
            cmd.args(["-a", "Firefox", url]);
        }
        BrowserChoice::Brave => {
            cmd.args(["-a", "Brave Browser", url]);
        }
    }
    Ok(cmd)
}

#[cfg(target_os = "linux")]
fn build_command(url: &str, browser: BrowserChoice) -> Result<Command> {
    let cmd = match browser {
        BrowserChoice::Default => {
            let mut c = Command::new("xdg-open");
            c.arg(url);
            c
        }
        BrowserChoice::Safari => {
            anyhow::bail!("Safari is not available on Linux");
        }
        BrowserChoice::Chrome => {
            let mut c = Command::new("google-chrome");
            c.arg(url);
            c
        }
        BrowserChoice::Firefox => {
            let mut c = Command::new("firefox");
            c.arg(url);
            c
        }
        BrowserChoice::Brave => {
            let mut c = Command::new("brave-browser");
            c.arg(url);
            c
        }
    };
    Ok(cmd)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn build_command(_url: &str, _browser: BrowserChoice) -> Result<Command> {
    anyhow::bail!("Unsupported operating system");
}
