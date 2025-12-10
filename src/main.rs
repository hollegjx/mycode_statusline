use mycode::cli::Cli;
use mycode::config::{Config, InputData};
use mycode::core::{collect_all_segments, StatusLineGenerator};
use mycode::wrapper::{find_claude_code, injector::ClaudeCodeInjector};
use std::io::{self, IsTerminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Migrate legacy config directory if needed
    migrate_legacy_config()?;

    let cli = Cli::parse_args();

    // Handle wrapper mode - inject into Claude Code
    if cli.wrap {
        return run_wrapper_mode(&cli);
    }

    // Handle configuration commands
    if cli.init {
        Config::init()?;

        // 自动配置 Claude Code settings.json
        println!("\n正在配置 Claude Code settings.json...");
        match mycode::auto_config::ClaudeSettingsConfigurator::configure_statusline() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("⚠ 配置 Claude settings.json 失败: {}", e);
                eprintln!("  你可以手动配置 statusLine 字段");
            }
        }

        return Ok(());
    }

    if cli.print {
        let mut config = Config::load().unwrap_or_else(|_| Config::default());

        // Apply theme override if provided
        if let Some(theme) = cli.theme {
            config = mycode::ui::themes::ThemePresets::get_theme(&theme);
        }

        config.print()?;
        return Ok(());
    }

    if cli.check {
        let config = Config::load()?;
        config.check()?;
        println!("✓ Configuration valid");
        return Ok(());
    }

    if cli.config {
        #[cfg(feature = "tui")]
        {
            mycode::ui::run_configurator()?;
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("TUI feature is not enabled. Please install with --features tui");
            std::process::exit(1);
        }
        return Ok(());
    }

    if cli.update {
        #[cfg(feature = "self-update")]
        {
            println!("Update feature not implemented in new architecture yet");
        }
        #[cfg(not(feature = "self-update"))]
        {
            println!("Update check not available (self-update feature disabled)");
        }
        return Ok(());
    }

    // Handle Claude Code patcher
    if let Some(claude_path) = cli.patch {
        use mycode::utils::ClaudeCodePatcher;

        println!("🔧 Claude Code Context Warning Disabler");
        println!("Target file: {}", claude_path);

        // Create backup in same directory
        let backup_path = format!("{}.backup", claude_path);
        std::fs::copy(&claude_path, &backup_path)?;
        println!("📦 Created backup: {}", backup_path);

        // Load and patch
        let mut patcher = ClaudeCodePatcher::new(&claude_path)?;

        // Apply all modifications
        println!("\n🔄 Applying patches...");

        // 1. Set verbose property to true
        if let Err(e) = patcher.write_verbose_property(true) {
            println!("⚠️ Could not modify verbose property: {}", e);
        }

        // 2. Disable context low warnings
        patcher.disable_context_low_warnings()?;

        // 3. Disable ESC interrupt display
        if let Err(e) = patcher.disable_esc_interrupt_display() {
            println!("⚠️ Could not disable esc/interrupt display: {}", e);
        }

        // 4. Add statusline auto-refresh (30 seconds interval)
        if let Err(e) = patcher.add_statusline_refresh_interval(30000) {
            println!("⚠️ Could not add statusline auto-refresh: {}", e);
        }

        patcher.save()?;

        println!("✅ All patches applied successfully!");
        println!("💡 To restore warnings, replace your cli.js with the backup file:");
        println!("   cp {} {}", backup_path, claude_path);

        return Ok(());
    }

    // Load configuration
    let mut config = Config::load().unwrap_or_else(|_| Config::default());

    // Apply theme override if provided
    if let Some(theme) = cli.theme {
        config = mycode::ui::themes::ThemePresets::get_theme(&theme);
    }

    // Check if stdin has data
    if io::stdin().is_terminal() {
        // No input data available, show main menu
        #[cfg(feature = "tui")]
        {
            use mycode::ui::{MainMenu, MenuResult};

            if let Some(result) = MainMenu::run()? {
                match result {
                    MenuResult::LaunchConfigurator => {
                        mycode::ui::run_configurator()?;
                    }
                    MenuResult::InitConfig => {
                        mycode::config::Config::init()?;
                        println!("Configuration initialized successfully!");
                    }
                    MenuResult::CheckConfig => {
                        let config = mycode::config::Config::load()?;
                        config.check()?;
                        println!("Configuration is valid!");
                    }
                    MenuResult::Exit => {
                        // Exit gracefully
                    }
                }
            }
        }
        #[cfg(not(feature = "tui"))]
        {
            eprintln!("No input data provided and TUI feature is not enabled.");
            eprintln!("Usage: echo '{{...}}' | uucode");
            eprintln!("   or: uucode --help");
        }
        return Ok(());
    }

    // Read Claude Code data from stdin
    let stdin = io::stdin();
    let input: InputData = serde_json::from_reader(stdin.lock())?;

    // Collect segment data
    let segments_data = collect_all_segments(&config, &input);

    // Render statusline
    let generator = StatusLineGenerator::new(config);
    let statusline = generator.generate(segments_data);

    println!("{}", statusline);

    Ok(())
}

fn run_wrapper_mode(_cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Find Claude Code executable
    let claude_path = find_claude_code()?;
    println!("✓ Found Claude Code at: {}", claude_path.display());

    // Load API keys from config
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let keys_path = home.join(".claude").join("uucode").join("api_keys.toml");

    let (_api_key, _glm_key) = if keys_path.exists() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct ApiKeys {
            #[serde(default)]
            uucode_api_key: Option<String>,
            #[serde(default)]
            glm_api_key: Option<String>,
        }

        let content = std::fs::read_to_string(&keys_path)?;
        let keys: ApiKeys = toml::from_str(&content)?;
        (keys.uucode_api_key, keys.glm_api_key)
    } else {
        (None, None)
    };

    let mut injector = ClaudeCodeInjector::new(claude_path, None)?;

    // Get remaining args to pass to Claude Code
    let claude_args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--wrap")
        .collect();

    println!("\n🚀 启动 Claude Code...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("😊 感谢您使用 uucode！");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    injector.run_with_interception(claude_args)
}

fn migrate_legacy_config() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(home) = dirs::home_dir() {
        let old_dir = home.join(".claude").join("88code");
        let new_dir = home.join(".claude").join("uucode");

        if old_dir.exists() && !new_dir.exists() {
            std::fs::rename(&old_dir, &new_dir)?;
            println!("✓ 已自动迁移旧配置目录到 ~/.claude/uucode");
        }
    }
    Ok(())
}
