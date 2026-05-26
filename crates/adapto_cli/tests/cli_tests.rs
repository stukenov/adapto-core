use std::fs;
use std::path::PathBuf;

// ── Clap parsing tests ─────────────────────────────────────────────────────

use adapto_cli::commands::{Cli, Commands, GenerateCommand};
use clap::Parser;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(args).expect("failed to parse CLI args")
}

#[test]
fn parse_new_command() {
    let cli = parse(&["adapto", "new", "my_app"]);
    match cli.command {
        Commands::New { name } => assert_eq!(name, "my_app"),
        other => panic!("expected New, got {other:?}"),
    }
}

#[test]
fn parse_dev_defaults() {
    let cli = parse(&["adapto", "dev"]);
    match cli.command {
        Commands::Dev { port, host } => {
            assert_eq!(port, 3000);
            assert_eq!(host, "127.0.0.1");
        }
        other => panic!("expected Dev, got {other:?}"),
    }
}

#[test]
fn parse_dev_custom_port() {
    let cli = parse(&["adapto", "dev", "--port", "8080"]);
    match cli.command {
        Commands::Dev { port, .. } => assert_eq!(port, 8080),
        other => panic!("expected Dev, got {other:?}"),
    }
}

#[test]
fn parse_build_default() {
    let cli = parse(&["adapto", "build"]);
    match cli.command {
        Commands::Build { release } => assert!(!release),
        other => panic!("expected Build, got {other:?}"),
    }
}

#[test]
fn parse_build_release() {
    let cli = parse(&["adapto", "build", "--release"]);
    match cli.command {
        Commands::Build { release } => assert!(release),
        other => panic!("expected Build, got {other:?}"),
    }
}

#[test]
fn parse_check() {
    let cli = parse(&["adapto", "check"]);
    assert!(matches!(cli.command, Commands::Check));
}

#[test]
fn parse_generate_resource() {
    let cli = parse(&["adapto", "generate", "resource", "Customer"]);
    match cli.command {
        Commands::Generate {
            resource: GenerateCommand::Resource { name },
        } => assert_eq!(name, "Customer"),
        other => panic!("expected Generate Resource, got {other:?}"),
    }
}

#[test]
fn parse_routes() {
    let cli = parse(&["adapto", "routes"]);
    assert!(matches!(cli.command, Commands::Routes));
}

#[test]
fn parse_doctor() {
    let cli = parse(&["adapto", "doctor"]);
    assert!(matches!(cli.command, Commands::Doctor));
}

// ── Filesystem tests ────────────────────────────────────────────────────────
//
// These tests use absolute paths as the project "name" so they are fully
// independent of the process-wide current working directory and can run
// in parallel without interference.

/// Create a unique, clean temporary directory for a test.
fn tmp_dir(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("adapto_cli_tests")
        .join(test_name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

#[test]
fn cmd_new_creates_project_structure() {
    let base = tmp_dir("new_structure");
    let project = base.join("demo");
    let project_str = project.to_string_lossy().to_string();

    adapto_cli::commands::run(Cli {
        command: Commands::New {
            name: project_str,
        },
    })
    .expect("cmd_new should succeed");

    assert!(project.join("app").is_dir());
    assert!(project.join("app/dashboard").is_dir());
    assert!(project.join("components").is_dir());
    assert!(project.join("resources").is_dir());
    assert!(project.join("public").is_dir());
    assert!(project.join("tests").is_dir());
}

#[test]
fn cmd_new_creates_adapto_toml() {
    let base = tmp_dir("new_toml");
    let project = base.join("myapp");
    let project_str = project.to_string_lossy().to_string();

    adapto_cli::commands::run(Cli {
        command: Commands::New {
            name: project_str,
        },
    })
    .expect("cmd_new should succeed");

    let toml = fs::read_to_string(project.join("adapto.toml")).expect("read adapto.toml");
    assert!(toml.contains("[server]"));
    assert!(toml.contains("[security]"));
    assert!(toml.contains("[live]"));
    assert!(toml.contains("[tenant]"));
}

#[test]
fn cmd_new_creates_root_page() {
    let base = tmp_dir("new_page");
    let project = base.join("proj");
    let project_str = project.to_string_lossy().to_string();

    adapto_cli::commands::run(Cli {
        command: Commands::New {
            name: project_str,
        },
    })
    .expect("cmd_new should succeed");

    let page = fs::read_to_string(project.join("app/page.adapto")).expect("read page.adapto");
    assert!(page.contains("<route>"));
    assert!(page.contains("path: \"/\""));
    assert!(page.contains("<template>"));
}

#[test]
fn cmd_new_creates_layout() {
    let base = tmp_dir("new_layout");
    let project = base.join("proj");
    let project_str = project.to_string_lossy().to_string();

    adapto_cli::commands::run(Cli {
        command: Commands::New {
            name: project_str,
        },
    })
    .expect("cmd_new should succeed");

    let layout =
        fs::read_to_string(project.join("app/layout.adapto")).expect("read layout.adapto");
    assert!(layout.contains("<layout name=\"main\">"));
    assert!(layout.contains("<slot />"));
}

#[test]
#[ignore] // uses set_current_dir which races with parallel tests; run with --ignored
fn cmd_generate_creates_resource_file() {
    let base = tmp_dir("gen_resource");
    std::env::set_current_dir(&base).unwrap();

    adapto_cli::commands::run(Cli {
        command: Commands::Generate {
            resource: GenerateCommand::Resource {
                name: "Invoice".to_string(),
            },
        },
    })
    .expect("cmd_generate should succeed");

    let resource =
        fs::read_to_string(base.join("resources/Invoice.adapto")).expect("read resource file");
    assert!(resource.contains("name=\"Invoice\""));
    assert!(resource.contains("table=\"invoices\""));

    let page =
        fs::read_to_string(base.join("app/invoices/page.adapto")).expect("read generated page");
    assert!(page.contains("path: \"/invoices\""));
    assert!(page.contains("InvoiceRepo"));
}

#[test]
#[ignore] // uses set_current_dir which races with parallel tests; run with --ignored
fn find_adapto_files_recursive() {
    let base = tmp_dir("find_files");
    std::env::set_current_dir(&base).unwrap();

    fs::create_dir_all(base.join("app/users")).unwrap();
    fs::create_dir_all(base.join("components")).unwrap();
    fs::write(base.join("app/page.adapto"), "root").unwrap();
    fs::write(base.join("app/users/page.adapto"), "users").unwrap();
    fs::write(base.join("components/button.adapto"), "btn").unwrap();
    fs::write(base.join("app/notes.txt"), "ignored").unwrap();

    let files = adapto_cli::commands::find_adapto_files().expect("find files");
    assert_eq!(files.len(), 3, "should find exactly 3 .adapto files");

    for f in &files {
        assert!(f.ends_with(".adapto"), "unexpected file: {f}");
    }
}

// ── Additional Clap parsing tests ──────────────────────────────────────────

#[test]
fn parse_dev_custom_host() {
    let cli = parse(&["adapto", "dev", "--host", "0.0.0.0", "--port", "9090"]);
    match cli.command {
        Commands::Dev { port, host } => {
            assert_eq!(port, 9090);
            assert_eq!(host, "0.0.0.0");
        }
        other => panic!("expected Dev, got {other:?}"),
    }
}

#[test]
fn parse_unknown_command_fails() {
    let result = Cli::try_parse_from(&["adapto", "deploy"]);
    assert!(result.is_err());
}

#[test]
fn parse_new_requires_name() {
    let result = Cli::try_parse_from(&["adapto", "new"]);
    assert!(result.is_err());
}

#[test]
fn parse_generate_resource_requires_name() {
    let result = Cli::try_parse_from(&["adapto", "generate", "resource"]);
    assert!(result.is_err());
}

// ── Error display tests ────────────────────────────────────────────────────

use adapto_cli::error::CliError;

#[test]
fn error_display_io() {
    let err = CliError::IoError("disk full".to_string());
    assert_eq!(err.to_string(), "IO error: disk full");
}

#[test]
fn error_display_compile() {
    let err = CliError::CompileError("syntax at line 5".to_string());
    assert_eq!(err.to_string(), "Compile error: syntax at line 5");
}

#[test]
fn error_display_check_failed() {
    let err = CliError::CheckFailed(3);
    assert_eq!(err.to_string(), "Check failed with 3 error(s)");
}

#[test]
fn error_display_config() {
    let err = CliError::ConfigError("missing key".to_string());
    assert_eq!(err.to_string(), "Config error: missing key");
}

#[test]
fn error_display_not_a_project() {
    let err = CliError::NotAProject;
    assert_eq!(
        err.to_string(),
        "Not an Adapto project (adapto.toml not found)"
    );
}

// ── Command variant exhaustiveness ─────────────────────────────────────────

#[test]
fn all_commands_parse() {
    let cases: Vec<(&[&str], &str)> = vec![
        (&["adapto", "new", "x"], "New"),
        (&["adapto", "dev"], "Dev"),
        (&["adapto", "build"], "Build"),
        (&["adapto", "check"], "Check"),
        (&["adapto", "generate", "resource", "Foo"], "Generate"),
        (&["adapto", "routes"], "Routes"),
        (&["adapto", "doctor"], "Doctor"),
    ];

    for (args, label) in cases {
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok(), "Failed to parse {label}: {:?}", cli.err());
    }
}
