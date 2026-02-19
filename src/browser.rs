use anyhow::Result;
use std::process::Command;

use crate::cli::BrowserChoice;

pub fn open_url(url: &str, browser: BrowserChoice) -> Result<()> {
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
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("open exited with {:?}", status.code());
    }
    Ok(())
}
