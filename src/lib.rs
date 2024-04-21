use std::path::{self, PathBuf};
use std::{env, fs};
use zed::lsp::{Completion, CompletionKind};
use zed::CodeLabelSpan;
use zed_extension_api::{self as zed, serde_json, Result};

struct AngularExtension {
    did_find_server: bool,
}

const SERVER_PATH: &str = "node_modules/@angular/language-server/index.js";
const PACKAGE_NAME: &str = "@angular/language-server";

impl AngularExtension {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, id: &zed::LanguageServerId) -> Result<String> {
        let server_exists = self.server_exists();
        if self.did_find_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    println!("PACKAGE INSTALLED CORRECTLY");
                    if !self.server_exists() {
                        Err(format!(
                            "installed package {PACKAGE_NAME} did not contain expected path {SERVER_PATH}",
                        ))?;
                    }
                }
                Err(error) => {
                    println!(
                        "ERROR INSTALLING PACKAGE {} VERSION {} ",
                        PACKAGE_NAME, &version
                    );
                    if !self.server_exists() {
                        Err(error)?;
                    }
                }
            }
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }
}

impl zed::Extension for AngularExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        id: &zed::LanguageServerId,
        _: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.server_script_path(id)?;
        let current_dir = env::current_dir().unwrap_or(PathBuf::new());
        let full_path = current_dir.join(&server_path);
        println!("_ANGULAR_ SERVER PATH {:?}", &server_path);
        println!("BUFFER PATH {:?}", full_path);
        println!(
            "CURRENT PATH {}",
            current_dir.join("node_modules").to_string_lossy()
        );

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                full_path.to_string_lossy().to_string(),
                "--stdio".to_string(),
                format!(
                    "--tsProbeLocations {}",
                    current_dir.join("node_modules").to_string_lossy()
                ),
                format!(
                    "--ngProbeLocations {}",
                    current_dir.join("node_modules").to_string_lossy()
                ),
            ],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _: &zed::LanguageServerId,
        _: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(Some(serde_json::json!({
            "typescript": {
                "tsdk": "node_modules/typescript/lib"
            }
        })))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &zed::LanguageServerId,
        completion: Completion,
    ) -> Option<zed::CodeLabel> {
        let highlight_name = match completion.kind? {
            CompletionKind::Class | CompletionKind::Interface => "type",
            CompletionKind::Constructor => "type",
            CompletionKind::Constant => "constant",
            CompletionKind::Function | CompletionKind::Method => "function",
            CompletionKind::Property | CompletionKind::Field => "tag",
            CompletionKind::Variable => "type",
            CompletionKind::Keyword => "keyword",
            CompletionKind::Value => "tag",
            _ => return None,
        };

        let len = completion.label.len();
        let name_span = CodeLabelSpan::literal(completion.label, Some(highlight_name.to_string()));

        Some(zed::CodeLabel {
            code: Default::default(),
            spans: if let Some(detail) = completion.detail {
                vec![
                    name_span,
                    CodeLabelSpan::literal(" ", None),
                    CodeLabelSpan::literal(detail, None),
                ]
            } else {
                vec![name_span]
            },
            filter_range: (0..len).into(),
        })
    }
}

zed::register_extension!(AngularExtension);
