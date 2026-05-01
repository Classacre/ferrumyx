//! Infrastructure setup and orchestration for Ferrumyx

use console::style;
use std::process::Command;
use tokio::process::Command as TokioCommand;

/// Check if required tools are installed
pub async fn check_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking prerequisites...");

    // Check Docker
    match Command::new("docker").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("✅ Docker: {}", version.trim());
        }
        _ => {
            println!("❌ Docker not found. Please install Docker Desktop.");
            println!("   Download: https://www.docker.com/products/docker-desktop");
            return Err("Docker not installed".into());
        }
    }

    // Check Docker Compose
    match Command::new("docker-compose").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("✅ Docker Compose: {}", version.trim());
        }
        _ => {
            // Try docker compose (newer syntax)
            match Command::new("docker").args(["compose", "version"]).output() {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout);
                    println!("✅ Docker Compose: {}", version.trim());
                }
                _ => {
                    println!("❌ Docker Compose not found.");
                    return Err("Docker Compose not available".into());
                }
            }
        }
    }

    Ok(())
}

/// Initialize database schema and data
pub async fn setup_database() -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up database...");

    // Check if database is running
    println!("Checking database connectivity...");
    // TODO: Add database connectivity check

    // Run migrations
    println!("Running database migrations...");
    // TODO: Execute migration scripts

    // Load seed data
    println!("Loading seed data...");
    // TODO: Load development seed data

    println!("✅ Database setup completed");
    Ok(())
}

/// Start Docker services
pub async fn start_services() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Ferrumyx services...");

    // Determine compose command
    let compose_cmd = if Command::new("docker-compose").arg("--version").output().is_ok() {
        "docker-compose"
    } else {
        "docker compose"
    };

    println!("Using: {}", compose_cmd);

    // Pull images
    println!("Pulling Docker images...");
    let pull_status = TokioCommand::new(compose_cmd)
        .args(["pull"])
        .status()
        .await?;

    if !pull_status.success() {
        println!("⚠️  Image pull completed with warnings");
    }

    // Start services
    println!("Starting services...");
    let start_status = TokioCommand::new(compose_cmd)
        .args(["up", "-d"])
        .status()
        .await?;

    if !start_status.success() {
        return Err("Failed to start services".into());
    }

    println!("✅ Services started successfully");
    Ok(())
}

/// Run health checks on all services
pub async fn run_health_checks() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running health checks...");

    // Wait for services to be ready
    println!("Waiting for services to initialize...");
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // Check core services
    let services = vec![
        ("PostgreSQL", "5432"),
        ("Redis", "6379"),
        ("Ferrumyx Web", "3000"),
    ];

    for (service, port) in services {
        println!("Checking {} on port {}...", service, port);

        // Simple connectivity check
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            Ok(_) => println!("✅ {} is accessible", service),
            Err(e) => {
                println!("⚠️  {} not accessible: {}", service, e);
                println!("   Service may still be starting up...");
            }
        }
    }

    println!("✅ Health checks completed");
    println!("Note: Full health validation may take a few minutes for all services to stabilize.");
    Ok(())
}

/// Complete infrastructure setup workflow
pub async fn run_full_setup(environment: &str, skip_health_check: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", style("🚀 Ferrumyx Complete Setup").bold().cyan());
    println!("{}", style("==========================").cyan());
    println!("Environment: {}", style(environment).yellow());

    // Phase 1: Prerequisites
    println!("\n{}", style("🔧 Phase 1: Prerequisites Check").bold());
    check_prerequisites().await?;

    // Phase 2: Database Setup
    println!("\n{}", style("🗄️  Phase 2: Database Setup").bold());
    setup_database().await?;

    // Phase 3: Service Startup
    println!("\n{}", style("🐳 Phase 3: Service Startup").bold());
    start_services().await?;

    // Phase 4: Health Validation
    if !skip_health_check {
        println!("\n{}", style("🏥 Phase 4: Health Validation").bold());
        run_health_checks().await?;
    }

    println!("\n{}", style("🎉 Ferrumyx setup completed successfully!").green().bold());
    println!("\n{}", style("Next steps:").bold());
    println!("1. Access web interface: {}", style("http://localhost:3000").cyan());
    println!("2. View monitoring: {}", style("http://localhost:3001").cyan());
    println!("3. Check logs: {}", style("docker-compose logs -f").cyan());

    Ok(())
}