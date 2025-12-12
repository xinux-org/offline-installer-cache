use std::{collections::HashMap, fs, process::Command};

use anyhow::{Context, Result};
use log::debug;

use crate::parse::Choice;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UserConfig {
    pub name: String,
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub rootpassword: Option<String>,
    pub autologin: bool,
}

#[derive(Debug)]
pub struct MakeConfig {
    pub id: String,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub keyboard: Option<String>,
    pub user: Option<UserConfig>,
    pub list: HashMap<String, HashMap<String, Choice>>,
    pub bootdisk: Option<String>,
    pub imperative_timezone: bool,
}

impl MakeConfig {
    pub fn with_list(list: HashMap<String, HashMap<String, Choice>>) -> Self {
        MakeConfig {
            id: "nixos".to_string(),
            language: Some("en_US".to_string()),
            timezone: Some("Asia/Tashkent".to_string()),
            keyboard: Some("en_US".to_string()),
            user: Some(UserConfig {
                name: "User".to_string(),
                username: "user".to_string(),
                password: "password".to_string(),
                hostname: "nixos".to_string(),
                rootpassword: Some("password".to_string()),
                autologin: true,
            }),
            list,
            bootdisk: None,
            imperative_timezone: true,
        }
    }
}

pub fn iterwrite(
    base_dir: &str,
    temp_dir: &str,
    makeconfig: &MakeConfig,
    path: &str,
    efi: bool,
    arch: &str,
) -> Result<()> {
    // Iterate through files in configs/
    for file in (fs::read_dir(
        format!("{}/xeonitte/{}/{}", base_dir, makeconfig.id, path).replace("//", "/"),
    )?)
    .flatten()
    {
        // Check if it is a dir
        if file.metadata()?.is_dir() {
            // Iterate through files in the dir
            debug!("Iterating through {}", file.path().to_string_lossy());
            debug!("Path: {}", path);
            debug!(
                "!= {}/xeonitte/{}/modules/{{efiboot,biosboot}}",
                base_dir, makeconfig.id
            );
            let _ = iterwrite(
                base_dir,
                temp_dir,
                makeconfig,
                &format!(
                    "{}/{}",
                    path.trim_end_matches('/'),
                    file.file_name().to_string_lossy()
                ),
                efi,
                arch,
            );
        } else if file.file_name().to_string_lossy().ends_with(".nix") {
            let mut config = fs::read_to_string(file.path())?;
            config = config.replace("@NVIDIAOFFLOAD@", "");

            config = config.replace("@ARCH@", &format!("{}-linux", arch));

            if efi {
                config = config.replace("@BOOTLOADER@", "");
                config = config.replace("@BOOTLOADER_MODULE@", "xinux-modules.nixosModules.efiboot")
            } else {
                config = config.replace(
                    "@BOOTLOADER@",
                    &format!(
                        r#"  boot.loader.grub.device = "{}";"#,
                        makeconfig
                            .bootdisk
                            .as_ref()
                            .context("Failed to get bootloader disk")?
                    ),
                );
                config =
                    config.replace("@BOOTLOADER_MODULE@", "xinux-modules.nixosModules.biosboot")
            }

            config = config.replace(
                "@NETWORK@",
                &format!(
                    r#"  # Define your hostname.
networking.hostName = "{}";"#,
                    makeconfig
                        .user
                        .as_ref()
                        .map(|x| x.hostname.as_ref())
                        .unwrap_or("nixos")
                ),
            );

            if makeconfig.imperative_timezone {
                config = config.replace("@TIMEZONE@", "");
            } else if let Some(tz) = &makeconfig.timezone {
                config = config.replace(
                    "@TIMEZONE@",
                    &format!(
                        r#"  # Set your time zone.
            time.timeZone = "{}";"#,
                        tz
                    ),
                );
            }

            if let Some(locale) = &makeconfig.language {
                config = config.replace(
                    "@LOCALE@",
                    &format!(
                        r#"  # Select internationalisation properties.
modules.xinux.language = "{}";"#,
                        locale
                    ),
                );
            }

            if let Some(keymap) = &makeconfig.keyboard {
                if keymap.contains('+') {
                    let mut split = keymap.split('+');
                    if let (Some(layout), Some(variant)) = (split.next(), split.next()) {
                        config = config.replace(
                            "@KEYBOARD@",
                            &format!(
                                r#"  # Set the keyboard layout.
services.xserver.xkb = {{
layout = "{}";
variant = "{}";
}};
console.useXkbConfig = true;"#,
                                layout, variant
                            ),
                        );
                    }
                } else {
                    config = config.replace(
                        "@KEYBOARD@",
                        &format!(
                            r#"  # Set the keyboard layout.
services.xserver.xkb.layout = "{}";
console.useXkbConfig = true;"#,
                            keymap
                        ),
                    );
                }
            }

            if let Some(user) = &makeconfig.user {
                config = config.replace("@USERNAME@", &user.username);
                config = config.replace("@FULLNAME@", &user.name);
                config = config.replace("@HOSTNAME@", &user.hostname);

                let mut autocfg = String::new();
                if user.autologin {
                    autocfg.push_str(&format!(
                        r#"  # Enable automatic login for the user.
services.displayManager.autoLogin.enable = true;
services.displayManager.autoLogin.user = "{}";
"#,
                        user.username
                    ));
                    autocfg.push_str(
                              r#"  # Workaround for GNOME autologin: https://github.com/NixOS/nixpkgs/issues/103746#issuecomment-945091229
systemd.services."getty@tty1".enable = false;
systemd.services."autovt@tty1".enable = false;
"#,
                          );
                }
                config = config.replace("@AUTOLOGIN@", &autocfg);
            }

            // List configuration options
            let mut extrapkgs = vec![];
            for (id, choices) in makeconfig.list.iter() {
                let mut listcfg = String::new();
                for (_key, choice) in choices.iter() {
                    if let Some(pkgs) = &choice.packages {
                        for pkg in pkgs {
                            extrapkgs.push(pkg.to_string());
                        }
                    }
                    if let Some(cfg) = &choice.config {
                        cfg.lines()
                            .for_each(|x| listcfg.push_str(&format!("  {}\n", x)));
                    }
                }
                config = config.replace(&format!("@{}@", id), &listcfg);
            }

            config = config.replace(
                "@PACKAGES@",
                &if extrapkgs.is_empty() {
                    r#"  # List packages installed in system profile.
environment.systemPackages = with pkgs; [
libreoffice
];"#
                    .to_string()
                } else {
                    format!(
                        r#"  # List packages installed in system profile.
environment.systemPackages = with pkgs; [
libreoffice
{}
];"#,
                        extrapkgs.join("\n    ")
                    )
                },
            );

            config = config.replace(
                "@STATEVERSION@",
                &format!(
                    r#"  system.stateVersion = "{}"; # Did you read the comment?"#,
                    String::from_utf8_lossy(
                        &Command::new("nixos-version")
                            .output()
                            .context("Failed to get nixos version")?
                            .stdout
                    )
                    .to_string()
                    .get(0..5)
                    .context("Failed to get nixos version")?
                ),
            );

            let mut cmd = Command::new(format!("{}/xeonitte-helper", base_dir))
                .arg("write-file")
                .arg("--path")
                .arg(if path.is_empty() {
                    format!("{temp_dir}/{}", file.file_name().to_string_lossy())
                } else {
                    format!(
                        "{temp_dir}/{}/{}",
                        path.replace("ARCH", &format!("{}-linux", arch)).replace(
                            "HOSTNAME",
                            makeconfig
                                .user
                                .as_ref()
                                .map(|x| x.hostname.as_ref())
                                .unwrap_or("nixos")
                        ),
                        file.file_name().to_string_lossy()
                    )
                })
                .arg("--contents")
                .arg(config)
                .spawn()?;
            cmd.wait()?;
        } else if file.metadata()?.is_file() {
            Command::new("mkdir")
                .arg("-p")
                .arg(if path.is_empty() {
                    "{temp_dir}/".to_string()
                } else {
                    format!(
                        "{temp_dir}/{}/",
                        path.replace("ARCH", &format!("{}-linux", arch)).replace(
                            "HOSTNAME",
                            makeconfig
                                .user
                                .as_ref()
                                .map(|x| x.hostname.as_ref())
                                .unwrap_or("nixos")
                        )
                    )
                })
                .spawn()?
                .wait()?;

            Command::new("cp")
                .arg(file.path().to_string_lossy().to_string())
                .arg(if path.is_empty() {
                    format!("{temp_dir}/{}", file.file_name().to_string_lossy())
                } else {
                    format!(
                        "{temp_dir}/{}/{}",
                        path.replace("ARCH", &format!("{}-linux", arch)).replace(
                            "HOSTNAME",
                            makeconfig
                                .user
                                .as_ref()
                                .map(|x| x.hostname.as_ref())
                                .unwrap_or("nixos")
                        ),
                        file.file_name().to_string_lossy()
                    )
                })
                .spawn()?
                .wait()?;
        }
    }
    Ok(())
}
