use anyhow::{ensure, Context, Error};
use dialoguer::Confirm;
use qrcode::render::unicode;
use qrcode::QrCode;
use structopt::StructOpt;

use std::{
    fs::File,
    io,
    path::PathBuf,
    process::{self, Stdio},
};

fn read_secret(secret: &str) -> Result<(), Error> {
    let status = process::Command::new("vault")
        .args(&["read", "-field=data", &format!("password-store/{}", secret)])
        .status()?;

    ensure!(status.success(), "vault failed");

    Ok(())
}

fn vread(secret: &str, temp_file: &PathBuf) -> Result<(), Error> {
    let mut cmd = process::Command::new("vault")
        .args(&["read", "-field=data", &format!("password-store/{}", secret)])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdout = cmd.stdout.take().context("no stdout")?;
    let mut file = File::create(temp_file)?;

    io::copy(&mut stdout, &mut file)?;

    cmd.wait()?;

    Ok(())
}

fn edit(path: &PathBuf) -> Result<(), Error> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = process::Command::new(editor)
        .args(&[path.to_str().context("err to_str()")?])
        .status()?;

    ensure!(status.success(), "vault failed");

    Ok(())
}

fn vwrite(secret: &str, path: &PathBuf) -> Result<(), Error> {
    let mut process = process::Command::new("vault")
        .args(&["write", &format!("password-store/{}", secret), "data=-"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut file = File::open(path)?;
    let mut writer = process.stdin.as_ref().context("err stdin")?;

    io::copy(&mut file, &mut writer)?;

    process.wait()?;

    Ok(())
}

fn secret_exists(secret: &str) -> Result<bool, Error> {
    let status = process::Command::new("vault")
        .args(&["read", "-field=data", &format!("password-store/{}", secret)])
        .stdout(Stdio::null())
        .status()?;

    Ok(status.success())
}

fn genpass(secret: &str) -> Result<(), Error> {
    let mut cmd = process::Command::new("genpass")
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdout = cmd.stdout.take().context("no stdout")?;

    let mut process = process::Command::new("vault")
        .args(&["write", &format!("password-store/{}", secret), "data=-"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut writer = process.stdin.as_ref().context("err stdin")?;

    io::copy(&mut stdout, &mut writer)?;

    process.wait()?;

    Ok(())
}

fn qr(secret: &str) -> Result<(), Error> {
    let output = process::Command::new("vault")
        .args(&["read", "-field=data", &format!("password-store/{}", secret)])
        .output()?;

    let code = QrCode::new(output.stdout)?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();

    println!("{}", image);

    Ok(())
}

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    /// Command
    command: Option<Command>,

    /// Read secret
    secret: Option<String>,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Edit secret
    #[structopt()]
    Edit {
        /// Secret
        secret: String,
    },

    /// Generate secret
    #[structopt()]
    Generate {
        /// Secret
        secret: String,
    },

    /// Generate qr-code from secret
    #[structopt()]
    Qr {
        /// Secret
        secret: String,
    },
}

fn main() -> Result<(), Error> {
    let opt = Opts::from_args();

    if let Some(secret) = opt.secret {
        read_secret(&secret)?;
        return Ok(());
    }

    match opt.command.unwrap() {
        Command::Edit { secret } => {
            let tempdir = tempfile::tempdir_in("/dev/shm")?;
            let temp_file = tempdir.path().join("secret");

            vread(&secret, &temp_file)?;

            edit(&temp_file)?;

            vwrite(&secret, &temp_file)?;
        }
        Command::Generate { secret } => {
            if secret_exists(&secret)?
                && !Confirm::new()
                    .with_prompt(format!(
                        "A password already exists for {:?}. Overwrite?",
                        secret
                    ))
                    .interact()?
            {
                return Ok(());
            }

            genpass(&secret)?;
        }
        Command::Qr { secret } => {
            qr(&secret)?;
        }
    }

    Ok(())
}
